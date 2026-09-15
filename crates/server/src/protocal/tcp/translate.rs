use axum::{
    Json,
    extract::{Path, State},
};
use midlang_core::{store::KVStore, translation::Translation};

pub async fn translate_handler<Store: KVStore>(
    Path((locale, key)): Path<(String, String)>,
    state: State<Translation<Store>>,
) -> Json<Option<String>> {
    let translation = state.0;
    let result = translation.get_key(&locale, &key).await;
    match result {
        Ok(value) => Json(value),
        Err(_) => Json(None),
    }
}

pub async fn translate_namespace_handler<Store: KVStore>(
    Path((locale, namespace, key)): Path<(String, String, String)>,
    state: State<Translation<Store>>,
) -> Json<Option<String>> {
    let translation = state.0;
    let result = translation.get(&locale, &namespace, &key).await;
    match result {
        Ok(value) => Json(value),
        Err(_) => Json(None),
    }
}
