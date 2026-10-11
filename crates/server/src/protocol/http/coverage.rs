use std::collections::BTreeMap;

use axum::{
    Json, Router,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use midlang_core::{
    coverage::{CoverageReport, LocaleCoverage},
    query::pagination::DEFAULT_PAGE_SIZE,
    translation::Translation,
};

use crate::{
    protocol::error::{self, ErrorBody},
    secure::{self, AuthContext},
};

/// One locale's coverage measured against the reference locale.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct LocaleCoverageMessage {
    /// Reference keys this locale translates with a non-empty value.
    pub present: usize,
    /// `reference_keys - present`.
    pub missing: usize,
    /// `present` over the report's `reference_keys`; `1.0` when the reference
    /// translates no keys.
    pub ratio: f64,
}

/// How much of the reference locale every other locale translates.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct CoverageMessage {
    /// The locale that defines the key universe.
    pub reference: String,
    /// Keys the reference locale translates.
    pub reference_keys: usize,
    /// `Σpresent / (locales × reference_keys)`; `1.0` when there is nothing to
    /// translate.
    pub overall: f64,
    /// Coverage per locale; the reference itself is not a row.
    pub per_locale: BTreeMap<String, LocaleCoverageMessage>,
}

/// One page of the reference keys a locale does not translate.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct MissingKeysPage {
    /// Untranslated keys, in ascending key order.
    pub items: Vec<String>,
    /// Key to resume after on the next page, if one exists.
    pub next: Option<String>,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MissingKeysQuery {
    /// Number of keys to return. Defaults to 100.
    pub limit: Option<usize>,
    /// Key returned as `next` by the previous page.
    pub cursor: Option<String>,
}

/// The locales coverage can measure.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct CoverageLocales {
    /// Locales coverage knows about, in ascending order.
    pub locales: Vec<String>,
}

impl From<LocaleCoverage> for LocaleCoverageMessage {
    fn from(coverage: LocaleCoverage) -> Self {
        LocaleCoverageMessage {
            present: coverage.present,
            missing: coverage.missing,
            ratio: coverage.ratio,
        }
    }
}

impl From<CoverageReport> for CoverageMessage {
    fn from(report: CoverageReport) -> Self {
        CoverageMessage {
            reference: report.reference,
            reference_keys: report.reference_keys,
            overall: report.overall,
            per_locale: report
                .per_locale
                .into_iter()
                .map(|(locale, coverage)| (locale, LocaleCoverageMessage::from(coverage)))
                .collect(),
        }
    }
}

pub fn router<Store>() -> Router<Translation<Store>>
where
    Store: midlang_core::store::KVRead + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/coverage/locales", get(locales_handler::<Store>))
        .route("/coverage/{reference}", get(coverage_handler::<Store>))
        .route(
            "/coverage/{reference}/{locale}/missing",
            get(missing_keys_handler::<Store>),
        )
}

#[utoipa::path(
    get,
    path = "/coverage/locales",
    responses(
        (status = 200, description = "Locales coverage knows about", body = CoverageLocales),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 500, description = "Coverage unavailable", body = ErrorBody),
        (status = 503, description = "Coverage tracking is not enabled", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn locales_handler<Store>(
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
) -> Response
where
    Store: midlang_core::store::KVRead + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "diagnostic:read") {
        return response;
    }

    match translation.coverage_locales().await {
        Ok(locales) => (StatusCode::OK, Json(CoverageLocales { locales })).into_response(),
        Err(err) => error::coverage(err),
    }
}

#[utoipa::path(
    get,
    path = "/coverage/{reference}",
    params(
        ("reference" = String, Path, description = "Locale that defines the key universe")
    ),
    responses(
        (status = 200, description = "Coverage of every other locale against the reference", body = CoverageMessage),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Reference locale not found", body = ErrorBody),
        (status = 500, description = "Coverage unavailable", body = ErrorBody),
        (status = 503, description = "Coverage tracking is not enabled", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn coverage_handler<Store>(
    Path(reference): Path<String>,
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
) -> Response
where
    Store: midlang_core::store::KVRead + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "diagnostic:read") {
        return response;
    }

    match translation.coverage(&reference).await {
        Ok(report) => (StatusCode::OK, Json(CoverageMessage::from(report))).into_response(),
        Err(err) => error::coverage(err),
    }
}

#[utoipa::path(
    get,
    path = "/coverage/{reference}/{locale}/missing",
    params(
        ("reference" = String, Path, description = "Locale that defines the key universe"),
        ("locale" = String, Path, description = "Locale the untranslated keys are listed for"),
        MissingKeysQuery
    ),
    responses(
        (status = 200, description = "Paged untranslated keys", body = MissingKeysPage),
        (status = 400, description = "Invalid query", body = ErrorBody),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Reference locale not found", body = ErrorBody),
        (status = 500, description = "Coverage unavailable", body = ErrorBody),
        (status = 503, description = "Coverage tracking is not enabled", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn missing_keys_handler<Store>(
    Path((reference, locale)): Path<(String, String)>,
    Query(query): Query<MissingKeysQuery>,
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
) -> Response
where
    Store: midlang_core::store::KVRead + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "diagnostic:read") {
        return response;
    }

    let limit = query.limit.unwrap_or(DEFAULT_PAGE_SIZE);
    match translation
        .missing_keys(&reference, &locale, query.cursor.as_deref(), limit)
        .await
    {
        Ok(page) => (
            StatusCode::OK,
            Json(MissingKeysPage {
                items: page.items,
                next: page.next,
            }),
        )
            .into_response(),
        Err(err) => error::coverage(err),
    }
}
