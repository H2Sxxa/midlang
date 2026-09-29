use anyhow::Result;
use sqlx::SqlitePool;

/// Shared dictionaries used by internal services.
pub async fn ensure_tables(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS midlang_locales (
            id     INTEGER PRIMARY KEY,
            locale TEXT NOT NULL UNIQUE
        )
        ",
    )
    .execute(pool)
    .await?;
    Ok(())
}
