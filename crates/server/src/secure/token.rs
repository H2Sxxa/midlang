use anyhow::{Result, anyhow, bail};
use midlang_core::time::unix_now;
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use super::builtins::expand_permissions;
use super::group::normalized_names;
use super::{AuthContext, AuthStore};

const TOKEN_PREFIX: &str = "mlk";

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

pub enum UpdateTokenOutcome {
    Updated(TokenInfo),
    NotFound,
    InvalidInput(String),
    UnknownGroups(Vec<String>),
}

impl AuthStore {
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
        let granted: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT gp.permission_name
             FROM midlang_token_groups tg
             JOIN midlang_group_permissions gp ON gp.group_name = tg.group_name
             WHERE tg.token_id = ? ORDER BY gp.permission_name",
        )
        .bind(token_id)
        .fetch_all(&self.pool)
        .await?;
        let permissions = expand_permissions(&granted);
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
            tokens.push(self.build_token_info(row).await?);
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

    pub async fn update_token(
        &self,
        token_id: &str,
        name: &str,
        groups: &[String],
        expires_at: Option<i64>,
    ) -> Result<UpdateTokenOutcome> {
        if name.trim().is_empty() {
            return Ok(UpdateTokenOutcome::InvalidInput(
                "token name cannot be empty".to_string(),
            ));
        }
        let groups = normalized_names(groups);
        if groups.is_empty() {
            return Ok(UpdateTokenOutcome::InvalidInput(
                "at least one permission group is required".to_string(),
            ));
        }

        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM midlang_tokens WHERE id = ? AND status = 'active'")
                .bind(token_id)
                .fetch_optional(&self.pool)
                .await?;
        if exists.is_none() {
            return Ok(UpdateTokenOutcome::NotFound);
        }

        let mut unknown = Vec::new();
        for group in &groups {
            if !self.group_exists(group).await? {
                unknown.push(group.clone());
            }
        }
        if !unknown.is_empty() {
            return Ok(UpdateTokenOutcome::UnknownGroups(unknown));
        }

        let mut transaction = self.pool.begin().await?;
        sqlx::query("UPDATE midlang_tokens SET name = ?, expires_at = ? WHERE id = ?")
            .bind(name)
            .bind(expires_at)
            .bind(token_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM midlang_token_groups WHERE token_id = ?")
            .bind(token_id)
            .execute(&mut *transaction)
            .await?;
        for group in &groups {
            sqlx::query("INSERT INTO midlang_token_groups (token_id, group_name) VALUES (?, ?)")
                .bind(token_id)
                .bind(group)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;

        Ok(match self.token_info(token_id).await? {
            Some(info) => UpdateTokenOutcome::Updated(info),
            None => UpdateTokenOutcome::NotFound,
        })
    }

    async fn token_info(&self, token_id: &str) -> Result<Option<TokenInfo>> {
        let row = sqlx::query(
            "SELECT id, name, token_prefix, status, expires_at, created_at,
                    last_used_at, revoked_at
             FROM midlang_tokens WHERE id = ?",
        )
        .bind(token_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else { return Ok(None) };
        Ok(Some(self.build_token_info(row).await?))
    }

    async fn build_token_info(&self, row: sqlx::sqlite::SqliteRow) -> Result<TokenInfo> {
        let id: String = row.get("id");
        let groups = sqlx::query_scalar(
            "SELECT group_name FROM midlang_token_groups
             WHERE token_id = ? ORDER BY group_name",
        )
        .bind(&id)
        .fetch_all(&self.pool)
        .await?;
        Ok(TokenInfo {
            id,
            name: row.get("name"),
            token_prefix: row.get("token_prefix"),
            status: row.get("status"),
            groups,
            expires_at: row.get("expires_at"),
            created_at: row.get("created_at"),
            last_used_at: row.get("last_used_at"),
            revoked_at: row.get("revoked_at"),
        })
    }
}

fn hash_secret(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    hex_encode(&digest)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
