use anyhow::Result;
use sqlx::SqlitePool;

pub enum ChangeState {
    Created,
    Updated,
    Deleted,
}

impl ToString for ChangeState {
    fn to_string(&self) -> String {
        match self {
            ChangeState::Created => "created".to_string(),
            ChangeState::Updated => "updated".to_string(),
            ChangeState::Deleted => "deleted".to_string(),
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
    pub created_at: String,
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
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn new(locale: String, key: String, origin: String, state: ChangeState) -> Self {
        Changelog {
            locale,
            key,
            origin,
            message: None,
            state,
            created_at: chrono::Utc::now().to_rfc3339(),
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
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_changelog_key_locale
            ON midlang_changelog (key, locale, id);
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
        .bind(&change.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
