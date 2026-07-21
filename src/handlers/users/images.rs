use crate::models::File;
use crate::{error::AppError, state::AppState};
use aws_config::SdkConfig;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::delete_object::DeleteObjectError;
use aws_sdk_s3::operation::put_object::PutObjectError;
use aws_sdk_s3::primitives::ByteStream;
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

// Uploads a file to Tigris and returns the key
async fn upload_file(
    aws_config: &SdkConfig,
    bucket: &str,
    filename: &str,
    bytes: Vec<u8>,
    content_type: &str,
) -> Result<String, SdkError<PutObjectError>> {
    let client = aws_sdk_s3::Client::new(aws_config);

    // Prefix the key with a timestamp so repeated uploads of the same
    // filename do not overwrite each other.
    let key = format!("uploads/{}-{}", Utc::now().timestamp_millis(), filename);

    client
        .put_object()
        .bucket(bucket)
        .key(&key)
        .body(ByteStream::from(bytes))
        .content_type(content_type)
        .send()
        .await
        .map(|_| key)
}

// Deletes a file from Tigris by key
async fn delete_file(
    aws_config: &SdkConfig,
    bucket: &str,
    key: &str,
) -> Result<(), SdkError<DeleteObjectError>> {
    let client = aws_sdk_s3::Client::new(aws_config);

    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map(|_| ())
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

    let key = upload_file(&aws_config, &bucket, &filename, bytes, &content_type)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to upload object to s3");
            AppError::Storage
        })?;

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
        if let Err(delete_err) = delete_file(&aws_config, &bucket, &key).await {
            tracing::error!(error = %delete_err, "failed to delete orphaned object from s3");
        }
        return Err(err.into());
    }

    Ok((StatusCode::CREATED, Json(UploadResponse { key })))
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
