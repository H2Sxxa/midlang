use std::net::SocketAddr;

pub mod health;
pub mod openapi;
pub mod store;
pub mod translate;

use anyhow::Result;
use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{any, get},
    serve::serve,
};
use midlang_core::{store::KVStore, translation::Translation};
use tokio::net::TcpListener;

use crate::secure::{self, AuthStore};

/// Axum HTTP server. The transport is TCP, but the protocol exposed to clients
/// is HTTP, so the type is named after the protocol rather than the socket.
pub struct HttpServer<Store: KVStore> {
    translation: Translation<Store>,
    auth: AuthStore,
    addr: SocketAddr,
}

impl<Store> HttpServer<Store>
where
    Store: KVStore + Clone + Send + Sync + 'static,
{
    pub fn new(translation: Translation<Store>, auth: AuthStore, addr: SocketAddr) -> Self {
        Self {
            translation,
            auth,
            addr,
        }
    }

    pub async fn translate(&self, locale: &str, key: &str) -> Result<Option<String>> {
        self.translation.get_key(locale, key).await
    }

    pub async fn serve(&self) -> Result<()> {
        let protected = Router::new()
            .route("/store/statistics", get(store::statistics_handler::<Store>))
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

        let router = Router::new()
            .route("/health", any(health::health))
            .route("/openapi.json", get(openapi::handler))
            .merge(protected)
            .with_state(self.translation.clone());

        let listener = TcpListener::bind(self.addr).await?;
        serve(listener, router).await?;
        Ok(())
    }
}
