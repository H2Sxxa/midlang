use anyhow::Result;
use axum::http::HeaderValue;
use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{any, get},
};
use midlang_core::{store::KVStore, translation::Translation};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

use crate::secure::{self, AuthStore};

/// Builds the HTTP API router without binding a listener.
///
/// The returned router can be served by any Axum-compatible transport or
/// invoked directly in tests and embedded applications.
pub fn router<Store>(
    translation: Translation<Store>,
    auth: AuthStore,
    cors_origins: &[String],
) -> Result<Router>
where
    Store: KVStore + Clone + Send + Sync + 'static,
{
    let protected = Router::new()
        .route(
            "/auth/permissions",
            get(super::http::permissions::permissions_handler),
        )
        .route(
            "/store/statistics",
            get(super::http::store::statistics_handler::<Store>),
        )
        .route(
            "/t/{locale}",
            get(super::http::translate::list_translation_handler::<Store>),
        )
        .route(
            "/t/{locale}/{key}",
            get(super::http::translate::translate_handler::<Store>)
                .put(super::http::translate::set_translation_handler::<Store>)
                .delete(super::http::translate::delete_translation_handler::<Store>),
        )
        .route(
            "/t/{locale}/{namespace}/{key}",
            get(super::http::translate::translate_namespace_handler::<Store>)
                .put(super::http::translate::set_namespace_translation_handler::<Store>)
                .delete(super::http::translate::delete_namespace_translation_handler::<Store>),
        )
        .layer(from_fn_with_state(auth, secure::middleware));

    let cors = if cors_origins.is_empty() {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origins = cors_origins
            .iter()
            .map(HeaderValue::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(Any)
            .allow_headers(Any)
    };

    Ok(Router::new()
        .route("/health", any(super::http::health::health))
        .route("/openapi.json", get(super::http::openapi::handler))
        .merge(protected)
        .with_state(translation)
        .layer(cors))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use midlang_core::{
        internals::{InternalService, ServiceOptions},
        store::MemStore,
        translation::Translation,
    };
    use sqlx::SqlitePool;

    use crate::secure::AuthStore;

    #[tokio::test]
    async fn build_memorystore_service() {
        let store = MemStore::new();
        let sql = "sqlite::memory:";
        let pool = SqlitePool::connect(sql).await.unwrap();
        let internals = Arc::new(
            InternalService::conn_with_pool(
                pool,
                ServiceOptions {
                    issue: true,
                    changelog: true,
                    coverage: true,
                },
            )
            .await
            .unwrap(),
        );
        let translation = Translation::new(store, internals);
        let auth = AuthStore::connect(sql).await.unwrap();
        let _router = super::router(translation, auth, &[]).unwrap();
        // router.oneshot("/health").await.unwrap();
    }
}
