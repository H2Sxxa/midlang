use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use midlang_core::{
    store::{KVRead, KVStore, StoreError},
    translation::Translation,
};

use crate::{
    protocol::{error, error::ErrorBody},
    secure::{self, AuthContext},
};

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct SetTranslationRequest {
    pub value: String,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TranslationMessage {
    pub locale: String,
    pub key: String,
    pub value: String,
}

#[utoipa::path(
    get,
    path = "/t/{locale}/{key}",
    params(
        ("locale" = String, Path, description = "Locale identifier"),
        ("key" = String, Path, description = "Translation key")
    ),
    responses(
        (status = 200, description = "Translation found", body = TranslationMessage),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Translation not found", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn translate_handler<Store>(
    Path((locale, key)): Path<(String, String)>,
    Extension(auth): Extension<AuthContext>,
    state: State<Translation<Store>>,
) -> Response
where
    Store: KVRead + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "translation:read") {
        return response;
    }

    match state.0.get_key(&locale, &key).await {
        Ok(Some(value)) => (
            StatusCode::OK,
            Json(TranslationMessage { locale, key, value }),
        )
            .into_response(),
        Ok(None) => error::store(StoreError::LocaleKeyNotExist(locale, key).into()),
        Err(err) => error::store(err),
    }
}

#[utoipa::path(
    put,
    path = "/t/{locale}/{key}",
    params(
        ("locale" = String, Path, description = "Locale identifier"),
        ("key" = String, Path, description = "Translation key")
    ),
    request_body = SetTranslationRequest,
    responses(
        (status = 200, description = "Translation updated", body = TranslationMessage),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Translation not found", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn set_translation_handler<Store>(
    Path((locale, key)): Path<(String, String)>,
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
    Json(request): Json<SetTranslationRequest>,
) -> Response
where
    Store: KVStore + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "translation:write") {
        return response;
    }

    match translation.set(&locale, &key, &request.value).await {
        Ok(()) => (
            StatusCode::OK,
            Json(TranslationMessage {
                locale,
                key,
                value: request.value,
            }),
        )
            .into_response(),
        Err(err) => error::store(err.into()),
    }
}

#[utoipa::path(
    delete,
    path = "/t/{locale}/{key}",
    params(
        ("locale" = String, Path, description = "Locale identifier"),
        ("key" = String, Path, description = "Translation key")
    ),
    responses(
        (status = 204, description = "Translation deleted"),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Translation not found", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_translation_handler<Store>(
    Path((locale, key)): Path<(String, String)>,
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
) -> Response
where
    Store: KVStore + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "translation:delete") {
        return response;
    }

    match translation.delete(&locale, &key).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => error::store(err.into()),
    }
}

#[utoipa::path(
    get,
    path = "/t/{locale}/{namespace}/{key}",
    params(
        ("locale" = String, Path, description = "Locale identifier"),
        ("namespace" = String, Path, description = "Translation namespace"),
        ("key" = String, Path, description = "Translation key")
    ),
    responses(
        (status = 200, description = "Translation found", body = TranslationMessage),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Translation not found", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn translate_namespace_handler<Store>(
    Path((locale, namespace, key)): Path<(String, String, String)>,
    Extension(auth): Extension<AuthContext>,
    state: State<Translation<Store>>,
) -> Response
where
    Store: KVRead + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "translation:read") {
        return response;
    }

    match state.0.get(&locale, &namespace, &key).await {
        Ok(Some(value)) => (
            StatusCode::OK,
            Json(TranslationMessage {
                locale,
                key: format!("{namespace}.{key}"),
                value,
            }),
        )
            .into_response(),
        Ok(None) => {
            error::store(StoreError::LocaleKeyNotExist(locale, format!("{namespace}.{key}")).into())
        }
        Err(err) => error::store(err),
    }
}

#[utoipa::path(
    put,
    path = "/t/{locale}/{namespace}/{key}",
    params(
        ("locale" = String, Path, description = "Locale identifier"),
        ("namespace" = String, Path, description = "Translation namespace"),
        ("key" = String, Path, description = "Translation key")
    ),
    request_body = SetTranslationRequest,
    responses(
        (status = 200, description = "Translation updated", body = TranslationMessage),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Translation not found", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn set_namespace_translation_handler<Store>(
    Path((locale, namespace, key)): Path<(String, String, String)>,
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
    Json(request): Json<SetTranslationRequest>,
) -> Response
where
    Store: KVStore + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "translation:write") {
        return response;
    }

    let full_key = format!("{namespace}.{key}");
    match translation.set(&locale, &full_key, &request.value).await {
        Ok(()) => (
            StatusCode::OK,
            Json(TranslationMessage {
                locale,
                key: full_key,
                value: request.value,
            }),
        )
            .into_response(),
        Err(err) => error::store(err.into()),
    }
}

#[utoipa::path(
    delete,
    path = "/t/{locale}/{namespace}/{key}",
    params(
        ("locale" = String, Path, description = "Locale identifier"),
        ("namespace" = String, Path, description = "Translation namespace"),
        ("key" = String, Path, description = "Translation key")
    ),
    responses(
        (status = 204, description = "Translation deleted"),
        (status = 401, description = "Missing or invalid token", body = ErrorBody),
        (status = 403, description = "Permission denied", body = ErrorBody),
        (status = 404, description = "Translation not found", body = ErrorBody)
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_namespace_translation_handler<Store>(
    Path((locale, namespace, key)): Path<(String, String, String)>,
    Extension(auth): Extension<AuthContext>,
    State(translation): State<Translation<Store>>,
) -> Response
where
    Store: KVStore + Clone + Send + Sync + 'static,
{
    if let Err(response) = secure::require_permission(&auth, "translation:delete") {
        return response;
    }

    let full_key = format!("{namespace}.{key}");
    match translation.delete(&locale, &full_key).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => error::store(err.into()),
    }
}
