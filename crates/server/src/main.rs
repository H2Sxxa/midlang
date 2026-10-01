use std::{net::SocketAddr, sync::Arc};

use crate::cli::StoreType;
use anyhow::Result;
use clap::Parser;
use midlang_core::store::{KVStore, MemStore, RedbStore};
use midlang_core::{
    internals::{InternalService, ServiceOptions},
    translation::Translation,
};
use midlang_server::protocol::http::HttpServer;
use midlang_server::secure::AuthStore;
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase, sqlite::SqlitePoolOptions};
pub mod cli;

#[tokio::main]
pub async fn main() {
    create_service().await.unwrap();
}

pub async fn create_service() -> Result<()> {
    use cli::Args;
    let arg = Args::parse();

    let store: Arc<dyn KVStore> = match arg.store {
        StoreType::Redb => Arc::new(RedbStore::new(
            arg.store_url.unwrap_or("translation.rdb".to_string()),
        )?),
        StoreType::Memory => Arc::new(MemStore::new()),
    };
    let sqlite_url = arg
        .sqlite_url
        .clone()
        .or_else(|| arg.auth_sqlite_url.clone())
        .unwrap_or("sqlite.db".to_string());
    if !sqlite_url.starts_with("sqlite://") && !std::path::Path::new(&sqlite_url).exists() {
        Sqlite::create_database(&sqlite_url).await?;
    }
    let pool: SqlitePool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&sqlite_url)
        .await?;
    let auth = AuthStore::from_pool(pool.clone()).await?;
    if let Some(token) = auth.bootstrap_admin_token().await? {
        eprintln!(
            "Created initial admin token. Store it securely; it will not be shown again:\n{}",
            token.token
        );
    }
    let internal = Arc::new(
        InternalService::conn_with_pool(
            pool,
            ServiceOptions {
                issue: arg.issue,
                changelog: arg.changelog,
                coverage: arg.coverage,
            },
        )
        .await?
        .service(),
    );

    // The observer has to be attached before the store serves traffic, because
    // coverage only learns about writes that happen while it is listening.
    if let Some(coverage) = &internal.coverage {
        store.add_observer(internal.clone());
        coverage.resync(&store).await?;
    }

    let translation = Translation::new(store, internal);
    let server = HttpServer::new(
        translation,
        auth,
        SocketAddr::new("127.0.0.1".parse().unwrap(), arg.port),
        arg.cors_origin,
    );
    server.serve().await?;
    Ok(())
}
