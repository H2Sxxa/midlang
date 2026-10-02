use std::net::SocketAddr;

pub mod auth;
pub mod health;
pub mod openapi;
pub mod store;
pub mod translate;

use anyhow::Result;
use axum::http::HeaderValue;
use axum::serve::serve;
use axum::{Router, middleware::from_fn_with_state, routing::any};
use midlang_core::{store::KVStore, translation::Translation};
use tokio::net::TcpListener;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use utoipa_swagger_ui::SwaggerUi;

use crate::secure::{self, AuthStore};

/// Builds the HTTP API router without binding a listener.
///
/// The returned router can be served by any Axum-compatible transport or
/// invoked directly in tests and embedded applications.
pub fn router<Store>(
    translation: Translation<Store>,
    auth_store: AuthStore,
    cors_origins: &[String],
) -> Result<Router>
where
    Store: KVStore + Clone + Send + Sync + 'static,
{
    let protected = auth::router::<Translation<Store>>()
        .merge(store::router::<Store>())
        .merge(translate::router::<Store>())
        .layer(from_fn_with_state(auth_store, secure::middleware));

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
        .route("/health", any(health::health))
        .merge(SwaggerUi::new("/docs").url("/openapi.json", openapi::ApiDoc::document()))
        .merge(protected)
        .with_state(translation)
        .layer(cors))
}

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

    /// Builds the HTTP router without binding a listener.
    ///
    /// Callers can use the returned router with any Axum-compatible
    /// transport, or invoke it directly in tests and embedded applications.
    pub fn router(&self) -> Result<axum::Router> {
        router(
            self.translation.clone(),
            self.auth.clone(),
            &self.cors_origins,
        )
    }

    pub async fn serve(&self) -> Result<()> {
        let router = self.router()?;
        let listener = TcpListener::bind(self.addr).await?;
        serve(listener, router).await?;
        Ok(())
    }
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
    }
}
