use crate::internals::worker::{WorkState, Workable};
use anyhow::{Ok, Result};
use chrono::Utc;
use scc::HashMap;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Issue {
    pub count: usize,
    pub eventtype: String,
    pub event: IssueEvent,
    pub created_at: String,
    pub last_seen: String,
    pub state: IssueState,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum IssueState {
    Open,
    Closed,
    Ignored,
}

impl ToString for IssueState {
    fn to_string(&self) -> String {
        match self {
            IssueState::Open => "open".to_string(),
            IssueState::Closed => "closed".to_string(),
            IssueState::Ignored => "ignored".to_string(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub enum IssueEvent {
    MissingTranslation { locale: String, key: String },
    MissingLocale { locale: String },
}

impl ToString for IssueEvent {
    fn to_string(&self) -> String {
        match self {
            IssueEvent::MissingTranslation { locale, key } => {
                format!(
                    "Missing translation for locale '{}' and key '{}'",
                    locale, key
                )
            }
            IssueEvent::MissingLocale { locale } => {
                format!("Missing locale '{}'", locale)
            }
        }
    }
}

impl IssueEvent {
    pub fn eventtype(&self) -> String {
        match self {
            IssueEvent::MissingTranslation { locale: _, key: _ } => {
                "missing_translation".to_string()
            }
            IssueEvent::MissingLocale { locale: _ } => "missing_locale".to_string(),
        }
    }

    pub fn id(&self) -> Uuid {
        Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            match self {
                IssueEvent::MissingTranslation { locale, key } => {
                    format!("MISS:{}:{}", locale, key)
                }
                IssueEvent::MissingLocale { locale } => {
                    format!("MISS:{}", locale)
                }
            }
            .as_bytes(),
        )
    }
}

pub struct IssueCollector {
    // store
    pub pending: HashMap<IssueEvent, Issue>,
    pub batch: Mutex<Vec<Issue>>,
    pool: Pool<Sqlite>,
}
impl IssueCollector {
    pub async fn new(pool: Pool<Sqlite>) -> Result<Self> {
        let ic = IssueCollector {
            pending: HashMap::new(),
            batch: Mutex::new(Vec::new()),
            pool,
        };
        ic.ensure_table().await?;
        Ok(ic)
    }

    pub async fn ensure_table(&self) -> Result<()> {
        sqlx::query(
            "
        CREATE TABLE IF NOT EXISTS midlang_issues (
            id TEXT PRIMARY KEY NOT NULL,
            event TEXT NOT NULL,
            count INTEGER NOT NULL,
            last_seen TEXT NOT NULL,
            state TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_midlang_issues_state ON midlang_issues (state);
        ",
        )
        .execute(&mut *self.pool.acquire().await?)
        .await?;
        Ok(())
    }

    pub async fn report(&self, event: IssueEvent) -> Result<()> {
        self.pending
            .entry_async(event.clone())
            .await
            .and_modify(|v| {
                v.count += 1;
                v.last_seen = Utc::now().to_rfc3339();
            })
            .or_insert_with(|| Issue {
                count: 1,
                eventtype: event.eventtype(),
                event,
                created_at: Utc::now().to_rfc3339(),
                last_seen: Utc::now().to_rfc3339(),
                state: IssueState::Open,
            });
        Ok(())
    }

    pub async fn commit(&self) -> Result<()> {
        let mut guard = self.batch.lock().await;
        if guard.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;
        // Create the table if it doesn't exist

        for issue in guard.iter() {
            // If Issue Ignored, skip it
            sqlx::query(
                "
            INSERT INTO midlang_issues (id, event, count, last_seen, state, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                count = midlang_issues.count + excluded.count,
                last_seen = excluded.last_seen,
                state = CASE
                    WHEN midlang_issues.state = 'ignored'
                        THEN midlang_issues.state
                    ELSE excluded.state
                END
            ",
            )
            .bind(issue.event.id().to_string())
            .bind(issue.event.to_string())
            .bind(issue.count as i64)
            .bind(&issue.last_seen)
            .bind(issue.state.to_string())
            .bind(&issue.created_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        guard.clear();
        Ok(())
    }
}

#[async_trait::async_trait]
impl Workable for IssueCollector {
    async fn work(&self) -> Result<WorkState> {
        // Batch is None, safe to move pending into batch and clear pending
        let mut guard = self.batch.lock().await;
        self.pending
            .retain_async(|_, value| {
                guard.push(value.clone());
                false
            })
            .await;
        drop(guard);
        self.commit().await?;
        Ok(WorkState::ACTIVE)
    }
}
