use axum::{
    extract::{Path, State},
    response::{Html, Redirect},
};
use chrono::{DateTime, Utc};
use minijinja::context;
use serde::Serialize;
use sqlx::Row;
use std::time::Duration;

use crate::app::{
    error::{AppError, WebError},
    files,
    state::AppState,
};

pub async fn get(State(state): State<AppState>) -> Result<Html<String>, WebError> {
    let rows = sqlx::query(
        "SELECT id, key, content_type, user_id, created_at, updated_at, ai_flagged_at
           FROM files
           WHERE ai_flagged_at IS NOT NULL
             AND human_reviewed_at IS NULL",
    )
    .fetch_all(&state.db)
    .await?;

    let aws_config = &state.env.aws_config;
    let bucket = &state.env.s3_bucket;

    let mut flagged = Vec::with_capacity(rows.len());
    for row in rows {
        let key: String = row.get("key");
        let url = files::presign_download(aws_config, bucket, &key, ONE_DAY).await?;
        flagged.push(FlaggedFileView {
            id: row.get("id"),
            content_type: row.get::<&str, _>("content_type").to_string(),
            user_id: row.get("user_id"),
            url,
            created_at: format_datetime(row.get("created_at")),
            updated_at: format_datetime(row.get("updated_at")),
            ai_flagged_at: format_datetime(row.get("ai_flagged_at")),
        });
    }
    let html = state
        .templates
        .get_template("images.html")?
        .render(context! { flagged })?;

    Ok(Html(html))
}

/// View model for the images web template. Pre-formats dates
/// and keeps template logic minimal.
#[derive(Serialize)]
struct FlaggedFileView {
    id: i64,
    content_type: String,
    user_id: i64,
    url: String,
    created_at: (String, String),
    updated_at: (String, String),
    ai_flagged_at: (String, String),
}

pub async fn post_approve(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Redirect, WebError> {
    let now = Utc::now();
    sqlx::query!(
        "UPDATE files SET human_reviewed_at = ? WHERE id = ?",
        now,
        id
    )
    .execute(&state.db)
    .await?;

    Ok(Redirect::to("/images/web"))
}

pub async fn post_delete(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Redirect, WebError> {
    let aws_config = &state.env.aws_config;
    let bucket = &state.env.s3_bucket;

    let key: String = sqlx::query_scalar("SELECT key FROM files WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(WebError::App(AppError::NotFound))?;

    // Remove the object from storage first so a database delete failure does
    // not leave a dangling record pointing at a missing object.
    files::delete(aws_config, bucket, &key).await?;

    sqlx::query!("DELETE FROM files WHERE id = ?", id)
        .execute(&state.db)
        .await?;

    Ok(Redirect::to("/images/web"))
}

const ONE_DAY: Duration = Duration::from_secs(86400);

fn format_datetime(value: DateTime<Utc>) -> (String, String) {
    (
        value.format("%H:%M:%S UTC").to_string(),
        value.format("%-d %B %Y").to_string(),
    )
}

#[cfg(test)]
mod tests {
    use crate::test::*;
    use chrono::Utc;
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn get_images_web_renders_flagged_files(pool: SqlitePool) {
        let now = Utc::now();

        // Create a user first to satisfy the foreign key constraint
        sqlx::query(
            "INSERT INTO users (name, email, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind("Alice")
        .bind("alice@example.com")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("inserting a user should succeed");

        sqlx::query(
            "INSERT INTO files (key, content_type, user_id, created_at, updated_at, ai_flagged_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("uploads/photo.png")
        .bind("image/png")
        .bind(1)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("inserting a flagged file should succeed");

        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/images/web"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .send()
            .await
            .expect("request to fetch images web page should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.contains("text/html"))
        );

        let body = response
            .text()
            .await
            .expect("response should have a text body");

        assert!(body.contains("<table"));
        // Presigned URL rendered as thumbnail linking to full size
        assert!(body.contains("<img"));
        assert!(body.contains("class=\"thumbnail\""));
        assert!(body.contains("<a"));
        assert!(body.contains("target=\"_blank\""));
        assert!(body.contains("https:&#x2f;&#x2f;fake-presigned&#x2f;uploads&#x2f;photo.png"));
        assert!(body.contains("image&#x2f;png"));
    }

    #[sqlx::test]
    async fn get_images_web_excludes_reviewed_files(pool: SqlitePool) {
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO users (name, email, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind("Alice")
        .bind("alice@example.com")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("inserting a user should succeed");

        // Flagged but also reviewed — should not appear
        sqlx::query(
            "INSERT INTO files (key, content_type, user_id, created_at, updated_at, ai_flagged_at, human_reviewed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind("uploads/reviewed.png")
        .bind("image/png")
        .bind(1)
        .bind(now)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("inserting a reviewed file should succeed");

        // Flagged, not reviewed — should appear
        sqlx::query(
            "INSERT INTO files (key, content_type, user_id, created_at, updated_at, ai_flagged_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("uploads/pending.png")
        .bind("image/jpeg")
        .bind(1)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("inserting a flagged file should succeed");

        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/images/web"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .send()
            .await
            .expect("request to fetch images web page should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let body = response
            .text()
            .await
            .expect("response should have a text body");

        assert!(!body.contains("fake-presigned&#x2f;uploads&#x2f;reviewed.png"));
        assert!(body.contains("fake-presigned&#x2f;uploads&#x2f;pending.png"));
    }

    #[sqlx::test]
    async fn web_route_without_credentials_is_unauthorized(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/images/web"))
            .send()
            .await
            .expect("request without credentials should complete");

        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
        assert!(
            response
                .headers()
                .get("www-authenticate")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.contains("Basic"))
        );
    }

    #[sqlx::test]
    async fn post_delete_removes_flagged_file(pool: SqlitePool) {
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO users (name, email, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind("Alice")
        .bind("alice@example.com")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("inserting a user should succeed");

        let file_id: i64 = sqlx::query_scalar(
            "INSERT INTO files (key, content_type, user_id, created_at, updated_at, ai_flagged_at)
             VALUES (?, ?, ?, ?, ?, ?)
             RETURNING id",
        )
        .bind("uploads/flagged.png")
        .bind("image/png")
        .bind(1)
        .bind(now)
        .bind(now)
        .bind(now)
        .fetch_one(&pool)
        .await
        .expect("inserting a flagged file should succeed");

        let addr = start_app(pool).await;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let response = client
            .post(format!("http://{addr}/images/web/images/{file_id}/delete"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .send()
            .await
            .expect("request to delete flagged file should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::SEE_OTHER);
        assert_eq!(
            response
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok()),
            Some("/images/web")
        );

        // Verify the file no longer appears in the flagged list.
        let body = client
            .get(format!("http://{addr}/images/web"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .send()
            .await
            .expect("request to fetch images web page should succeed")
            .text()
            .await
            .expect("response should have a text body");

        assert!(!body.contains("fake-presigned&#x2f;uploads&#x2f;flagged.png"));
    }

    #[sqlx::test]
    async fn post_delete_missing_file_returns_not_found(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let response = client
            .post(format!("http://{addr}/images/web/images/999/delete"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .send()
            .await
            .expect("request to delete missing file should complete");

        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

        let body = response
            .text()
            .await
            .expect("response should have a text body");

        assert!(body.contains("resource not found"));
    }
}
