use anyhow::Result;
use chrono::Utc;
use scc::HashMap;
use serde::{Deserialize, Serialize};
use sqlx::AnyPool;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Issue {
    pub count: usize,
    pub event: IssueEvent,
    pub last_seen: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub enum IssueEvent {
    MissingTranslation { locale: String, key: String },
}

impl IssueEvent {
    pub fn format(&self) -> String {
        match self {
            IssueEvent::MissingTranslation { locale, key } => {
                format!(
                    "Missing translation for locale '{}' and key '{}'",
                    locale, key
                )
            }
        }
    }

    pub fn id(&self) -> Uuid {
        Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            match self {
                IssueEvent::MissingTranslation { locale, key } => {
                    format!("MISS:{}:{}", locale, key)
                }
            }
            .as_bytes(),
        )
    }
}

pub struct IssueCollector {
    // store
    pub pending: HashMap<IssueEvent, Issue>,
    pub batch: Option<Vec<Issue>>,
    pool: AnyPool,
}
impl IssueCollector {
    pub fn new(pool: AnyPool) -> Self {
        IssueCollector {
            pending: HashMap::new(),
            batch: None,
            pool,
        }
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
                event: event,
                last_seen: String::new(),
            });
        Ok(())
    }

    pub async fn commit(&mut self) -> Result<()> {
        let Some(batch) = &self.batch else {
            return Ok(());
        };

        let mut tx = self.pool.begin().await?;
        // Create the table if it doesn't exist
        sqlx::query(
            "
        CREATE TABLE IF NOT EXISTS midlang_issues (
            id CHAR(36) PRIMARY KEY NOT NULL,
            event TEXT NOT NULL,
            count INTEGER NOT NULL,
            last_seen TEXT NOT NULL
        )
        ",
        )
        .execute(&mut *tx)
        .await?;

        for issue in batch {
            sqlx::query(
                "
            INSERT INTO midlang_issues (id, event, count, last_seen)
            VALUES (?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                count = midlang_issues.count + excluded.count,
                last_seen = excluded.last_seen
            ",
            )
            .bind(issue.event.id().to_string())
            .bind(issue.event.format())
            .bind(issue.count as i64)
            .bind(&issue.last_seen)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.batch = None;

        Ok(())
    }
}
