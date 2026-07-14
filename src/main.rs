mod db;
mod handlers;
mod models;
mod state;

use axum::{
    Router,
    routing::{get, post},
};

use state::AppState;

#[tokio::main]
async fn main() {
    let pool = db::init_db().await;

    db::create_table(&pool).await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind server");

    println!("Server running on http://localhost:3000");

    axum::serve(listener, app(pool)).await.unwrap();
}

fn app(pool: sqlx::SqlitePool) -> Router {
    let state = AppState { db: pool };

    Router::new()
        .route(
            "/users",
            post(handlers::create_user).get(handlers::get_users),
        )
        .route(
            "/users/{id}",
            get(handlers::get_user_by_id)
                .put(handlers::update_user)
                .delete(handlers::delete_user),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn create_user_and_verify_exists(pool: SqlitePool) {
        db::create_table(&pool).await;

        // Bind to an OS-assigned port and run the real server in the background,
        // so the test exercises the app over HTTP rather than calling handlers directly.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app(pool)).await.unwrap();
        });

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
}
