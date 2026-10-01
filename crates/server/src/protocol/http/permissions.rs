use axum::{Json, extract::Extension};

use crate::secure::AuthContext;

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PermissionsResponse {
    /// Permissions granted to the bearer token used for this request.
    pub permissions: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/auth/permissions",
    responses(
        (status = 200, description = "Permissions granted to the current token", body = PermissionsResponse),
        (status = 401, description = "Missing or invalid token", body = crate::protocol::error::ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn permissions_handler(
    Extension(auth): Extension<AuthContext>,
) -> Json<PermissionsResponse> {
    Json(PermissionsResponse {
        permissions: auth.permissions,
    })
}
