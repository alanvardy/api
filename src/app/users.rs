use crate::app::models::User;
use chrono::{DateTime, Utc};
use sqlx::{Error, Pool, Sqlite, sqlite::SqliteQueryResult};

pub async fn create(db: &Pool<Sqlite>, name: &str, email: &str) -> Result<User, Error> {
    let now = Utc::now();
    sqlx::query_as!(
        User,
        r#"INSERT INTO users (name, email, created_at, updated_at) VALUES (?, ?, ?, ?) RETURNING id, name, email,
            created_at AS "created_at: DateTime<Utc>",
            updated_at AS "updated_at: DateTime<Utc>""#,
        name,
        email,
        now,
        now,
    )
    .fetch_one(db)
    .await
}

pub async fn list(db: &Pool<Sqlite>) -> Result<Vec<User>, Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, name, email,
            created_at AS "created_at: DateTime<Utc>",
            updated_at AS "updated_at: DateTime<Utc>"
        FROM users"#
    )
    .fetch_all(db)
    .await
}

pub async fn get_by_id(db: &Pool<Sqlite>, id: i64) -> Result<Option<User>, Error> {
    sqlx::query_as!(
        User,
        r#"SELECT id, name, email,
            created_at AS "created_at: DateTime<Utc>",
            updated_at AS "updated_at: DateTime<Utc>"
        FROM users WHERE id = ?"#,
        id
    )
    .fetch_optional(db)
    .await
}

pub async fn update(
    db: &Pool<Sqlite>,
    id: i64,
    name: &str,
    email: &str,
) -> Result<Option<User>, Error> {
    sqlx::query_as!(
        User,
        r#"UPDATE users
         SET name = ?, email = ?
         WHERE id = ?
         RETURNING id, name, email,
            created_at AS "created_at: DateTime<Utc>",
            updated_at AS "updated_at: DateTime<Utc>""#,
        name,
        email,
        id
    )
    .fetch_optional(db)
    .await
}

pub async fn delete(db: &Pool<Sqlite>, id: i64) -> Result<SqliteQueryResult, Error> {
    sqlx::query!("DELETE FROM users WHERE id = ?", id)
        .execute(db)
        .await
}
