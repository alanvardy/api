mod auth;
mod db;
mod handlers;
mod log;
mod models;
mod routes;
mod state;

use axum::Router;

use state::AppState;

#[tokio::main]
async fn main() {
    log::init();

    let pool = db::init_db().await;

    // Require a non-empty web password so the admin UI never boots unprotected.
    let web_password = std::env::var("FEATURE_FLAGS_WEB_PASSWORD")
        .ok()
        .filter(|password| !password.is_empty())
        .expect("FEATURE_FLAGS_WEB_PASSWORD must be set and non-empty");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind server");

    tracing::info!("Server running on http://localhost:3000");

    axum::serve(listener, app(pool, &web_password))
        .await
        .unwrap();
}

fn app(pool: sqlx::SqlitePool, web_password: &str) -> Router {
    let state = AppState { db: pool };

    routes::routes(web_password)
        .layer(log::trace_layer())
        .with_state(state)
}
