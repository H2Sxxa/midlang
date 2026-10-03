use axum::{
    Json, Router,
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};

use crate::{
    protocol::error::ErrorBody,
    secure::{self, AuthContext, AuthStore, CreatedToken, TokenInfo, UpdateTokenOutcome},
};

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateTokenRequest {
    pub name: String,
    pub groups: Vec<String>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TokenListResponse {
    pub tokens: Vec<TokenInfo>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateTokenRequest {
    pub name: String,
    pub groups: Vec<String>,
    pub expires_at: Option<i64>,
}

pub fn router<State>() -> Router<State>
where
    State: Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/auth/tokens",
            get(list_tokens_handler).post(create_token_handler),
        )
        .route(
            "/auth/tokens/{id}",
            delete(revoke_token_handler).patch(update_token_handler),
        )
        .route("/auth/tokens/{id}/rotate", post(rotate_token_handler))
}

#[utoipa::path(
    get,
    path = "/auth/tokens",
    responses(
        (status = 200, description = "Authentication tokens", body = TokenListResponse),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 500, description = "Token listing unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_tokens_handler(
    Extension(auth): Extension<AuthContext>,
    Extension(store): Extension<AuthStore>,
) -> Response {
    if let Err(response) = secure::require_permission(&auth, "token:read") {
        return response;
    }
    match store.list_tokens().await {
        Ok(tokens) => Json(TokenListResponse { tokens }).into_response(),
        Err(error) => token_error(error),
    }
}

#[utoipa::path(
    post,
    path = "/auth/tokens",
    request_body = CreateTokenRequest,
    responses(
        (status = 201, description = "Token created", body = CreatedToken),
        (status = 400, description = "Invalid token request", body = ErrorBody),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 500, description = "Token creation unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_token_handler(
    Extension(auth): Extension<AuthContext>,
    Extension(store): Extension<AuthStore>,
    Json(request): Json<CreateTokenRequest>,
) -> Response {
    if let Err(response) = secure::require_permission(&auth, "token:create") {
        return response;
    }
    match store
        .create_token(&request.name, &request.groups, request.expires_at)
        .await
    {
        Ok(token) => (StatusCode::CREATED, Json(token)).into_response(),
        Err(error) => crate::protocol::error::bad_request(error.to_string()),
    }
}

#[utoipa::path(
    delete,
    path = "/auth/tokens/{id}",
    params(("id" = String, Path, description = "Token identifier")),
    responses(
        (status = 204, description = "Token revoked"),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Token not found", body = ErrorBody),
        (status = 500, description = "Token revocation unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn revoke_token_handler(
    Extension(auth): Extension<AuthContext>,
    Extension(store): Extension<AuthStore>,
    Path(id): Path<String>,
) -> Response {
    if let Err(response) = secure::require_permission(&auth, "token:revoke") {
        return response;
    }
    match store.revoke(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => crate::protocol::error::not_found("token not found or already revoked".into()),
        Err(error) => token_error(error),
    }
}

#[utoipa::path(
    post,
    path = "/auth/tokens/{id}/rotate",
    params(("id" = String, Path, description = "Token identifier")),
    responses(
        (status = 201, description = "Replacement token created", body = CreatedToken),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Token not found or inactive", body = ErrorBody),
        (status = 500, description = "Token rotation unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn rotate_token_handler(
    Extension(auth): Extension<AuthContext>,
    Extension(store): Extension<AuthStore>,
    Path(id): Path<String>,
) -> Response {
    if let Err(response) = secure::require_permission(&auth, "token:rotate") {
        return response;
    }
    match store.rotate_token(&id).await {
        Ok(Some(token)) => (StatusCode::CREATED, Json(token)).into_response(),
        Ok(None) => crate::protocol::error::not_found("token not found or inactive".into()),
        Err(error) => token_error(error),
    }
}

#[utoipa::path(
    patch,
    path = "/auth/tokens/{id}",
    params(("id" = String, Path, description = "Token identifier")),
    request_body = UpdateTokenRequest,
    responses(
        (status = 200, description = "Token updated", body = TokenInfo),
        (status = 400, description = "Invalid token request", body = ErrorBody),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Token not found or inactive", body = ErrorBody),
        (status = 500, description = "Token update unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_token_handler(
    Extension(auth): Extension<AuthContext>,
    Extension(store): Extension<AuthStore>,
    Path(id): Path<String>,
    Json(request): Json<UpdateTokenRequest>,
) -> Response {
    if let Err(response) = secure::require_permission(&auth, "token:update") {
        return response;
    }
    match store
        .update_token(&id, &request.name, &request.groups, request.expires_at)
        .await
    {
        Ok(UpdateTokenOutcome::Updated(token)) => Json(token).into_response(),
        Ok(UpdateTokenOutcome::NotFound) => {
            crate::protocol::error::not_found("token not found or inactive".into())
        }
        Ok(UpdateTokenOutcome::InvalidInput(message)) => {
            crate::protocol::error::bad_request(message)
        }
        Ok(UpdateTokenOutcome::UnknownGroups(unknown)) => crate::protocol::error::bad_request(
            format!("unknown permission groups: {}", unknown.join(", ")),
        ),
        Err(error) => token_error(error),
    }
}

fn token_error(error: anyhow::Error) -> Response {
    tracing::error!(error = ?error, "token management request failed");
    crate::protocol::error::internal("token management unavailable")
}
