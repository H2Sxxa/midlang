use std::{net::SocketAddr, sync::Arc};

use crate::cli::StoreType;
use anyhow::Result;
use clap::Parser;
use midlang_core::{internals::InternalService, store::RedbStore, translation::Translation};
use midlang_server::protocal::tcp::TCPProtocalServer;
pub mod cli;
pub mod protocal;

#[tokio::main]
pub async fn main() {
    create_service().await.unwrap();
}

pub async fn create_service() -> Result<()> {
    use cli::Args;
    let arg = Args::parse();

    let store = match arg.store {
        StoreType::Redb => RedbStore::new(arg.store_url.unwrap_or("translation.rdb".to_string()))?,
    };
    let internal = InternalService::conn(
        &arg.sqlite_url.unwrap_or("sqlite.db".to_string()),
        arg.issue,
        arg.changelog,
    )
    .await?
    .service();
    let translation = Translation::new(store, arg.cache_capacity, Arc::new(internal));
    let server = TCPProtocalServer::new(
        translation,
        SocketAddr::new("127.0.0.1".parse().unwrap(), arg.port),
    );
    server.serve().await?;
    Ok(())
}
