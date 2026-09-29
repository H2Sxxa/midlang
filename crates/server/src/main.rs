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

    let mut store = match arg.store {
        StoreType::Redb => RedbStore::new(arg.store_url.unwrap_or("translation.rdb".to_string()))?,
    };
    let internal = Arc::new(
        InternalService::conn(
            &arg.sqlite_url.unwrap_or("sqlite.db".to_string()),
            arg.issue,
            arg.changelog,
            arg.coverage,
        )
        .await?
        .service(),
    );

    // The observer has to be attached before the store serves traffic, because
    // coverage only learns about writes that happen while it is listening.
    if let Some(coverage) = &internal.coverage {
        store.attach_observer(internal.clone());
        coverage.resync(&store).await?;
    }

    let translation = Translation::new(store, internal);
    let server = TCPProtocalServer::new(
        translation,
        SocketAddr::new("127.0.0.1".parse().unwrap(), arg.port),
    );
    server.serve().await?;
    Ok(())
}
