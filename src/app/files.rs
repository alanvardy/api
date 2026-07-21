use crate::app::error::AppError;
use aws_config::SdkConfig;
use aws_sdk_s3::primitives::ByteStream;
use chrono::Utc;

// Uploads a file to Tigris and returns the key
pub async fn upload(
    aws_config: &SdkConfig,
    bucket: &str,
    filename: &str,
    bytes: Vec<u8>,
    content_type: &str,
) -> Result<String, AppError> {
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
        .map_err(|err| {
            tracing::error!(error = %err, "failed to upload object to s3");
            AppError::Storage
        })
}

// Deletes a file from Tigris by key
pub async fn delete(aws_config: &SdkConfig, bucket: &str, key: &str) -> Result<(), AppError> {
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
