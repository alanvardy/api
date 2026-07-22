use crate::app::models::{ContentType, File};
use crate::app::{error::AppError, files, state::AppState};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const ONE_DAY: Duration = Duration::from_secs(86400);

#[derive(Deserialize)]
pub struct UploadImage {
    pub filename: String,
    pub content_type: String,
    pub data: String,
}

#[derive(Serialize)]
pub struct FileResponse {
    pub id: i64,
    pub key: String,
    pub content_type: ContentType,
    pub user_id: i64,
    pub url: String,
}

pub async fn post(
    Path(user_id): Path<i64>,
    State(state): State<AppState>,
    Json(payload): Json<UploadImage>,
) -> Result<(StatusCode, Json<FileResponse>), AppError> {
    let aws_config = state.env.aws_config;
    let bucket = state.env.s3_bucket;
    let filename = payload.filename;
    let content_type = payload.content_type;
    let bytes = STANDARD
        .decode(payload.data.as_bytes())
        .map_err(|_| AppError::BadRequest("invalid base64 data"))?;

    let key = files::upload(&aws_config, &bucket, &filename, bytes, &content_type).await?;

    let file = match sqlx::query_as!(
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
        Ok(file) => file,
        Err(err) => {
            // Roll back the uploaded object so we do not leave orphaned files
            // in storage when the database insert fails.
            files::delete(&aws_config, &bucket, &key).await?;
            return Err(err.into());
        }
    };

    let url = files::presign_download(&aws_config, &bucket, &file.key, ONE_DAY).await?;

    Ok((
        StatusCode::CREATED,
        Json(FileResponse {
            id: file.id,
            key: file.key,
            content_type: file.content_type,
            user_id: file.user_id,
            url,
        }),
    ))
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
) -> Result<Json<Vec<FileResponse>>, AppError> {
    let files = sqlx::query_as!(
        File,
        "SELECT id, key, content_type, user_id FROM files WHERE user_id = ?",
        user_id
    )
    .fetch_all(&state.db)
    .await?;

    let aws_config = &state.env.aws_config;
    let bucket = &state.env.s3_bucket;

    let mut responses = Vec::with_capacity(files.len());
    for file in files {
        let url = files::presign_download(aws_config, bucket, &file.key, ONE_DAY).await?;
        responses.push(FileResponse {
            id: file.id,
            key: file.key,
            content_type: file.content_type,
            user_id: file.user_id,
            url,
        });
    }

    Ok(Json(responses))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;
    use reqwest::header;
    use sqlx::SqlitePool;

    fn bearer_header() -> header::HeaderMap {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {BEARER_TOKEN}")).unwrap(),
        );
        headers
    }

    // Creates a user via the API and returns its id, satisfying the
    // files.user_id foreign key before exercising image endpoints.
    async fn create_user(client: &reqwest::Client, addr: &std::net::SocketAddr) -> i64 {
        let response = client
            .post(format!("http://{addr}/users"))
            .header("content-type", "application/json")
            .body(r#"{"name":"Alice","email":"alice@example.com"}"#)
            .send()
            .await
            .expect("request to create user should succeed");

        let created: serde_json::Value = response
            .json()
            .await
            .expect("response should be valid JSON");

        created["id"]
            .as_i64()
            .expect("created user should have an id")
    }

    #[sqlx::test]
    async fn image_endpoints_reject_missing_bearer_token(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/users/1/images"))
            .send()
            .await
            .expect("request without token should complete");

        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn image_endpoints_reject_wrong_bearer_token(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/users/1/images"))
            .header(
                header::AUTHORIZATION,
                header::HeaderValue::from_static("Bearer wrong-token"),
            )
            .send()
            .await
            .expect("request with wrong token should complete");

        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn upload_image_returns_created_with_key(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();
        let user_id = create_user(&client, &addr).await;

        let data = STANDARD.encode(b"hello world");
        let response = client
            .post(format!("http://{addr}/users/{user_id}/images"))
            .headers(bearer_header())
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "filename": "photo.png",
                "content_type": "image/png",
                "data": data,
            }))
            .send()
            .await
            .expect("request to upload image should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::CREATED);

        let body: serde_json::Value = response
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(
            body["id"].as_i64().expect("response should include an id") > 0,
            true,
            "response should include a positive id"
        );
        assert!(
            body["key"]
                .as_str()
                .expect("response should include a key")
                .ends_with("photo.png"),
            "key should end with the uploaded filename"
        );
        assert_eq!(body["content_type"], "image/png");
        assert_eq!(body["user_id"].as_i64(), Some(user_id));
        assert!(
            body["url"]
                .as_str()
                .expect("response should include a url")
                .starts_with("http"),
            "url should be a presigned url"
        );
    }

    #[sqlx::test]
    async fn upload_image_with_invalid_base64_returns_bad_request(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();
        let user_id = create_user(&client, &addr).await;

        let response = client
            .post(format!("http://{addr}/users/{user_id}/images"))
            .headers(bearer_header())
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "filename": "photo.png",
                "content_type": "image/png",
                "data": "not valid base64!!!",
            }))
            .send()
            .await
            .expect("request to upload image should complete");

        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    }

    #[sqlx::test]
    async fn get_images_returns_uploaded_images(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();
        let user_id = create_user(&client, &addr).await;

        let data = STANDARD.encode(b"hello world");
        client
            .post(format!("http://{addr}/users/{user_id}/images"))
            .headers(bearer_header())
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "filename": "photo.png",
                "content_type": "image/png",
                "data": data,
            }))
            .send()
            .await
            .expect("request to upload image should succeed");

        let response = client
            .get(format!("http://{addr}/users/{user_id}/images"))
            .headers(bearer_header())
            .send()
            .await
            .expect("request to list images should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let images: serde_json::Value = response
            .json()
            .await
            .expect("response should be valid JSON");

        let images = images.as_array().expect("response should be an array");
        assert_eq!(images.len(), 1);
        assert_eq!(images[0]["content_type"], "image/png");
        assert_eq!(images[0]["user_id"], user_id);
    }

    #[sqlx::test]
    async fn get_images_for_user_without_images_returns_empty(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();
        let user_id = create_user(&client, &addr).await;

        let response = client
            .get(format!("http://{addr}/users/{user_id}/images"))
            .headers(bearer_header())
            .send()
            .await
            .expect("request to list images should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let images: serde_json::Value = response
            .json()
            .await
            .expect("response should be valid JSON");

        assert!(
            images
                .as_array()
                .expect("response should be an array")
                .is_empty()
        );
    }

    #[sqlx::test]
    async fn delete_image_removes_it(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();
        let user_id = create_user(&client, &addr).await;

        let data = STANDARD.encode(b"hello world");
        client
            .post(format!("http://{addr}/users/{user_id}/images"))
            .headers(bearer_header())
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "filename": "photo.png",
                "content_type": "image/png",
                "data": data,
            }))
            .send()
            .await
            .expect("request to upload image should succeed");

        let images: serde_json::Value = client
            .get(format!("http://{addr}/users/{user_id}/images"))
            .headers(bearer_header())
            .send()
            .await
            .expect("request to list images should succeed")
            .json()
            .await
            .expect("response should be valid JSON");
        let image_id = images[0]["id"]
            .as_i64()
            .expect("uploaded image should have an id");

        let response = client
            .delete(format!("http://{addr}/users/{user_id}/images/{image_id}"))
            .headers(bearer_header())
            .send()
            .await
            .expect("request to delete image should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);

        let remaining: serde_json::Value = client
            .get(format!("http://{addr}/users/{user_id}/images"))
            .headers(bearer_header())
            .send()
            .await
            .expect("request to list images should succeed")
            .json()
            .await
            .expect("response should be valid JSON");

        assert!(
            remaining
                .as_array()
                .expect("response should be an array")
                .is_empty()
        );
    }

    #[sqlx::test]
    async fn delete_missing_image_returns_not_found(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .delete(format!("http://{addr}/users/1/images/999"))
            .headers(bearer_header())
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
