use axum::{
    Form,
    extract::{Path, State},
    response::{Html, Redirect},
};
use chrono::{DateTime, Utc};
use minijinja::context;
use serde::Serialize;

use crate::app::{
    error::WebError,
    models::{CreateFeatureFlag, FeatureFlag, UpdateFeatureFlag},
    state::AppState,
};

pub async fn get(State(state): State<AppState>) -> Result<Html<String>, WebError> {
    let feature_flags = sqlx::query_as!(
        FeatureFlag,
        r#"SELECT id, name, enabled,
            created_at AS "created_at: DateTime<Utc>",
            updated_at AS "updated_at: DateTime<Utc>"
           FROM feature_flags"#
    )
    .fetch_all(&state.db)
    .await?;

    let flags: Vec<FlagView> = feature_flags.into_iter().map(FlagView::from).collect();
    let html = state
        .templates
        .get_template("feature_flags.html")?
        .render(context! { flags })?;

    Ok(Html(html))
}

pub async fn create(
    State(state): State<AppState>,
    Form(payload): Form<CreateFeatureFlag>,
) -> Result<Redirect, WebError> {
    let now = Utc::now();
    sqlx::query!(
        "INSERT INTO feature_flags (name, enabled, created_at, updated_at)
         VALUES (?, ?, ?, ?)",
        payload.name,
        payload.enabled,
        now,
        now
    )
    .execute(&state.db)
    .await?;

    Ok(Redirect::to("/feature_flags/web"))
}

pub async fn update(
    Path(id): Path<i64>,
    State(state): State<AppState>,
    Form(payload): Form<UpdateFeatureFlag>,
) -> Result<Redirect, WebError> {
    let now = Utc::now();
    sqlx::query!(
        "UPDATE feature_flags
         SET enabled = ?, updated_at = ?
         WHERE id = ?",
        payload.enabled,
        now,
        id
    )
    .execute(&state.db)
    .await?;

    Ok(Redirect::to("/feature_flags/web"))
}

/// View model for the feature flags template. Pre-formats dates
/// and keeps template logic minimal.
#[derive(Serialize)]
struct FlagView {
    id: i64,
    name: String,
    enabled: bool,
    created_at: (String, String),
    updated_at: (String, String),
}

impl From<FeatureFlag> for FlagView {
    fn from(flag: FeatureFlag) -> Self {
        Self {
            id: flag.id,
            name: flag.name,
            enabled: flag.enabled,
            created_at: format_datetime(flag.created_at),
            updated_at: format_datetime(flag.updated_at),
        }
    }
}

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
    async fn get_feature_flags_web_renders_table(pool: SqlitePool) {
        sqlx::query(
            "INSERT INTO feature_flags (name, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind("dark_mode")
        .bind(true)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&pool)
        .await
        .expect("inserting a feature flag should succeed");

        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/feature_flags/web"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .send()
            .await
            .expect("request to fetch feature flags web page should succeed");

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
        assert!(body.contains("dark_mode"));
        assert!(body.contains("Enabled"));
    }

    #[sqlx::test]
    async fn create_feature_flag_web_inserts_flag(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let response = client
            .post(format!("http://{addr}/feature_flags/web"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .header("content-type", "application/x-www-form-urlencoded")
            .body("name=new_flag&enabled=true")
            .send()
            .await
            .expect("request to create feature flag should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::SEE_OTHER);

        let flags: serde_json::Value = client
            .get(format!("http://{addr}/feature_flags"))
            .send()
            .await
            .expect("request to fetch feature flags should succeed")
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(flags[0]["name"], "new_flag");
        assert_eq!(flags[0]["enabled"], true);
    }

    #[sqlx::test]
    async fn create_feature_flag_web_defaults_enabled_to_false(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        client
            .post(format!("http://{addr}/feature_flags/web"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .header("content-type", "application/x-www-form-urlencoded")
            .body("name=off_flag")
            .send()
            .await
            .expect("request to create feature flag should succeed");

        let flags: serde_json::Value = client
            .get(format!("http://{addr}/feature_flags"))
            .send()
            .await
            .expect("request to fetch feature flags should succeed")
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(flags[0]["name"], "off_flag");
        assert_eq!(flags[0]["enabled"], false);
    }

    #[sqlx::test]
    async fn update_feature_flag_web_updates_flag(pool: SqlitePool) {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO feature_flags (name, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?)
             RETURNING id",
        )
        .bind("dark_mode")
        .bind(true)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&pool)
        .await
        .expect("inserting a feature flag should succeed");

        let addr = start_app(pool).await;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let response = client
            .post(format!("http://{addr}/feature_flags/web/{id}"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .header("content-type", "application/x-www-form-urlencoded")
            .send()
            .await
            .expect("request to update feature flag should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::SEE_OTHER);

        let flags: serde_json::Value = client
            .get(format!("http://{addr}/feature_flags"))
            .send()
            .await
            .expect("request to fetch feature flags should succeed")
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(flags[0]["enabled"], false);
    }

    #[sqlx::test]
    async fn create_duplicate_feature_flag_web_returns_conflict(pool: SqlitePool) {
        sqlx::query(
            "INSERT INTO feature_flags (name, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind("dark_mode")
        .bind(true)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&pool)
        .await
        .expect("inserting a feature flag should succeed");

        let addr = start_app(pool).await;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let response = client
            .post(format!("http://{addr}/feature_flags/web"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .header("content-type", "application/x-www-form-urlencoded")
            .body("name=dark_mode&enabled=true")
            .send()
            .await
            .expect("request to create duplicate feature flag should complete");

        assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    }

    #[sqlx::test]
    async fn web_route_without_credentials_is_unauthorized(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/feature_flags/web"))
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
    async fn web_route_with_wrong_password_is_unauthorized(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/feature_flags/web"))
            .basic_auth(WEB_USERNAME, Some("wrong-password"))
            .send()
            .await
            .expect("request with wrong password should complete");

        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    }
}
