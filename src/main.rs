mod auth;
mod db;
mod env;
mod handlers;
mod log;
mod models;
mod routes;
mod state;
mod test;

use axum::Router;
use env::Env;
use state::AppState;

#[tokio::main]
async fn main() {
    log::init();

    let pool = db::init().await;
    let env = Env::init();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind server");

    tracing::info!("Server running on http://localhost:3000");

    axum::serve(listener, app(pool, &env)).await.unwrap();
}

fn app(pool: sqlx::SqlitePool, env: &Env) -> Router {
    let state = AppState { db: pool };

    routes::routes(env)
        .layer(log::trace_layer())
        .with_state(state)
}
