mod app;
mod domain;
mod infra;
mod interfaces;
#[cfg(test)]
mod test;

use app::env::Env;
use app::state::AppState;
use axum::Router;

#[tokio::main]
async fn main() {
    app::log::init();
    let env = Env::init().await;
    let _guard = infra::sentry::init(&env.sentry_dsn);

    let pool = infra::db::init().await;
    let http_port = env.http_port;
    let address = format!("0.0.0.0:{http_port}");

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind server");

    tracing::info!("Server running on http://localhost:{http_port}");

    axum::serve(listener, app(pool, &env)).await.unwrap();
}

fn app(pool: sqlx::SqlitePool, env: &Env) -> Router {
    let templates = app::templates::init();
    let state = AppState {
        db: pool,
        env: env.clone(),
        templates,
    };

    interfaces::routes::routes(env)
        .layer(app::log::trace_layer())
        .with_state(state)
}
