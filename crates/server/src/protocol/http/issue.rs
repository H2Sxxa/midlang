use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use midlang_core::{
    internals::issues::{IssueCursor, IssueFilter, IssueRecord, IssueSort, IssueState},
    query::{SortOrder, pagination::DEFAULT_PAGE_SIZE},
    translation::Translation,
};

use crate::{
    protocol::error::{self, ErrorBody},
    secure::{self, AuthContext},
};

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct IssueListQuery {
    /// Filter by state: `open`, `closed` or `ignored`.
    pub state: Option<String>,
    /// Filter by issue kind, e.g. `missing_translation` or `missing_locale`.
    pub kind: Option<String>,
    /// Case-insensitive substring matched against the summary, payload and kind.
    pub keyword: Option<String>,
    /// Sort field: `last_seen` (default), `created_at` or `count`.
    pub sort: Option<String>,
    /// Sort direction: `asc` or `desc` (default). A cursor carries its own direction.
    pub order: Option<String>,
    /// Number of issues to return. Defaults to 100.
    pub limit: Option<usize>,
    /// Cursor returned by the previous page.
    pub cursor: Option<String>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct IssueMessage {
    pub id: String,
    pub kind: String,
    pub payload: serde_json::Value,
    pub summary: String,
    pub count: usize,
    pub created_at: i64,
    pub last_seen: i64,
    pub state: String,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct IssueListPage {
    pub items: Vec<IssueMessage>,
    /// URL-safe cursor token for the next page, if one exists.
    pub next: Option<String>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateIssueRequest {
    /// New state: `open`, `closed` or `ignored`.
    pub state: String,
}

impl From<IssueRecord> for IssueMessage {
    fn from(record: IssueRecord) -> Self {
        IssueMessage {
            id: record.id,
            kind: record.kind,
            payload: record.payload,
            summary: record.summary,
            count: record.count,
            created_at: record.created_at,
            last_seen: record.last_seen,
            state: record.state.to_string(),
        }
    }
}

pub fn router<Store>() -> Router<Translation<Store>>
where
    Store: midlang_core::store::KVRead + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/issues", get(list_issues_handler::<Store>))
        .route(
            "/issues/{id}",
            get(get_issue_handler::<Store>).patch(update_issue_handler::<Store>),
        )
}

fn parse_state(value: Option<&str>) -> Result<Option<IssueState>, Response> {
    match value.map(IssueState::parse) {
        Some(Some(state)) => Ok(Some(state)),
        Some(None) => Err(error::bad_request(
            "state must be one of open, closed or ignored",
        )),
        None => Ok(None),
    }
}

fn parse_sort(value: Option<&str>) -> Result<IssueSort, Response> {
    match value.map(|value| value.to_ascii_lowercase()).as_deref() {
        Some("last_seen") => Ok(IssueSort::LastSeen),
        Some("created_at") => Ok(IssueSort::CreatedAt),
        Some("count") => Ok(IssueSort::Count),
        Some(_) => Err(error::bad_request(
            "sort must be one of last_seen, created_at or count",
        )),
        None => Ok(IssueSort::LastSeen),
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

#[utoipa::path(
    get,
    path = "/issues",
    params(IssueListQuery),
    responses(
        (status = 200, description = "Paged issues", body = IssueListPage),
        (status = 400, description = "Invalid query or cursor", body = ErrorBody),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 500, description = "Issue listing unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_issues_handler<Store>(
    Query(query): Query<IssueListQuery>,
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
    let sort = match parse_sort(query.sort.as_deref()) {
        Ok(sort) => sort,
        Err(response) => return response,
    };
    let order = match parse_order(query.order.as_deref()) {
        Ok(order) => order,
        Err(response) => return response,
    };

    let cursor = match query.cursor {
        Some(token) => match IssueCursor::decode(&token) {
            Ok(cursor) => cursor,
            Err(err) => return error::bad_request(format!("invalid cursor: {err}")),
        },
        None => IssueCursor::new(
            order,
            IssueFilter {
                state,
                kind: query.kind.clone(),
                keyword: query.keyword.clone(),
                sort,
            },
        ),
    };
    let limit = query.limit.unwrap_or(DEFAULT_PAGE_SIZE);

    match translation.list_issues(&cursor, limit).await {
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
                Json(IssueListPage {
                    items: page.items.into_iter().map(IssueMessage::from).collect(),
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
    path = "/issues/{id}",
    params(
        ("id" = String, Path, description = "Issue identifier")
    ),
    responses(
        (status = 200, description = "Issue found", body = IssueMessage),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Issue not found", body = ErrorBody),
        (status = 500, description = "Issue lookup unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_issue_handler<Store>(
    Path(id): Path<String>,
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
) -> Response
where
    Store: midlang_core::store::KVRead + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "diagnostic:read") {
        return response;
    }

    match translation.issue(&id).await {
        Ok(Some(record)) => (StatusCode::OK, Json(IssueMessage::from(record))).into_response(),
        Ok(None) => error::not_found(format!("issue '{id}' does not exist")),
        Err(err) => error::diagnostics(err),
    }
}

#[utoipa::path(
    patch,
    path = "/issues/{id}",
    params(
        ("id" = String, Path, description = "Issue identifier")
    ),
    request_body = UpdateIssueRequest,
    responses(
        (status = 200, description = "Issue state updated", body = IssueMessage),
        (status = 400, description = "Invalid state", body = ErrorBody),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Issue not found", body = ErrorBody),
        (status = 500, description = "Issue update unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_issue_handler<Store>(
    Path(id): Path<String>,
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
    Json(request): Json<UpdateIssueRequest>,
) -> Response
where
    Store: midlang_core::store::KVRead + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "diagnostic:manage") {
        return response;
    }

    let state = match IssueState::parse(&request.state) {
        Some(state) => state,
        None => return error::bad_request("state must be one of open, closed or ignored"),
    };

    match translation.set_issue_state(&id, state).await {
        Ok(true) => match translation.issue(&id).await {
            Ok(Some(record)) => (StatusCode::OK, Json(IssueMessage::from(record))).into_response(),
            Ok(None) => error::not_found(format!("issue '{id}' does not exist")),
            Err(err) => error::diagnostics(err),
        },
        Ok(false) => error::not_found(format!("issue '{id}' does not exist")),
        Err(err) => error::diagnostics(err),
    }
}
