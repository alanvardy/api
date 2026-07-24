use crate::app::error::AppError;
use aws_config::SdkConfig;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use chrono::Utc;
use std::time::Duration;

// Uploads a file to Tigris and returns the key
pub async fn upload(
    aws_config: &SdkConfig,
    bucket: &str,
    filename: &str,
    bytes: Vec<u8>,
    content_type: &str,
) -> Result<String, AppError> {
    // Prefix the key with a timestamp so repeated uploads of the same
    // filename do not overwrite each other.
    let key = format!("uploads/{}-{}", Utc::now().timestamp_millis(), filename);

    // Avoid hitting external storage during tests.
    if cfg!(test) {
        return Ok(key);
    }

    let client = aws_sdk_s3::Client::new(aws_config);

    client
        .put_object()
        .bucket(bucket)
        .key(&key)
        .body(ByteStream::from(bytes))
        .content_type(content_type)
        .send()
        .await
        .map(|_| key)
        .map_err(|err| {
            tracing::error!(error = %err, "failed to upload object to s3");
            AppError::Storage
        })
}

// Generates a presigned download URL for a key in the given bucket.
// The URL is valid for `expires_in` (max 7 days).
pub async fn presign_download(
    aws_config: &SdkConfig,
    bucket: &str,
    key: &str,
    expires_in: Duration,
) -> Result<String, AppError> {
    // Avoid hitting external credential resolution during tests.
    if cfg!(test) {
        return Ok(format!("https://fake-presigned/{key}"));
    }

    let client = aws_sdk_s3::Client::new(aws_config);

    let config = PresigningConfig::expires_in(expires_in).map_err(|err| {
        tracing::error!(error = %err, "invalid presigning duration");
        AppError::Storage
    })?;

    client
        .get_object()
        .bucket(bucket)
        .key(key)
        .presigned(config)
        .await
        .map(|req| req.uri().to_owned())
        .map_err(|err| {
            tracing::error!(error = %err, "failed to presign s3 get_object");
            AppError::Storage
        })
}

// Deletes a file from Tigris by key
pub async fn delete(aws_config: &SdkConfig, bucket: &str, key: &str) -> Result<(), AppError> {
    // Avoid hitting external storage during tests.
    if cfg!(test) {
        return Ok(());
    }

    let client = aws_sdk_s3::Client::new(aws_config);

    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map(|_| ())
        .map_err(|err| {
            tracing::error!(error = %err, "failed to delete object from s3");
            AppError::Storage
        })
}
