use crate::internals::worker::{WorkState, Workable};
use crate::query::{Page, QueryCursor, SortOrder};
use crate::time::unix_now;
use anyhow::{Context, Ok, Result};
use scc::HashMap;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, QueryBuilder, Sqlite};
use std::fmt;
use tokio::sync::Mutex;
use uuid::Uuid;

pub mod translation;

pub use translation::TranslationIssue;

/// One buffered issue: an event's identity and data plus the tracking state
/// shared by every kind of issue.
#[derive(Debug, Clone)]
pub struct Issue {
    pub id: Uuid,
    pub kind: IssueKind,
    pub payload: serde_json::Value,
    pub summary: String,
    pub count: usize,
    pub created_at: i64,
    pub last_seen: i64,
    pub state: IssueState,
}

/// Discriminator shared by every event of the same class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IssueKind(&'static str);

impl IssueKind {
    pub const fn new(name: &'static str) -> Self {
        IssueKind(name)
    }

    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for IssueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// An event that can be tracked as an issue.
///
/// Implementations live in the domain module that owns the event, so the issue
/// store stays agnostic about the event's fields.
pub trait IssueEvent {
    /// The class of issue this event belongs to.
    fn kind(&self) -> IssueKind;

    /// Canonical structured payload. Two events of the same kind with equal
    /// payloads describe the same issue.
    fn payload(&self) -> serde_json::Value;

    /// One-line, human readable summary used by listings and keyword search.
    fn summary(&self) -> String;

    /// Stable identity derived from the kind and the canonical payload.
    fn id(&self) -> Uuid {
        let canonical = serde_json::to_string(&self.payload())
            .expect("an issue payload is always serializable");
        Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("{}:{}", self.kind().as_str(), canonical).as_bytes(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Open,
    Closed,
    Ignored,
}

impl IssueState {
    /// Parses the lowercase name used both in the API and in the stored column.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "open" => Some(IssueState::Open),
            "closed" => Some(IssueState::Closed),
            "ignored" => Some(IssueState::Ignored),
            _ => None,
        }
    }
}

impl fmt::Display for IssueState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueState::Open => write!(f, "open"),
            IssueState::Closed => write!(f, "closed"),
            IssueState::Ignored => write!(f, "ignored"),
        }
    }
}

/// Column an issue listing is ordered by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSort {
    LastSeen,
    CreatedAt,
    Count,
}

impl IssueSort {
    fn column(&self) -> &'static str {
        match self {
            IssueSort::LastSeen => "last_seen",
            IssueSort::CreatedAt => "created_at",
            IssueSort::Count => "count",
        }
    }
}

/// Which issues a listing includes. It travels with the page cursor, so a
/// resumed page cannot drift away from the filter that produced the first one.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IssueFilter {
    pub state: Option<IssueState>,
    pub kind: Option<String>,
    pub keyword: Option<String>,
    pub sort: IssueSort,
}

/// Position of the last row of a page: its sort value plus the id tiebreaker.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IssuePosition {
    pub value: i64,
    pub id: String,
}

pub type IssueCursor = QueryCursor<IssueFilter, IssuePosition>;

/// One persisted issue row.
#[derive(Debug, Clone, Serialize)]
pub struct IssueRecord {
    pub id: String,
    pub kind: String,
    pub payload: serde_json::Value,
    pub summary: String,
    pub count: usize,
    pub created_at: i64,
    pub last_seen: i64,
    pub state: IssueState,
}

pub type IssuePage = Page<IssueRecord, IssueCursor>;

#[derive(sqlx::FromRow)]
struct IssueRow {
    id: String,
    kind: String,
    payload: String,
    summary: String,
    count: i64,
    created_at: i64,
    last_seen: i64,
    state: String,
}

impl TryFrom<IssueRow> for IssueRecord {
    type Error = anyhow::Error;

    fn try_from(row: IssueRow) -> Result<Self> {
        let state = IssueState::parse(&row.state)
            .with_context(|| format!("issue '{}' has unknown state '{}'", row.id, row.state))?;
        let payload = serde_json::from_str(&row.payload)
            .with_context(|| format!("issue '{}' has an invalid payload", row.id))?;
        Ok(IssueRecord {
            id: row.id,
            kind: row.kind,
            payload,
            summary: row.summary,
            count: row.count as usize,
            created_at: row.created_at,
            last_seen: row.last_seen,
            state,
        })
    }
}

fn sort_value(row: &IssueRow, sort: IssueSort) -> i64 {
    match sort {
        IssueSort::LastSeen => row.last_seen,
        IssueSort::CreatedAt => row.created_at,
        IssueSort::Count => row.count,
    }
}

pub struct IssueCollector {
    // store
    pub pending: HashMap<Uuid, Issue>,
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
            kind TEXT NOT NULL,
            payload TEXT NOT NULL,
            summary TEXT NOT NULL,
            count INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            last_seen INTEGER NOT NULL,
            state TEXT NOT NULL
        ) STRICT;
        ",
        )
        .execute(&mut *self.pool.acquire().await?)
        .await?;
        sqlx::query(
            "
        CREATE INDEX IF NOT EXISTS idx_midlang_issues_state ON midlang_issues (state, last_seen);

        CREATE INDEX IF NOT EXISTS idx_midlang_issues_kind ON midlang_issues (kind, last_seen);
        ",
        )
        .execute(&mut *self.pool.acquire().await?)
        .await?;
        Ok(())
    }

    pub async fn report<E: IssueEvent>(&self, event: E) -> Result<()> {
        let id = event.id();
        self.pending
            .entry_async(id)
            .await
            .and_modify(|v| {
                v.count += 1;
                v.last_seen = unix_now();
            })
            .or_insert_with(|| Issue {
                id,
                kind: event.kind(),
                payload: event.payload(),
                summary: event.summary(),
                count: 1,
                created_at: unix_now(),
                last_seen: unix_now(),
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
        for issue in guard.iter() {
            sqlx::query(
                "
            INSERT INTO midlang_issues
                (id, kind, payload, summary, count, last_seen, state, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
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
            .bind(issue.id.to_string())
            .bind(issue.kind.to_string())
            .bind(
                serde_json::to_string(&issue.payload)
                    .expect("an issue payload is always serializable"),
            )
            .bind(&issue.summary)
            .bind(issue.count as i64)
            .bind(issue.last_seen)
            .bind(issue.state.to_string())
            .bind(issue.created_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        guard.clear();
        Ok(())
    }

    /// Moves every buffered report into the write batch and commits it.
    ///
    /// Queries call this first so an issue reported a moment ago is visible
    /// immediately instead of waiting for the next worker tick.
    pub async fn flush(&self) -> Result<()> {
        // Reads call this before querying, so it stays cheap when nothing was
        // reported; `commit` already no-ops on an empty batch.
        if !self.pending.is_empty() {
            let mut guard = self.batch.lock().await;
            self.pending
                .retain_async(|_, value| {
                    guard.push(value.clone());
                    false
                })
                .await;
        }
        self.commit().await
    }

    /// One page of issue rows for `cursor`, ordered by the cursor's sort field
    /// and breaking ties on the id. `next` is `None` on the last page.
    pub async fn list(&self, cursor: &IssueCursor, limit: usize) -> Result<IssuePage> {
        self.flush().await?;
        if limit == 0 {
            return Ok(IssuePage {
                items: Vec::new(),
                next: None,
            });
        }

        let column = cursor.filter.sort.column();
        let comparison = match cursor.order {
            SortOrder::Asc => ">",
            SortOrder::Desc => "<",
        };

        let mut builder = QueryBuilder::new(
            "SELECT id, kind, payload, summary, count, created_at, last_seen, state \
             FROM midlang_issues WHERE 1 = 1",
        );
        if let Some(state) = cursor.filter.state {
            builder.push(" AND state = ").push_bind(state.to_string());
        }
        if let Some(kind) = &cursor.filter.kind {
            builder.push(" AND kind = ").push_bind(kind.clone());
        }
        if let Some(keyword) = &cursor.filter.keyword {
            let pattern = keyword.to_lowercase();
            builder
                .push(" AND (instr(lower(summary), ")
                .push_bind(pattern.clone())
                .push(") > 0 OR instr(lower(payload), ")
                .push_bind(pattern.clone())
                .push(") > 0 OR instr(lower(kind), ")
                .push_bind(pattern)
                .push(") > 0)");
        }
        if let Some(position) = &cursor.next {
            builder
                .push(" AND (")
                .push(column)
                .push(format!(" {comparison} "))
                .push_bind(position.value)
                .push(" OR (")
                .push(column)
                .push(" = ")
                .push_bind(position.value)
                .push(format!(" AND id {comparison} "))
                .push_bind(position.id.clone())
                .push("))");
        }
        builder
            .push(" ORDER BY ")
            .push(column)
            .push(format!(" {}, id {}", cursor.order, cursor.order))
            .push(" LIMIT ")
            .push_bind(limit as i64 + 1);

        let mut rows = builder
            .build_query_as::<IssueRow>()
            .fetch_all(&self.pool)
            .await?;

        let next = if rows.len() > limit {
            rows.truncate(limit);
            let last = rows.last().expect("a truncated page is not empty");
            Some(IssueCursor {
                order: cursor.order,
                filter: cursor.filter.clone(),
                next: Some(IssuePosition {
                    value: sort_value(last, cursor.filter.sort),
                    id: last.id.clone(),
                }),
            })
        } else {
            None
        };

        let items = rows
            .into_iter()
            .map(IssueRecord::try_from)
            .collect::<Result<Vec<_>>>()?;
        Ok(IssuePage { items, next })
    }

    /// Loads one issue by its id.
    pub async fn get(&self, id: &str) -> Result<Option<IssueRecord>> {
        self.flush().await?;
        let row = sqlx::query_as::<_, IssueRow>(
            "SELECT id, kind, payload, summary, count, created_at, last_seen, state \
             FROM midlang_issues WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(IssueRecord::try_from).transpose()
    }

    /// Marks one issue's state. Returns `false` when no such issue exists.
    pub async fn set_state(&self, id: &str, state: IssueState) -> Result<bool> {
        let result = sqlx::query("UPDATE midlang_issues SET state = ? WHERE id = ?")
            .bind(state.to_string())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[async_trait::async_trait]
impl Workable for IssueCollector {
    async fn work(&self) -> Result<WorkState> {
        self.flush().await?;
        Ok(WorkState::ACTIVE)
    }
}
