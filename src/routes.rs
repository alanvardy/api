use crate::{handlers, state::AppState};
use axum::{
    Router,
    routing::{get, post},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/feature_flags", feature_flags())
        .nest("/users", users())
}
pub fn feature_flags() -> Router<AppState> {
    Router::new().route("/", get(handlers::get_feature_flags))
}
pub fn users() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::create_user).get(handlers::get_users))
        .route(
            "/{id}",
            get(handlers::get_user_by_id)
                .put(handlers::update_user)
                .delete(handlers::delete_user),
        )
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use crate::app;
    use chrono::Utc;
    use sqlx::{Pool, Sqlite, SqlitePool};

    async fn start_app(pool: Pool<Sqlite>) -> SocketAddr {
        // Bind to an OS-assigned port and run the real server in the background,
        // so the test exercises the app over HTTP rather than calling handlers directly.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app(pool)).await.unwrap();
        });

        address
    }

    #[sqlx::test]
    async fn create_user_and_verify_exists(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let create_response = client
            .post(format!("http://{addr}/users"))
            .header("content-type", "application/json")
            .body(r#"{"name":"Alice","email":"alice@example.com"}"#)
            .send()
            .await
            .expect("request to create user should succeed");

        assert_eq!(create_response.status(), reqwest::StatusCode::CREATED);

        let created: serde_json::Value = create_response
            .json()
            .await
            .expect("response should be valid JSON");
        let id = created["id"]
            .as_i64()
            .expect("created user should have an id");

        let get_response = client
            .get(format!("http://{addr}/users/{id}"))
            .send()
            .await
            .expect("request to fetch user should succeed");

        assert_eq!(get_response.status(), reqwest::StatusCode::OK);

        let fetched: serde_json::Value = get_response
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(fetched["id"], id);
        assert_eq!(fetched["name"], "Alice");
        assert_eq!(fetched["email"], "alice@example.com");
    }

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
}
