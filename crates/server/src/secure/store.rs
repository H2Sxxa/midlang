use anyhow::{Context, Result};
use midlang_core::time::unix_now;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

use super::AuthStore;
use super::builtins::{BUILTIN_GROUPS, BUILTIN_PERMISSIONS};

impl AuthStore {
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await
            .with_context(|| format!("connect auth database {url}"))?;
        Self::from_pool(pool).await
    }

    pub async fn from_pool(pool: SqlitePool) -> Result<Self> {
        let store = Self { pool };
        store.ensure_schema().await?;
        store.seed_permissions().await?;
        store.seed_builtin_groups().await?;
        Ok(store)
    }

    async fn ensure_schema(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS midlang_permissions (
                name TEXT PRIMARY KEY,
                description TEXT NOT NULL DEFAULT ''
            ) STRICT",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS midlang_permission_groups (
                name TEXT PRIMARY KEY,
                description TEXT NOT NULL DEFAULT '',
                built_in INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            ) STRICT",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS midlang_group_permissions (
                group_name TEXT NOT NULL REFERENCES midlang_permission_groups(name) ON DELETE CASCADE,
                permission_name TEXT NOT NULL REFERENCES midlang_permissions(name) ON DELETE CASCADE,
                PRIMARY KEY (group_name, permission_name)
            ) STRICT",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS midlang_tokens (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                token_prefix TEXT NOT NULL UNIQUE,
                secret_hash TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                expires_at INTEGER,
                created_at INTEGER NOT NULL,
                last_used_at INTEGER,
                revoked_at INTEGER
            ) STRICT",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS midlang_token_groups (
                token_id TEXT NOT NULL REFERENCES midlang_tokens(id) ON DELETE CASCADE,
                group_name TEXT NOT NULL REFERENCES midlang_permission_groups(name),
                PRIMARY KEY (token_id, group_name)
            ) STRICT",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Writes the code-defined catalog into the table, refreshing descriptions
    /// so an upgraded binary keeps stored metadata in sync.
    async fn seed_permissions(&self) -> Result<()> {
        for permission in BUILTIN_PERMISSIONS {
            sqlx::query(
                "INSERT INTO midlang_permissions (name, description) VALUES (?, ?)
                 ON CONFLICT(name) DO UPDATE SET description = excluded.description",
            )
            .bind(permission.name)
            .bind(permission.description)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn seed_builtin_groups(&self) -> Result<()> {
        let now = unix_now();
        for (group, permissions) in BUILTIN_GROUPS {
            sqlx::query(
                "INSERT OR IGNORE INTO midlang_permission_groups
                 (name, description, built_in, created_at) VALUES (?, ?, 1, ?)",
            )
            .bind(*group)
            .bind(format!("Built-in {group} permission group"))
            .bind(now)
            .execute(&self.pool)
            .await?;

            for permission in *permissions {
                sqlx::query(
                    "INSERT OR IGNORE INTO midlang_group_permissions
                     (group_name, permission_name) VALUES (?, ?)",
                )
                .bind(*group)
                .bind(*permission)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
}
