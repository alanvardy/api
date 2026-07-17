mod db;
mod handlers;
mod models;
mod state;

use axum::{
    Router,
    extract::{MatchedPath, Request},
    routing::{get, post},
};
use tower_http::trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_subscriber::{EnvFilter, fmt};

use state::AppState;

// Emit one structured JSON log line per event so Fly.io's stdout capture can
// forward request logs to downstream aggregators such as Loki/Grafana.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info"));

    fmt()
        .json()
        .flatten_event(true)
        .with_current_span(false)
        .with_env_filter(filter)
        .init();
}

#[tokio::main]
async fn main() {
    init_tracing();

    let pool = db::init_db().await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind server");

    tracing::info!("Server running on http://localhost:3000");

    axum::serve(listener, app(pool)).await.unwrap();
}

fn app(pool: sqlx::SqlitePool) -> Router {
    let state = AppState { db: pool };

    // Log the matched route (e.g. `/users/{id}`) rather than the concrete path
    // so per-request logs stay low cardinality and group cleanly in Grafana.
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|request: &Request<_>| {
            let path = request
                .extensions()
                .get::<MatchedPath>()
                .map(MatchedPath::as_str)
                .unwrap_or_else(|| request.uri().path());

            tracing::info_span!(
                "http_request",
                method = %request.method(),
                path = %path,
            )
        })
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

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
        .layer(trace_layer)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn create_user_and_verify_exists(pool: SqlitePool) {
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
