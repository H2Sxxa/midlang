use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use midlang_core::{
    store::{KVRead, StoreError},
    translation::Translation,
};

use crate::protocal::error;

pub async fn translate_handler<Store>(
    Path((locale, key)): Path<(String, String)>,
    state: State<Translation<Store>>,
) -> Response
where
    Store: KVRead + Clone + Send + Sync + 'static,
{
    match state.0.get_key(&locale, &key).await {
        Ok(Some(value)) => (StatusCode::OK, Json(value)).into_response(),
        Ok(None) => error::store(StoreError::LocaleKeyNotExist(locale, key).into()),
        Err(err) => error::store(err),
    }
}

pub async fn translate_namespace_handler<Store>(
    Path((locale, namespace, key)): Path<(String, String, String)>,
    state: State<Translation<Store>>,
) -> Response
where
    Store: KVRead + Clone + Send + Sync + 'static,
{
    match state.0.get(&locale, &namespace, &key).await {
        Ok(Some(value)) => (StatusCode::OK, Json(value)).into_response(),
        Ok(None) => {
            error::store(StoreError::LocaleKeyNotExist(locale, format!("{namespace}.{key}")).into())
        }
        Err(err) => error::store(err),
    }
}
