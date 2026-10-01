use std::net::SocketAddr;

pub mod health;
pub mod openapi;
pub mod store;
pub mod translate;

use anyhow::Result;
use axum::serve::serve;
use midlang_core::{store::KVStore, translation::Translation};
use tokio::net::TcpListener;

use crate::secure::AuthStore;

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
        super::router::router(
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
