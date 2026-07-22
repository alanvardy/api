mod app;
mod handlers;
mod routes;
mod sentry;
mod test;

use app::db;
use app::env::Env;
use app::log;
use app::state::AppState;
use axum::Router;

#[tokio::main]
async fn main() {
    log::init();
    let env = Env::init().await;
    let _guard = sentry::init(&env);

    let pool = db::init().await;
    let http_port = env.http_port;
    let address = format!("0.0.0.0:{http_port}");

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind server");

    tracing::info!("Server running on http://localhost:{http_port}");

    axum::serve(listener, app(pool, &env)).await.unwrap();
}

fn app(pool: sqlx::SqlitePool, env: &Env) -> Router {
    let templates = init_templates();
    let state = AppState {
        db: pool,
        env: env.clone(),
        templates,
    };

    routes::routes(env)
        .layer(log::trace_layer())
        .with_state(state)
}

fn init_templates() -> minijinja::Environment<'static> {
    let mut templates = minijinja::Environment::new();
    templates.set_loader(minijinja::path_loader("templates"));
    templates.set_auto_escape_callback(|name| {
        if name.ends_with(".html") {
            minijinja::AutoEscape::Html
        } else {
            minijinja::AutoEscape::None
        }
    });
    templates
}
