pub mod key;
pub mod locale;
pub mod namespace;

use axum::{Router, routing::get};
use midlang_core::{store::KVStore, translation::Translation};

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

pub fn router<Store>() -> Router<Translation<Store>>
where
    Store: KVStore + Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/t/{locale}",
            get(locale::list_translation_handler::<Store>),
        )
        .route(
            "/t/{locale}/{key}",
            get(key::translate_handler::<Store>)
                .put(key::set_translation_handler::<Store>)
                .delete(key::delete_translation_handler::<Store>),
        )
        .route(
            "/t/{locale}/{namespace}/{key}",
            get(namespace::translate_namespace_handler::<Store>)
                .put(namespace::set_namespace_translation_handler::<Store>)
                .delete(namespace::delete_namespace_translation_handler::<Store>),
        )
}
