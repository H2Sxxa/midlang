use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use midlang_core::{
    internals::changelog::{ChangeRecord, ChangeState, ChangelogCursor, ChangelogFilter},
    query::{SortOrder, pagination::DEFAULT_PAGE_SIZE},
    translation::{RollbackOutcome, Translation},
};

use crate::{
    protocol::error::{self, ErrorBody},
    secure::{self, AuthContext},
};

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ChangelogListQuery {
    /// Filter by locale.
    pub locale: Option<String>,
    /// Filter by change state: `created`, `updated` or `deleted`.
    pub state: Option<String>,
    /// Case-insensitive substring matched against the key or either value.
    pub keyword: Option<String>,
    /// Sort direction: `asc` or `desc` (default). A cursor carries its own direction.
    pub order: Option<String>,
    /// Number of entries to return. Defaults to 100.
    pub limit: Option<usize>,
    /// Cursor returned by the previous page.
    pub cursor: Option<String>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ChangelogMessage {
    pub id: i64,
    pub locale: String,
    pub key: String,
    pub state: String,
    /// Value before the change; `null` when the key did not exist yet.
    pub previous_value: Option<String>,
    /// Value after the change; `null` when the key was deleted.
    pub new_value: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ChangelogListPage {
    pub items: Vec<ChangelogMessage>,
    /// URL-safe cursor token for the next page, if one exists.
    pub next: Option<String>,
}

impl From<ChangeRecord> for ChangelogMessage {
    fn from(record: ChangeRecord) -> Self {
        ChangelogMessage {
            id: record.id,
            locale: record.locale,
            key: record.key,
            state: record.state.to_string(),
            previous_value: record.previous_value,
            new_value: record.new_value,
            created_at: record.created_at,
        }
    }
}

pub fn router<Store>() -> Router<Translation<Store>>
where
    Store: midlang_core::store::KVStore + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/changelog", get(list_changes_handler::<Store>))
        .route("/changelog/{id}", get(get_change_handler::<Store>))
        .route(
            "/changelog/{id}/rollback",
            post(rollback_change_handler::<Store>),
        )
}

fn parse_state(value: Option<&str>) -> Result<Option<ChangeState>, Response> {
    match value.map(ChangeState::parse) {
        Some(Some(state)) => Ok(Some(state)),
        Some(None) => Err(error::bad_request(
            "state must be one of created, updated or deleted",
        )),
        None => Ok(None),
    }
}

fn parse_order(value: Option<&str>) -> Result<SortOrder, Response> {
    match value.map(|value| value.to_ascii_lowercase()).as_deref() {
        Some("asc") => Ok(SortOrder::Asc),
        Some("desc") => Ok(SortOrder::Desc),
        Some(_) => Err(error::bad_request("order must be either asc or desc")),
        None => Ok(SortOrder::Desc),
    }
}

/// Renders an optional stored value for an error message.
fn describe(value: &Option<String>) -> String {
    match value {
        Some(value) => format!("'{value}'"),
        None => "no value".to_string(),
    }
}

#[utoipa::path(
    get,
    path = "/changelog",
    params(ChangelogListQuery),
    responses(
        (status = 200, description = "Paged changes", body = ChangelogListPage),
        (status = 400, description = "Invalid query or cursor", body = ErrorBody),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 500, description = "Changelog listing unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_changes_handler<Store>(
    Query(query): Query<ChangelogListQuery>,
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
) -> Response
where
    Store: midlang_core::store::KVRead + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "diagnostic:read") {
        return response;
    }

    let state = match parse_state(query.state.as_deref()) {
        Ok(state) => state,
        Err(response) => return response,
    };
    let order = match parse_order(query.order.as_deref()) {
        Ok(order) => order,
        Err(response) => return response,
    };

    let cursor = match query.cursor {
        Some(token) => match ChangelogCursor::decode(&token) {
            Ok(cursor) => cursor,
            Err(err) => return error::bad_request(format!("invalid cursor: {err}")),
        },
        None => ChangelogCursor::new(
            order,
            ChangelogFilter {
                locale: query.locale.clone(),
                state,
                keyword: query.keyword.clone(),
            },
        ),
    };
    let limit = query.limit.unwrap_or(DEFAULT_PAGE_SIZE);

    match translation.list_changes(&cursor, limit).await {
        Ok(page) => {
            let next = match page.next {
                Some(cursor) => match cursor.encode() {
                    Ok(token) => Some(token),
                    Err(err) => return error::diagnostics(err),
                },
                None => None,
            };
            (
                StatusCode::OK,
                Json(ChangelogListPage {
                    items: page.items.into_iter().map(ChangelogMessage::from).collect(),
                    next,
                }),
            )
                .into_response()
        }
        Err(err) => error::diagnostics(err),
    }
}

#[utoipa::path(
    get,
    path = "/changelog/{id}",
    params(
        ("id" = i64, Path, description = "Changelog entry identifier")
    ),
    responses(
        (status = 200, description = "Entry found", body = ChangelogMessage),
        (status = 400, description = "Invalid identifier", body = ErrorBody),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Entry not found", body = ErrorBody),
        (status = 500, description = "Changelog lookup unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_change_handler<Store>(
    Path(id): Path<i64>,
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
) -> Response
where
    Store: midlang_core::store::KVRead + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "diagnostic:read") {
        return response;
    }

    match translation.change(id).await {
        Ok(Some(record)) => (StatusCode::OK, Json(ChangelogMessage::from(record))).into_response(),
        Ok(None) => error::not_found(format!("changelog entry '{id}' does not exist")),
        Err(err) => error::diagnostics(err),
    }
}

#[utoipa::path(
    post,
    path = "/changelog/{id}/rollback",
    params(
        ("id" = i64, Path, description = "Changelog entry identifier")
    ),
    responses(
        (status = 200, description = "Entry rolled back", body = ChangelogMessage),
        (status = 400, description = "Invalid identifier", body = ErrorBody),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Entry not found", body = ErrorBody),
        (status = 409, description = "Entry was superseded by a later change", body = ErrorBody),
        (status = 500, description = "Rollback unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn rollback_change_handler<Store>(
    Path(id): Path<i64>,
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
) -> Response
where
    Store: midlang_core::store::KVStore + Clone + Send + Sync + 'static,
{
    // Rolling back writes a translation, so it needs both the authority to
    // change translations and the visibility into the history it acts on.
    if let Err(response) = secure::require_permission(&auth, "diagnostic:read") {
        return response;
    }
    if let Err(response) = secure::require_permission(&auth, "translation:write") {
        return response;
    }

    match translation.rollback(id).await {
        Ok(RollbackOutcome::Restored(entry)) => {
            (StatusCode::OK, Json(ChangelogMessage::from(entry))).into_response()
        }
        Ok(RollbackOutcome::Conflict { expected, actual }) => error::conflict(format!(
            "changelog entry '{id}' expects the store to hold {} but it holds {}",
            describe(&expected),
            describe(&actual)
        )),
        Ok(RollbackOutcome::Missing) => {
            error::not_found(format!("changelog entry '{id}' does not exist"))
        }
        Err(err) => error::diagnostics(err),
    }
}
