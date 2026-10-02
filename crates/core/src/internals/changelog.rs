use anyhow::Result;
use sqlx::SqlitePool;
use std::fmt;

use crate::time::unix_now;

pub enum ChangeState {
    Created,
    Updated,
    Deleted,
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

pub struct Changelog {
    pub locale: String,
    pub key: String,
    // Origin for rollback
    pub origin: String,
    pub message: Option<String>,
    pub state: ChangeState,
    pub created_at: i64,
}

impl Changelog {
    pub fn new_with_message(
        locale: String,
        key: String,
        origin: String,
        state: ChangeState,
        message: String,
    ) -> Self {
        Changelog {
            locale,
            key,
            origin,
            message: Some(message),
            state,
            created_at: unix_now(),
        }
    }

    pub fn new(locale: String, key: String, origin: String, state: ChangeState) -> Self {
        Changelog {
            locale,
            key,
            origin,
            message: None,
            state,
            created_at: unix_now(),
        }
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
                origin TEXT NOT NULL,
                state TEXT NOT NULL,
                message TEXT,
                created_at INTEGER NOT NULL
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
                origin,
                state,
                message,
                created_at
            )
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&change.key)
        .bind(&change.locale)
        .bind(&change.origin)
        .bind(change.state.to_string())
        .bind(&change.message)
        .bind(change.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
