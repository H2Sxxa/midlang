use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, SqlitePool};
use std::fmt;

use crate::{
    query::{Page, QueryCursor, SortOrder},
    time::unix_now,
};

/// Discriminator of a recorded mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeState {
    Created,
    Updated,
    Deleted,
}

impl ChangeState {
    /// Parses the lowercase name used both in the API and in the stored column.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "created" => Some(ChangeState::Created),
            "updated" => Some(ChangeState::Updated),
            "deleted" => Some(ChangeState::Deleted),
            _ => None,
        }
    }
}

impl fmt::Display for ChangeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangeState::Created => write!(f, "created"),
            ChangeState::Updated => write!(f, "updated"),
            ChangeState::Deleted => write!(f, "deleted"),
        }
    }
}

/// One recorded store mutation: the key it touched, the value before and after
/// the change, and when it happened.
///
/// Who made the change and whether it went through an approval flow are
/// transport concerns, so they are not part of this record; an outer layer that
/// knows the caller attaches them to the entry id.
#[derive(Debug, Clone)]
pub struct Changelog {
    pub locale: String,
    pub key: String,
    /// Value before the change. `None` when the key did not exist yet.
    pub previous_value: Option<String>,
    /// Value after the change. `None` when the key was deleted.
    pub new_value: Option<String>,
    pub state: ChangeState,
    pub created_at: i64,
}

impl Changelog {
    pub fn created(locale: String, key: String, new_value: String) -> Self {
        Changelog {
            locale,
            key,
            previous_value: None,
            new_value: Some(new_value),
            state: ChangeState::Created,
            created_at: unix_now(),
        }
    }

    pub fn updated(locale: String, key: String, previous_value: String, new_value: String) -> Self {
        Changelog {
            locale,
            key,
            previous_value: Some(previous_value),
            new_value: Some(new_value),
            state: ChangeState::Updated,
            created_at: unix_now(),
        }
    }

    pub fn deleted(locale: String, key: String, previous_value: String) -> Self {
        Changelog {
            locale,
            key,
            previous_value: Some(previous_value),
            new_value: None,
            state: ChangeState::Deleted,
            created_at: unix_now(),
        }
    }
}

/// One persisted changelog row.
#[derive(Debug, Clone, Serialize)]
pub struct ChangeRecord {
    pub id: i64,
    pub locale: String,
    pub key: String,
    pub state: ChangeState,
    pub previous_value: Option<String>,
    pub new_value: Option<String>,
    pub created_at: i64,
}

/// Which entries a listing includes. It travels with the page cursor, so a
/// resumed page cannot drift away from the filter that produced the first one.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangelogFilter {
    pub locale: Option<String>,
    pub state: Option<ChangeState>,
    pub keyword: Option<String>,
}

/// Cursor of a changelog listing. Entries are ordered by id, which is unique and
/// monotonic, so the position needs no tiebreaker: `created_at` only counts
/// whole seconds and would collide.
pub type ChangelogCursor = QueryCursor<ChangelogFilter, i64>;

pub type ChangelogPage = Page<ChangeRecord, ChangelogCursor>;

#[derive(sqlx::FromRow)]
struct ChangeRow {
    id: i64,
    locale: String,
    key: String,
    state: String,
    previous_value: Option<String>,
    new_value: Option<String>,
    created_at: i64,
}

impl TryFrom<ChangeRow> for ChangeRecord {
    type Error = anyhow::Error;

    fn try_from(row: ChangeRow) -> Result<Self> {
        let state = ChangeState::parse(&row.state)
            .with_context(|| format!("changelog entry '{}' has unknown state", row.id))?;
        Ok(ChangeRecord {
            id: row.id,
            locale: row.locale,
            key: row.key,
            state,
            previous_value: row.previous_value,
            new_value: row.new_value,
            created_at: row.created_at,
        })
    }
}

pub struct ChangelogRecorder {
    pub pool: SqlitePool,
}

impl ChangelogRecorder {
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let recorder = ChangelogRecorder { pool };
        recorder.ensure_table().await?;
        Ok(recorder)
    }

    pub async fn ensure_table(&self) -> Result<()> {
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS midlang_changelog (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT NOT NULL,
                locale TEXT NOT NULL,
                state TEXT NOT NULL,
                previous_value TEXT,
                new_value TEXT,
                created_at INTEGER NOT NULL,
                CHECK (state IN ('created', 'updated', 'deleted')),
                CHECK (
                    (state = 'created' AND previous_value IS NULL AND new_value IS NOT NULL)
                    OR (state = 'updated' AND previous_value IS NOT NULL AND new_value IS NOT NULL)
                    OR (state = 'deleted' AND previous_value IS NOT NULL AND new_value IS NULL)
                )
            ) STRICT;

            CREATE INDEX IF NOT EXISTS idx_changelog_key_locale
            ON midlang_changelog (key, locale, created_at);

            CREATE INDEX IF NOT EXISTS idx_changelog_created_at
            ON midlang_changelog (created_at);
        ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn report_change(&self, change: Changelog) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO midlang_changelog (
                key,
                locale,
                state,
                previous_value,
                new_value,
                created_at
            )
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&change.key)
        .bind(&change.locale)
        .bind(change.state.to_string())
        .bind(&change.previous_value)
        .bind(&change.new_value)
        .bind(change.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// One page of entries for `cursor`. `next` is `None` on the last page.
    pub async fn list(&self, cursor: &ChangelogCursor, limit: usize) -> Result<ChangelogPage> {
        if limit == 0 {
            return Ok(ChangelogPage {
                items: Vec::new(),
                next: None,
            });
        }

        let comparison = match cursor.order {
            SortOrder::Asc => ">",
            SortOrder::Desc => "<",
        };

        let mut builder = QueryBuilder::new(
            "SELECT id, key, locale, state, previous_value, new_value, created_at \
             FROM midlang_changelog WHERE 1 = 1",
        );
        if let Some(locale) = &cursor.filter.locale {
            builder.push(" AND locale = ").push_bind(locale.clone());
        }
        if let Some(state) = cursor.filter.state {
            builder.push(" AND state = ").push_bind(state.to_string());
        }
        if let Some(keyword) = &cursor.filter.keyword {
            let pattern = keyword.to_lowercase();
            builder
                .push(" AND (instr(lower(key), ")
                .push_bind(pattern.clone())
                .push(") > 0 OR instr(lower(previous_value), ")
                .push_bind(pattern.clone())
                .push(") > 0 OR instr(lower(new_value), ")
                .push_bind(pattern)
                .push(") > 0)");
        }
        if let Some(position) = cursor.next {
            builder
                .push(" AND id ")
                .push(comparison)
                .push_bind(position);
        }
        builder
            .push(" ORDER BY id ")
            .push(cursor.order.to_string())
            .push(" LIMIT ")
            .push_bind(limit as i64 + 1);

        let mut rows = builder
            .build_query_as::<ChangeRow>()
            .fetch_all(&self.pool)
            .await?;

        let next = if rows.len() > limit {
            rows.truncate(limit);
            let last = rows.last().expect("a truncated page is not empty");
            Some(ChangelogCursor {
                order: cursor.order,
                filter: cursor.filter.clone(),
                next: Some(last.id),
            })
        } else {
            None
        };

        let items = rows
            .into_iter()
            .map(ChangeRecord::try_from)
            .collect::<Result<Vec<_>>>()?;
        Ok(ChangelogPage { items, next })
    }

    /// Loads one entry by its id.
    pub async fn get(&self, id: i64) -> Result<Option<ChangeRecord>> {
        let row = sqlx::query_as::<_, ChangeRow>(
            "SELECT id, key, locale, state, previous_value, new_value, created_at \
             FROM midlang_changelog WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(ChangeRecord::try_from).transpose()
    }
}
