use std::net::SocketAddr;
pub mod health;
pub mod translate;
use anyhow::Result;
use axum::{
    Router,
    routing::{any, get},
    serve::serve,
};
use midlang_core::{store::KVStore, translation::Translation};
use tokio::net::TcpListener;
pub struct TCPProtocalServer<Store: KVStore> {
    translation: Translation<Store>,
    addr: SocketAddr,
}

impl<Store> TCPProtocalServer<Store>
where
    Store: KVStore + Clone + Send + Sync + 'static,
{
    pub fn new(translation: Translation<Store>, addr: SocketAddr) -> Self {
        TCPProtocalServer { translation, addr }
    }

    pub async fn translate(&self, locale: &str, key: &str) -> Result<Option<String>> {
        self.translation.get_key(locale, key).await
    }

    pub async fn serve(&self) -> Result<()> {
        let router = Router::new()
            .route("/health", any(health::health))
            .route(
                "/t/{locale}/{key}",
                get(translate::translate_handler::<Store>),
            )
            .route(
                "/t/{locale}/{namespace}/{key}",
                get(translate::translate_namespace_handler::<Store>),
            )
            .with_state(self.translation.clone());

        let listener = TcpListener::bind(self.addr).await?;
        serve(listener, router).await?;
        Ok(())
    }
}
