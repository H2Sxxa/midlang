use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Extension, State},
    response::{IntoResponse, Response},
};
use midlang_core::translation::Translation;
use serde::Serialize;

use crate::{
    protocol::error::ErrorBody,
    secure::{self, AuthContext},
};

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StoreStatistics {
    pub locales: usize,
    pub entries: usize,
    pub per_locale: BTreeMap<String, usize>,
}

#[utoipa::path(
    get,
    path = "/store/statistics",
    responses(
        (status = 200, description = "Current translation store statistics", body = StoreStatistics),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 500, description = "Store statistics unavailable", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn statistics_handler<Store>(
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
) -> Response
where
    Store: midlang_core::store::KVRead + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "translation:list") {
        return response;
    }

    match translation.statistics() {
        Ok(statistics) => Json(StoreStatistics {
            locales: statistics.locales,
            entries: statistics.entries,
            per_locale: statistics.per_locale,
        })
        .into_response(),
        Err(error) => crate::protocol::error::store(error),
    }
}
