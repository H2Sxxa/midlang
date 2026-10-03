use axum::{
    Json, Router,
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, put},
};

use crate::{
    protocol::error::{self, ErrorBody},
    secure::{
        AuthContext, AuthStore, CreateGroupOutcome, DeleteGroupOutcome, GroupInfo, PermissionInfo,
        UpdateGroupOutcome, require_permission,
    },
};

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateGroupRequest {
    pub description: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct GroupListResponse {
    pub groups: Vec<GroupInfo>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PermissionListResponse {
    pub permissions: Vec<PermissionInfo>,
}

pub fn router<State>() -> Router<State>
where
    State: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/auth/permissions/catalog", get(list_permissions_handler))
        .route(
            "/auth/groups",
            get(list_groups_handler).post(create_group_handler),
        )
        .route(
            "/auth/groups/{name}",
            put(update_group_handler).delete(delete_group_handler),
        )
}

#[utoipa::path(
    get,
    path = "/auth/permissions/catalog",
    responses(
        (status = 200, description = "Permission catalog known to the server", body = PermissionListResponse),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 500, description = "Permission catalog unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_permissions_handler(
    Extension(auth): Extension<AuthContext>,
    Extension(store): Extension<AuthStore>,
) -> Response {
    if let Err(response) = require_permission(&auth, "permission:read") {
        return response;
    }
    match store.list_permissions().await {
        Ok(permissions) => Json(PermissionListResponse { permissions }).into_response(),
        Err(error) => group_error(error),
    }
}

#[utoipa::path(
    get,
    path = "/auth/groups",
    responses(
        (status = 200, description = "Permission groups with their permissions", body = GroupListResponse),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 500, description = "Permission groups unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_groups_handler(
    Extension(auth): Extension<AuthContext>,
    Extension(store): Extension<AuthStore>,
) -> Response {
    if let Err(response) = require_permission(&auth, "permission:read") {
        return response;
    }
    match store.list_groups().await {
        Ok(groups) => Json(GroupListResponse { groups }).into_response(),
        Err(error) => group_error(error),
    }
}

#[utoipa::path(
    post,
    path = "/auth/groups",
    request_body = CreateGroupRequest,
    responses(
        (status = 201, description = "Permission group created", body = GroupInfo),
        (status = 400, description = "Invalid group or unknown permission", body = ErrorBody),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 409, description = "A group with that name already exists", body = ErrorBody),
        (status = 500, description = "Permission group creation unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_group_handler(
    Extension(auth): Extension<AuthContext>,
    Extension(store): Extension<AuthStore>,
    Json(request): Json<CreateGroupRequest>,
) -> Response {
    if let Err(response) = require_permission(&auth, "permission:manage") {
        return response;
    }
    match store
        .create_group(&request.name, &request.description, &request.permissions)
        .await
    {
        Ok(CreateGroupOutcome::Created(group)) => {
            (StatusCode::CREATED, Json(group)).into_response()
        }
        Ok(CreateGroupOutcome::AlreadyExists) => {
            error::conflict("a permission group with that name already exists")
        }
        Ok(CreateGroupOutcome::InvalidInput(message)) => error::bad_request(message),
        Ok(CreateGroupOutcome::UnknownPermissions(unknown)) => {
            error::bad_request(format!("unknown permissions: {}", unknown.join(", ")))
        }
        Err(error) => group_error(error),
    }
}

#[utoipa::path(
    put,
    path = "/auth/groups/{name}",
    params(("name" = String, Path, description = "Permission group name")),
    request_body = UpdateGroupRequest,
    responses(
        (status = 200, description = "Permission group updated", body = GroupInfo),
        (status = 400, description = "Unknown permission", body = ErrorBody),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Permission group not found", body = ErrorBody),
        (status = 409, description = "Built-in groups cannot be modified", body = ErrorBody),
        (status = 500, description = "Permission group update unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_group_handler(
    Extension(auth): Extension<AuthContext>,
    Extension(store): Extension<AuthStore>,
    Path(name): Path<String>,
    Json(request): Json<UpdateGroupRequest>,
) -> Response {
    if let Err(response) = require_permission(&auth, "permission:manage") {
        return response;
    }
    match store
        .update_group(&name, &request.description, &request.permissions)
        .await
    {
        Ok(UpdateGroupOutcome::Updated(group)) => Json(group).into_response(),
        Ok(UpdateGroupOutcome::NotFound) => {
            error::not_found("permission group not found".to_string())
        }
        Ok(UpdateGroupOutcome::BuiltIn) => {
            error::conflict("built-in permission groups cannot be modified")
        }
        Ok(UpdateGroupOutcome::InvalidInput(message)) => error::bad_request(message),
        Ok(UpdateGroupOutcome::UnknownPermissions(unknown)) => {
            error::bad_request(format!("unknown permissions: {}", unknown.join(", ")))
        }
        Err(error) => group_error(error),
    }
}

#[utoipa::path(
    delete,
    path = "/auth/groups/{name}",
    params(("name" = String, Path, description = "Permission group name")),
    responses(
        (status = 204, description = "Permission group deleted"),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Permission group not found", body = ErrorBody),
        (status = 409, description = "Built-in or assigned groups cannot be deleted", body = ErrorBody),
        (status = 500, description = "Permission group deletion unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_group_handler(
    Extension(auth): Extension<AuthContext>,
    Extension(store): Extension<AuthStore>,
    Path(name): Path<String>,
) -> Response {
    if let Err(response) = require_permission(&auth, "permission:manage") {
        return response;
    }
    match store.delete_group(&name).await {
        Ok(DeleteGroupOutcome::Deleted) => StatusCode::NO_CONTENT.into_response(),
        Ok(DeleteGroupOutcome::NotFound) => {
            error::not_found("permission group not found".to_string())
        }
        Ok(DeleteGroupOutcome::BuiltIn) => {
            error::conflict("built-in permission groups cannot be deleted")
        }
        Ok(DeleteGroupOutcome::InUse(tokens)) => error::conflict(format!(
            "permission group is assigned to tokens: {}",
            tokens.join(", ")
        )),
        Err(error) => group_error(error),
    }
}

fn group_error(error: anyhow::Error) -> Response {
    tracing::error!(error = ?error, "permission group request failed");
    error::internal("permission group management unavailable")
}
