use aws_sdk_s3::primitives::ByteStream;
use axum::{Json, extract::State, http::StatusCode};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

use crate::{error::AppError, state::AppState};

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

pub async fn upload(
    State(state): State<AppState>,
    Json(payload): Json<UploadImage>,
) -> Result<(StatusCode, Json<UploadResponse>), AppError> {
    let bytes = STANDARD
        .decode(payload.data.as_bytes())
        .map_err(|_| AppError::BadRequest("invalid base64 data"))?;

    let client = aws_sdk_s3::Client::new(&state.env.aws_config);

    // Prefix the key with a timestamp so repeated uploads of the same
    // filename do not overwrite each other.
    let key = format!(
        "uploads/{}-{}",
        chrono::Utc::now().timestamp_millis(),
        payload.filename
    );

    client
        .put_object()
        .bucket(&state.env.s3_bucket)
        .key(&key)
        .body(ByteStream::from(bytes))
        .content_type(payload.content_type)
        .send()
        .await
        .map_err(|err| {
            tracing::error!(error = %err, "failed to upload object to s3");
            AppError::Storage
        })?;

    Ok((StatusCode::CREATED, Json(UploadResponse { key })))
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
