use chrono::{DateTime, Utc};
use sqlx::{Error, Pool, Sqlite};

use crate::app::{error::AppError, models::File};

pub async fn create(
    db: &Pool<Sqlite>,
    key: &str,
    user_id: i64,
    content_type: &str,
) -> Result<File, Error> {
    sqlx::query_as!(
        File,
        "INSERT INTO files(key, content_type, user_id, updated_at, created_at, ai_flagged_at, human_reviewed_at) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id as \"id!\", key, content_type, user_id, created_at as \"created_at: DateTime<Utc>\", updated_at as \"updated_at: DateTime<Utc>\", ai_flagged_at as \"ai_flagged_at: DateTime<Utc>\", human_reviewed_at as \"human_reviewed_at: DateTime<Utc>\"",
        key,
        content_type,
        user_id,
        Utc::now(),
        Utc::now(),
        Utc::now(),
        None::<DateTime<Utc>>,
    )
    .fetch_one(db)
    .await
}

pub async fn list(db: &Pool<Sqlite>, user_id: i64) -> Result<Vec<File>, Error> {
    sqlx::query_as!(
        File,
        "SELECT id, key, content_type, user_id, created_at as \"created_at: DateTime<Utc>\", updated_at as \"updated_at: DateTime<Utc>\", ai_flagged_at as \"ai_flagged_at: DateTime<Utc>\", human_reviewed_at as \"human_reviewed_at: DateTime<Utc>\" FROM files WHERE user_id = ?",
        user_id
    )
    .fetch_all(db)
    .await
}

pub async fn list_where_flagged(db: &Pool<Sqlite>) -> Result<Vec<File>, Error> {
    sqlx::query_as!(
        File,
        "SELECT id, key, content_type, user_id, created_at as \"created_at: DateTime<Utc>\", updated_at as \"updated_at: DateTime<Utc>\", ai_flagged_at as \"ai_flagged_at: DateTime<Utc>\", human_reviewed_at as \"human_reviewed_at: DateTime<Utc>\"
           FROM files
           WHERE ai_flagged_at IS NOT NULL
             AND human_reviewed_at IS NULL",
    )
    .fetch_all(db)
    .await
}
pub async fn delete_by_id_and_user_id(
    db: &Pool<Sqlite>,
    id: i64,
    user_id: i64,
) -> Result<File, AppError> {
    sqlx::query_as!(
        File,
        "SELECT id, key, content_type, user_id, created_at as \"created_at: DateTime<Utc>\", updated_at as \"updated_at: DateTime<Utc>\", ai_flagged_at as \"ai_flagged_at: DateTime<Utc>\", human_reviewed_at as \"human_reviewed_at: DateTime<Utc>\" FROM files WHERE user_id = ? AND id = ?",
        user_id,
        id
    )
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)
}

pub async fn delete(db: &Pool<Sqlite>, id: i64) -> Result<(), AppError> {
    let result = sqlx::query!("DELETE FROM files WHERE id = ?", id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

pub async fn find(db: &Pool<Sqlite>, id: i64) -> Result<File, AppError> {
    sqlx::query_as!(File, "SELECT id, key, content_type, user_id, created_at as \"created_at: DateTime<Utc>\", updated_at as \"updated_at: DateTime<Utc>\", ai_flagged_at as \"ai_flagged_at: DateTime<Utc>\", human_reviewed_at as \"human_reviewed_at: DateTime<Utc>\"  FROM files WHERE id = ?", id)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound)
}
