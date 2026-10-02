//! Authentication and authorization for the server boundary.
//!
//! This module deliberately lives outside `midlang-core`: tokens, permission
//! groups and HTTP-facing authorization are transport concerns, while core
//! owns translation data and its storage abstractions.

use anyhow::{Context, Result, anyhow, bail};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
use uuid::Uuid;

const TOKEN_PREFIX: &str = "mlk";

const BUILTIN_GROUPS: &[(&str, &[&str])] = &[
    ("reader", &["translation:read", "translation:list"]),
    (
        "writer",
        &[
            "translation:read",
            "translation:list",
            "translation:write",
            "translation:delete",
        ],
    ),
    (
        "token-manager",
        &["token:create", "token:read", "token:revoke", "token:rotate"],
    ),
    ("auditor", &["diagnostic:read", "audit:read"]),
    (
        "admin",
        &[
            "token:create",
            "token:read",
            "token:revoke",
            "token:rotate",
            "permission:read",
            "permission:manage",
            "translation:read",
            "translation:list",
            "translation:write",
            "translation:delete",
            "translation:import",
            "translation:export",
            "diagnostic:read",
            "audit:read",
            "server:admin",
        ],
    ),
];

#[derive(Clone)]
pub struct AuthStore {
    pool: SqlitePool,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct CreatedToken {
    pub id: String,
    pub name: String,
    pub token: String,
    pub token_prefix: String,
    pub groups: Vec<String>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct TokenInfo {
    pub id: String,
    pub name: String,
    pub token_prefix: String,
    pub status: String,
    pub groups: Vec<String>,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AuthContext {
    pub token_id: String,
    pub name: String,
    pub groups: Vec<String>,
    pub permissions: Vec<String>,
    pub expires_at: Option<i64>,
}

impl AuthContext {
    pub fn can(&self, permission: &str) -> bool {
        self.permissions.iter().any(|item| item == permission)
    }
}

#[derive(Debug, serde::Serialize)]
struct SecurityError {
    code: &'static str,
    message: &'static str,
}

fn security_response(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, axum::Json(SecurityError { code, message })).into_response()
}

/// Authenticates every request in a protected router and makes the resulting
/// context available to handlers through request extensions.
pub async fn middleware(
    State(auth): State<AuthStore>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let Some(value) = request.headers().get(AUTHORIZATION) else {
        return security_response(
            StatusCode::UNAUTHORIZED,
            "missing_token",
            "missing bearer token",
        );
    };
    let Ok(value) = value.to_str() else {
        return security_response(
            StatusCode::UNAUTHORIZED,
            "invalid_token",
            "invalid authorization header",
        );
    };

    match auth.authenticate(value).await {
        Ok(Some(context)) => {
            request.extensions_mut().insert(auth.clone());
            request.extensions_mut().insert(context);
            next.run(request).await
        }
        Ok(None) => security_response(
            StatusCode::UNAUTHORIZED,
            "invalid_token",
            "invalid or expired token",
        ),
        Err(error) => {
            tracing::error!(error = ?error, "authentication request failed");
            security_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "authentication_unavailable",
                "authentication unavailable",
            )
        }
    }
}

/// Checks a permission after [`middleware`] has inserted an [`AuthContext`].
/// Resource-specific scope checks can build on this function later.
pub fn require_permission(context: &AuthContext, permission: &str) -> Result<(), Response> {
    if context.can(permission) {
        Ok(())
    } else {
        Err(security_response(
            StatusCode::FORBIDDEN,
            "permission_denied",
            "permission denied",
        ))
    }
}

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
        store.seed_builtin_groups().await?;
        Ok(store)
    }

    async fn ensure_schema(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS midlang_permissions (
                name TEXT PRIMARY KEY,
                description TEXT NOT NULL DEFAULT ''
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS midlang_permission_groups (
                name TEXT PRIMARY KEY,
                description TEXT NOT NULL DEFAULT '',
                built_in INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS midlang_group_permissions (
                group_name TEXT NOT NULL REFERENCES midlang_permission_groups(name) ON DELETE CASCADE,
                permission_name TEXT NOT NULL REFERENCES midlang_permissions(name) ON DELETE CASCADE,
                PRIMARY KEY (group_name, permission_name)
            )",
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
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS midlang_token_groups (
                token_id TEXT NOT NULL REFERENCES midlang_tokens(id) ON DELETE CASCADE,
                group_name TEXT NOT NULL REFERENCES midlang_permission_groups(name),
                PRIMARY KEY (token_id, group_name)
            )",
        )
        .execute(&self.pool)
        .await?;
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
                sqlx::query("INSERT OR IGNORE INTO midlang_permissions (name) VALUES (?)")
                    .bind(*permission)
                    .execute(&self.pool)
                    .await?;
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

    pub async fn create_token(
        &self,
        name: &str,
        groups: &[String],
        expires_at: Option<i64>,
    ) -> Result<CreatedToken> {
        if name.trim().is_empty() {
            bail!("token name cannot be empty");
        }
        if groups.is_empty() {
            bail!("at least one permission group is required");
        }

        for group in groups {
            let exists: Option<String> =
                sqlx::query_scalar("SELECT name FROM midlang_permission_groups WHERE name = ?")
                    .bind(group)
                    .fetch_optional(&self.pool)
                    .await?;
            if exists.is_none() {
                bail!("permission group '{group}' does not exist");
            }
        }

        let id = Uuid::new_v4().to_string();
        let mut secret_bytes = [0_u8; 32];
        rand::rng().fill_bytes(&mut secret_bytes);
        let secret = hex_encode(&secret_bytes);
        let token_prefix = format!("{TOKEN_PREFIX}_{id}");
        let token = format!("{token_prefix}_{secret}");
        let secret_hash = hash_secret(&secret);
        let now = unix_now();

        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO midlang_tokens
             (id, name, token_prefix, secret_hash, status, expires_at, created_at)
             VALUES (?, ?, ?, ?, 'active', ?, ?)",
        )
        .bind(&id)
        .bind(name)
        .bind(&token_prefix)
        .bind(secret_hash)
        .bind(expires_at)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        for group in groups {
            sqlx::query("INSERT INTO midlang_token_groups (token_id, group_name) VALUES (?, ?)")
                .bind(&id)
                .bind(group)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;

        Ok(CreatedToken {
            id,
            name: name.to_string(),
            token,
            token_prefix,
            groups: groups.to_vec(),
            expires_at,
        })
    }

    /// Creates the first administrator token when the auth database is empty.
    /// Returns `None` once an initial token already exists.
    pub async fn bootstrap_admin_token(&self) -> Result<Option<CreatedToken>> {
        let token_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM midlang_tokens")
            .fetch_one(&self.pool)
            .await?;
        if token_count != 0 {
            return Ok(None);
        }

        Ok(Some(
            self.create_token("bootstrap-admin", &["admin".to_string()], None)
                .await?,
        ))
    }

    pub async fn authenticate(&self, authorization: &str) -> Result<Option<AuthContext>> {
        let token = authorization
            .strip_prefix("Bearer ")
            .ok_or_else(|| anyhow!("authorization must use the Bearer scheme"))?;
        let mut parts = token.splitn(3, '_');
        if parts.next() != Some(TOKEN_PREFIX) {
            bail!("invalid token format");
        }
        let token_id = parts
            .next()
            .ok_or_else(|| anyhow!("invalid token format"))?;
        let secret = parts
            .next()
            .ok_or_else(|| anyhow!("invalid token format"))?;

        let row = sqlx::query(
            "SELECT id, name, secret_hash, status, expires_at
             FROM midlang_tokens WHERE id = ?",
        )
        .bind(token_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else { return Ok(None) };

        let expected_hash: String = row.get("secret_hash");
        let status: String = row.get("status");
        let expires_at: Option<i64> = row.get("expires_at");
        if status != "active"
            || expires_at.is_some_and(|expires_at| expires_at <= unix_now())
            || hash_secret(secret) != expected_hash
        {
            return Ok(None);
        }

        let groups: Vec<String> = sqlx::query_scalar(
            "SELECT group_name FROM midlang_token_groups WHERE token_id = ? ORDER BY group_name",
        )
        .bind(token_id)
        .fetch_all(&self.pool)
        .await?;
        let permissions: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT gp.permission_name
             FROM midlang_token_groups tg
             JOIN midlang_group_permissions gp ON gp.group_name = tg.group_name
             WHERE tg.token_id = ? ORDER BY gp.permission_name",
        )
        .bind(token_id)
        .fetch_all(&self.pool)
        .await?;
        sqlx::query("UPDATE midlang_tokens SET last_used_at = ? WHERE id = ?")
            .bind(unix_now())
            .bind(token_id)
            .execute(&self.pool)
            .await?;

        Ok(Some(AuthContext {
            token_id: token_id.to_string(),
            name: row.get("name"),
            groups,
            permissions,
            expires_at,
        }))
    }

    pub async fn revoke(&self, token_id: &str) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE midlang_tokens SET status = 'revoked', revoked_at = ?
             WHERE id = ? AND status = 'active'",
        )
        .bind(unix_now())
        .bind(token_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn list_tokens(&self) -> Result<Vec<TokenInfo>> {
        let rows = sqlx::query(
            "SELECT id, name, token_prefix, status, expires_at, created_at,
                    last_used_at, revoked_at
             FROM midlang_tokens ORDER BY created_at DESC, id DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut tokens = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.get("id");
            let groups = sqlx::query_scalar(
                "SELECT group_name FROM midlang_token_groups
                 WHERE token_id = ? ORDER BY group_name",
            )
            .bind(&id)
            .fetch_all(&self.pool)
            .await?;
            tokens.push(TokenInfo {
                id,
                name: row.get("name"),
                token_prefix: row.get("token_prefix"),
                status: row.get("status"),
                groups,
                expires_at: row.get("expires_at"),
                created_at: row.get("created_at"),
                last_used_at: row.get("last_used_at"),
                revoked_at: row.get("revoked_at"),
            });
        }
        Ok(tokens)
    }

    pub async fn rotate_token(&self, token_id: &str) -> Result<Option<CreatedToken>> {
        let row = sqlx::query(
            "SELECT name, expires_at FROM midlang_tokens
             WHERE id = ? AND status = 'active'",
        )
        .bind(token_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else { return Ok(None) };

        let groups = sqlx::query_scalar(
            "SELECT group_name FROM midlang_token_groups
             WHERE token_id = ? ORDER BY group_name",
        )
        .bind(token_id)
        .fetch_all(&self.pool)
        .await?;
        let name: String = row.get("name");
        let expires_at: Option<i64> = row.get("expires_at");
        let replacement = self.create_token(&name, &groups, expires_at).await?;
        if !self.revoke(token_id).await? {
            self.revoke(&replacement.id).await?;
            return Ok(None);
        }
        Ok(Some(replacement))
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before Unix epoch")
        .as_secs() as i64
}

fn hash_secret(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    hex_encode(&digest)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn creates_and_authenticates_a_persisted_token() {
        let store = AuthStore::connect("sqlite::memory:").await.unwrap();
        let created = store
            .create_token("test-reader", &["reader".to_string()], None)
            .await
            .unwrap();
        let context = store
            .authenticate(&format!("Bearer {}", created.token))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(context.token_id, created.id);
        assert!(context.can("translation:read"));
        assert!(!context.can("translation:write"));
    }

    #[tokio::test]
    async fn revoked_tokens_are_rejected() {
        let store = AuthStore::connect("sqlite::memory:").await.unwrap();
        let created = store
            .create_token("test-writer", &["writer".to_string()], None)
            .await
            .unwrap();
        assert!(store.revoke(&created.id).await.unwrap());
        assert!(
            store
                .authenticate(&format!("Bearer {}", created.token))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn bootstraps_only_one_admin_token() {
        let store = AuthStore::connect("sqlite::memory:").await.unwrap();
        let first = store.bootstrap_admin_token().await.unwrap().unwrap();
        assert_eq!(first.groups, vec!["admin"]);
        assert!(store.bootstrap_admin_token().await.unwrap().is_none());
    }
}
