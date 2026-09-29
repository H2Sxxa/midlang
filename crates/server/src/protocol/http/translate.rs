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
    protocol::error,
    secure::{self, AuthContext},
};

#[derive(Debug, serde::Deserialize)]
pub struct SetTranslationRequest {
    pub value: String,
}

#[derive(Debug, serde::Serialize)]
pub struct TranslationMessage {
    pub locale: String,
    pub key: String,
    pub value: String,
}

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
