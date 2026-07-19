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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind server");

    tracing::info!("Server running on http://localhost:3000");

    axum::serve(listener, app(pool)).await.unwrap();
}

fn app(pool: sqlx::SqlitePool) -> Router {
    let state = AppState { db: pool };

    routes::routes().layer(log::trace_layer()).with_state(state)
}
