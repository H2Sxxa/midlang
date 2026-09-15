pub mod internals;
pub mod store;
pub mod translation;

#[cfg(test)]
mod test {
    use std::{sync::Arc, time::Duration};

    use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};
    use tokio::time::sleep;

    use crate::translation;

    #[tokio::test]
    async fn test_translation() {
        Sqlite::create_database("sqlite.db").await.unwrap();
        let pool = SqlitePool::connect("sqlite.db").await.unwrap();

        let internal = Arc::new(
            crate::internals::InternalService::default()
                .issue_collector(&pool)
                .await
                .unwrap()
                .changelog_recorder(&pool)
                .await
                .unwrap()
                .service(),
        );

        let mut translation = translation::Translation::new(
            crate::store::RedbStore::new("translation.rdb").unwrap(),
            std::num::NonZeroUsize::new(100).unwrap(),
            internal.clone(),
        );

        let value = translation.get_key("zh-cn", "test").await.unwrap();
        println!("value: {:?}", value);

        sleep(Duration::from_secs(20)).await;
    }
}
