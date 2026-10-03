use anyhow::Result;
use midlang_core::time::unix_now;
use sqlx::Row;

use super::AuthStore;
use super::builtins::{BUILTIN_PERMISSIONS, GROUP_NAME_MAX_LEN, transitive_implies};

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct PermissionInfo {
    pub name: String,
    pub description: String,
    /// Permissions this one includes, transitively, so a client can tell that
    /// granting `translation:write` already covers `translation:read`.
    pub implies: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct GroupInfo {
    pub name: String,
    pub description: String,
    pub built_in: bool,
    pub permissions: Vec<String>,
    pub created_at: i64,
}

/// Outcome of creating a permission group. Expected failures are modelled
/// explicitly so callers can map them to distinct status codes.
pub enum CreateGroupOutcome {
    Created(GroupInfo),
    AlreadyExists,
    InvalidInput(String),
    UnknownPermissions(Vec<String>),
}

pub enum UpdateGroupOutcome {
    Updated(GroupInfo),
    NotFound,
    BuiltIn,
    InvalidInput(String),
    UnknownPermissions(Vec<String>),
}

pub enum DeleteGroupOutcome {
    Deleted,
    NotFound,
    BuiltIn,
    InUse(Vec<String>),
}

impl AuthStore {
    pub async fn list_permissions(&self) -> Result<Vec<PermissionInfo>> {
        Ok(BUILTIN_PERMISSIONS
            .iter()
            .map(|permission| PermissionInfo {
                name: permission.name.to_string(),
                description: permission.description.to_string(),
                implies: transitive_implies(permission.name),
            })
            .collect())
    }

    pub async fn list_groups(&self) -> Result<Vec<GroupInfo>> {
        let rows = sqlx::query(
            "SELECT name, description, built_in, created_at
             FROM midlang_permission_groups ORDER BY built_in DESC, name",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut groups = Vec::with_capacity(rows.len());
        for row in rows {
            let name: String = row.get("name");
            let permissions = self.group_permissions(&name).await?;
            groups.push(GroupInfo {
                name,
                description: row.get("description"),
                built_in: row.get::<i64, _>("built_in") != 0,
                permissions,
                created_at: row.get("created_at"),
            });
        }
        Ok(groups)
    }

    pub async fn create_group(
        &self,
        name: &str,
        description: &str,
        permissions: &[String],
    ) -> Result<CreateGroupOutcome> {
        if let Err(message) = validate_group_name(name) {
            return Ok(CreateGroupOutcome::InvalidInput(message));
        }
        if self.group_exists(name).await? {
            return Ok(CreateGroupOutcome::AlreadyExists);
        }

        let permissions = normalized_names(permissions);
        let unknown = self.unknown_permissions(&permissions).await?;
        if !unknown.is_empty() {
            return Ok(CreateGroupOutcome::UnknownPermissions(unknown));
        }

        let now = unix_now();
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO midlang_permission_groups
             (name, description, built_in, created_at) VALUES (?, ?, 0, ?)",
        )
        .bind(name)
        .bind(description)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        for permission in &permissions {
            sqlx::query(
                "INSERT INTO midlang_group_permissions (group_name, permission_name)
                 VALUES (?, ?)",
            )
            .bind(name)
            .bind(permission)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;

        Ok(CreateGroupOutcome::Created(GroupInfo {
            name: name.to_string(),
            description: description.to_string(),
            built_in: false,
            permissions,
            created_at: now,
        }))
    }

    pub async fn update_group(
        &self,
        name: &str,
        description: &str,
        permissions: &[String],
    ) -> Result<UpdateGroupOutcome> {
        let row = sqlx::query("SELECT built_in FROM midlang_permission_groups WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(UpdateGroupOutcome::NotFound);
        };
        if row.get::<i64, _>("built_in") != 0 {
            return Ok(UpdateGroupOutcome::BuiltIn);
        }

        let permissions = normalized_names(permissions);
        let unknown = self.unknown_permissions(&permissions).await?;
        if !unknown.is_empty() {
            return Ok(UpdateGroupOutcome::UnknownPermissions(unknown));
        }

        let mut transaction = self.pool.begin().await?;
        sqlx::query("UPDATE midlang_permission_groups SET description = ? WHERE name = ?")
            .bind(description)
            .bind(name)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM midlang_group_permissions WHERE group_name = ?")
            .bind(name)
            .execute(&mut *transaction)
            .await?;
        for permission in &permissions {
            sqlx::query(
                "INSERT INTO midlang_group_permissions (group_name, permission_name)
                 VALUES (?, ?)",
            )
            .bind(name)
            .bind(permission)
            .execute(&mut *transaction)
            .await?;
        }
        let created_at: i64 =
            sqlx::query_scalar("SELECT created_at FROM midlang_permission_groups WHERE name = ?")
                .bind(name)
                .fetch_one(&mut *transaction)
                .await?;
        transaction.commit().await?;

        Ok(UpdateGroupOutcome::Updated(GroupInfo {
            name: name.to_string(),
            description: description.to_string(),
            built_in: false,
            permissions,
            created_at,
        }))
    }

    pub async fn delete_group(&self, name: &str) -> Result<DeleteGroupOutcome> {
        let row = sqlx::query("SELECT built_in FROM midlang_permission_groups WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(DeleteGroupOutcome::NotFound);
        };
        if row.get::<i64, _>("built_in") != 0 {
            return Ok(DeleteGroupOutcome::BuiltIn);
        }

        let in_use: Vec<String> = sqlx::query_scalar(
            "SELECT token_id FROM midlang_token_groups
             WHERE group_name = ? ORDER BY token_id",
        )
        .bind(name)
        .fetch_all(&self.pool)
        .await?;
        if !in_use.is_empty() {
            return Ok(DeleteGroupOutcome::InUse(in_use));
        }

        sqlx::query("DELETE FROM midlang_permission_groups WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(DeleteGroupOutcome::Deleted)
    }

    async fn group_permissions(&self, name: &str) -> Result<Vec<String>> {
        let permissions = sqlx::query_scalar(
            "SELECT permission_name FROM midlang_group_permissions
             WHERE group_name = ? ORDER BY permission_name",
        )
        .bind(name)
        .fetch_all(&self.pool)
        .await?;
        Ok(permissions)
    }

    pub(super) async fn group_exists(&self, name: &str) -> Result<bool> {
        let exists: Option<String> =
            sqlx::query_scalar("SELECT name FROM midlang_permission_groups WHERE name = ?")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;
        Ok(exists.is_some())
    }

    async fn unknown_permissions(&self, permissions: &[String]) -> Result<Vec<String>> {
        let mut unknown = Vec::new();
        for permission in permissions {
            let exists: Option<String> =
                sqlx::query_scalar("SELECT name FROM midlang_permissions WHERE name = ?")
                    .bind(permission)
                    .fetch_optional(&self.pool)
                    .await?;
            if exists.is_none() {
                unknown.push(permission.clone());
            }
        }
        Ok(unknown)
    }
}

/// Trims, sorts and de-duplicates a list of names so database inserts never
/// depend on the request order or contain repeated entries. Blank entries are
/// kept so callers report them as unknown instead of silently dropping input.
pub(super) fn normalized_names(names: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = names.iter().map(|name| name.trim().to_string()).collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn validate_group_name(name: &str) -> std::result::Result<(), String> {
    if name.is_empty() {
        return Err("group name cannot be empty".to_string());
    }
    if name.len() > GROUP_NAME_MAX_LEN {
        return Err(format!(
            "group name cannot exceed {GROUP_NAME_MAX_LEN} characters"
        ));
    }
    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        return Err(
            "group name may only contain ASCII letters, digits, '-', '_' and '.'".to_string(),
        );
    }
    Ok(())
}
