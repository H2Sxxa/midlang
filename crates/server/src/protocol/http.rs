use std::net::SocketAddr;

pub mod health;
pub mod openapi;
pub mod store;
pub mod translate;

use anyhow::Result;
use axum::http::HeaderValue;
use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{any, get},
    serve::serve,
};
use midlang_core::{store::KVStore, translation::Translation};
use tokio::net::TcpListener;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

use crate::secure::{self, AuthStore};

/// Axum HTTP server. The transport is TCP, but the protocol exposed to clients
/// is HTTP, so the type is named after the protocol rather than the socket.
pub struct HttpServer<Store: KVStore> {
    translation: Translation<Store>,
    auth: AuthStore,
    addr: SocketAddr,
    cors_origins: Vec<String>,
}

impl<Store> HttpServer<Store>
where
    Store: KVStore + Clone + Send + Sync + 'static,
{
    pub fn new(
        translation: Translation<Store>,
        auth: AuthStore,
        addr: SocketAddr,
        cors_origins: Vec<String>,
    ) -> Self {
        Self {
            translation,
            auth,
            addr,
            cors_origins,
        }
    }

    pub async fn translate(&self, locale: &str, key: &str) -> Result<Option<String>> {
        self.translation.get_key(locale, key).await
    }

    pub async fn serve(&self) -> Result<()> {
        let protected = Router::new()
            .route("/store/statistics", get(store::statistics_handler::<Store>))
            .route(
                "/t/{locale}",
                get(translate::list_translation_handler::<Store>),
            )
            .route(
                "/t/{locale}/{key}",
                get(translate::translate_handler::<Store>)
                    .put(translate::set_translation_handler::<Store>)
                    .delete(translate::delete_translation_handler::<Store>),
            )
            .route(
                "/t/{locale}/{namespace}/{key}",
                get(translate::translate_namespace_handler::<Store>)
                    .put(translate::set_namespace_translation_handler::<Store>)
                    .delete(translate::delete_namespace_translation_handler::<Store>),
            )
            .layer(from_fn_with_state(self.auth.clone(), secure::middleware));

        let cors = if self.cors_origins.is_empty() {
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        } else {
            let origins = self
                .cors_origins
                .iter()
                .map(HeaderValue::try_from)
                .collect::<Result<Vec<_>, _>>()?;
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods(Any)
                .allow_headers(Any)
        };

        let router = Router::new()
            .route("/health", any(health::health))
            .route("/openapi.json", get(openapi::handler))
            .merge(protected)
            .with_state(self.translation.clone())
            .layer(cors);

        let listener = TcpListener::bind(self.addr).await?;
        serve(listener, router).await?;
        Ok(())
    }
}
