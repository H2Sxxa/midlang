//! Authentication and authorization for the server boundary.
//!
//! This module deliberately lives outside `midlang-core`: tokens, permission
//! groups and HTTP-facing authorization are transport concerns, while core
//! owns translation data and its storage abstractions.

mod builtins;
mod group;
mod store;
mod token;

pub use group::{
    CreateGroupOutcome, DeleteGroupOutcome, GroupInfo, PermissionInfo, UpdateGroupOutcome,
};
pub use token::{CreatedToken, TokenInfo, UpdateTokenOutcome};

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AuthStore {
    pool: SqlitePool,
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

#[cfg(test)]
mod tests;
