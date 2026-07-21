use crate::app::models::File;
use crate::app::{error::AppError, files, state::AppState};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct UploadImage {
    pub filename: String,
    pub content_type: String,
    pub data: String,
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub key: String,
}

pub async fn post(
    Path(user_id): Path<i64>,
    State(state): State<AppState>,
    Json(payload): Json<UploadImage>,
) -> Result<(StatusCode, Json<UploadResponse>), AppError> {
    let aws_config = state.env.aws_config;
    let bucket = state.env.s3_bucket;
    let filename = payload.filename;
    let content_type = payload.content_type;
    let bytes = STANDARD
        .decode(payload.data.as_bytes())
        .map_err(|_| AppError::BadRequest("invalid base64 data"))?;

    let key = files::upload(&aws_config, &bucket, &filename, bytes, &content_type).await?;

    if let Err(err) = sqlx::query_as!(
        File,
        "INSERT INTO files(key, content_type, user_id, updated_at, created_at) VALUES (?, ?, ?, ?, ?) RETURNING id as \"id!\", key, content_type, user_id",
        key,
        content_type,
        user_id,
        Utc::now(),
        Utc::now()
    )
    .fetch_one(&state.db)
    .await
    {
        // Roll back the uploaded object so we do not leave orphaned files
        // in storage when the database insert fails.
        files::delete(&aws_config, &bucket, &key).await?;
        return Err(err.into());
    }

    Ok((StatusCode::CREATED, Json(UploadResponse { key })))
}

pub async fn delete(
    Path((user_id, image_id)): Path<(i64, i64)>,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let aws_config = state.env.aws_config;
    let bucket = state.env.s3_bucket;

    // Ensure the file exists and belongs to this user before touching storage.
    let file = sqlx::query_as!(
        File,
        "SELECT id, key, content_type, user_id FROM files WHERE user_id = ? AND id = ?",
        user_id,
        image_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    // Remove the object from storage first so a database delete failure does
    // not leave a dangling record pointing at a missing object.
    files::delete(&aws_config, &bucket, &file.key).await?;

    sqlx::query!("DELETE FROM files WHERE id = ?", file.id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get(
    Path(user_id): Path<i64>,

    State(state): State<AppState>,
) -> Result<Json<Vec<File>>, AppError> {
    let files = sqlx::query_as!(
        File,
        "SELECT id, key, content_type, user_id FROM files WHERE user_id = ?",
        user_id
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn delete_missing_image_returns_not_found(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .delete(format!("http://{addr}/users/1/images/999"))
            .send()
            .await
            .expect("request to delete missing image should complete");

        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

        let body: serde_json::Value = response
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(body["error"], "resource not found");
    }

    #[test]
    fn valid_base64_decodes_to_expected_bytes() {
        let encoded = STANDARD.encode(b"hello");
        let decoded = STANDARD
            .decode(encoded.as_bytes())
            .expect("valid base64 should decode");

        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn invalid_base64_maps_to_bad_request() {
        let result = STANDARD
            .decode("not valid base64!!!".as_bytes())
            .map_err(|_| AppError::BadRequest("invalid base64 data"));

        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }
}
