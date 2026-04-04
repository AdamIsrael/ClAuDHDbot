pub mod schedules;
pub mod tasks;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

use crate::config::DatabaseConfig;

pub async fn init(config: &DatabaseConfig) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(&config.url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
