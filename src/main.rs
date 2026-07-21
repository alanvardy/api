mod app;
mod handlers;
mod routes;
mod test;

use app::db;
use app::env::Env;
use app::log;
use app::state::AppState;
use axum::Router;

#[tokio::main]
async fn main() {
    log::init();

    let pool = db::init().await;
    let env = Env::init().await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind server");

    tracing::info!("Server running on http://localhost:3000");

    axum::serve(listener, app(pool, &env)).await.unwrap();
}

fn app(pool: sqlx::SqlitePool, env: &Env) -> Router {
    let state = AppState {
        db: pool,
        env: env.clone(),
    };

    routes::routes(env)
        .layer(log::trace_layer())
        .with_state(state)
}
