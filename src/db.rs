use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::str::FromStr;

pub async fn init_db() -> SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite:test.db")
        .unwrap()
        .create_if_missing(true);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .unwrap()
}

pub async fn create_table(pool: &SqlitePool) {
    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .unwrap();
}
