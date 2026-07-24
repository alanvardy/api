use crate::app;
use axum::{Json, extract::State};

use crate::app::{error::AppError, models::FeatureFlag, state::AppState};
pub async fn get(State(state): State<AppState>) -> Result<Json<Vec<FeatureFlag>>, AppError> {
    let feature_flags = app::feature_flags::list(&state.db).await?;
    Ok(Json(feature_flags))
}

#[cfg(test)]
mod tests {
    use crate::test::*;
    use chrono::Utc;
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn get_feature_flags_returns_inserted_flag(pool: SqlitePool) {
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
            .get(format!("http://{addr}/feature_flags"))
            .send()
            .await
            .expect("request to fetch feature flags should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let flags: serde_json::Value = response
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(
            flags.as_array().expect("response should be an array").len(),
            1
        );
        assert_eq!(flags[0]["name"], "dark_mode");
        assert_eq!(flags[0]["enabled"], true);
    }

    #[sqlx::test]
    async fn json_route_does_not_require_credentials(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/feature_flags"))
            .send()
            .await
            .expect("request to fetch feature flags should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }
}
