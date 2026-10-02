use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use midlang_core::{
    query::pagination::DEFAULT_PAGE_SIZE,
    store::{KVCursor, KVRead, SortOrder},
    translation::Translation,
};

use crate::{
    protocol::{error, error::ErrorBody},
    secure::{self, AuthContext},
};

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TranslationListQuery {
    /// Case-insensitive substring matched against keys and values.
    pub keyword: Option<String>,
    /// Number of entries to return. Defaults to 100.
    pub limit: Option<usize>,
    /// Cursor returned by the previous page.
    pub cursor: Option<String>,
    /// Sort direction for the first page. A cursor carries its own direction.
    pub order: Option<String>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TranslationListEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TranslationListPage {
    pub items: Vec<TranslationListEntry>,
    /// URL-safe cursor token for the next page, if one exists.
    pub next: Option<String>,
}

#[utoipa::path(
    get,
    path = "/t/{locale}",
    params(
        ("locale" = String, Path, description = "Locale identifier"),
        TranslationListQuery
    ),
    responses(
        (status = 200, description = "Paged translations", body = TranslationListPage),
        (status = 400, description = "Invalid query or cursor", body = ErrorBody),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Locale not found", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_translation_handler<Store>(
    Path(locale): Path<String>,
    Query(query): Query<TranslationListQuery>,
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
) -> Response
where
    Store: KVRead + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "translation:list") {
        return response;
    }

    let order = match query
        .order
        .as_deref()
        .map(|order| order.to_ascii_lowercase())
    {
        Some(order) if order == "asc" => SortOrder::Asc,
        Some(order) if order == "desc" => SortOrder::Desc,
        Some(_) => return error::bad_request("order must be either ASC or DESC"),
        None => SortOrder::Asc,
    };

    let cursor = match query.cursor {
        Some(token) => match KVCursor::decode(&token) {
            Ok(cursor) => cursor,
            Err(err) => return error::bad_request(format!("invalid cursor: {err}")),
        },
        None => KVCursor::new(order, query.keyword.clone()),
    };
    let limit = query.limit.unwrap_or(DEFAULT_PAGE_SIZE);

    match translation.list(&locale, &cursor, limit) {
        Ok(page) => {
            let next = match page.next {
                Some(cursor) => match cursor.encode() {
                    Ok(token) => Some(token),
                    Err(err) => return error::store(err),
                },
                None => None,
            };
            (
                StatusCode::OK,
                Json(TranslationListPage {
                    items: page
                        .items
                        .into_iter()
                        .map(|entry| TranslationListEntry {
                            key: entry.key,
                            value: entry.value,
                        })
                        .collect(),
                    next,
                }),
            )
                .into_response()
        }
        Err(err) => error::store(err),
    }
}
