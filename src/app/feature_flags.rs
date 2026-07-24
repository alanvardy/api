use crate::app::models::FeatureFlag;
use chrono::{DateTime, Utc};
use sqlx::{Error, Pool, Sqlite, sqlite::SqliteQueryResult};

pub async fn list(db: &Pool<Sqlite>) -> Result<Vec<FeatureFlag>, Error> {
    sqlx::query_as!(
        FeatureFlag,
        r#"SELECT id, name, enabled,
            created_at AS "created_at: DateTime<Utc>",
            updated_at AS "updated_at: DateTime<Utc>"
           FROM feature_flags"#
    )
    .fetch_all(db)
    .await
}

pub async fn create(
    db: &Pool<Sqlite>,
    name: &str,
    enabled: bool,
) -> Result<SqliteQueryResult, Error> {
    let now = Utc::now();
    sqlx::query!(
        "INSERT INTO feature_flags (name, enabled, created_at, updated_at)
         VALUES (?, ?, ?, ?)",
        name,
        enabled,
        now,
        now
    )
    .execute(db)
    .await
}

pub async fn update(db: &Pool<Sqlite>, id: i64, enabled: bool) -> Result<SqliteQueryResult, Error> {
    let now = Utc::now();
    sqlx::query!(
        "UPDATE feature_flags
         SET enabled = ?, updated_at = ?
         WHERE id = ?",
        enabled,
        now,
        id
    )
    .execute(db)
    .await
}
