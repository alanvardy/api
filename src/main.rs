mod app;
mod domain;
mod infra;
mod interfaces;
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

#[cfg(test)]
mod tests {
    use rust_arkitect::dsl::architectural_rules::ArchitecturalRules;
    use rust_arkitect::dsl::arkitect::Arkitect;
    use rust_arkitect::dsl::project::Project;
    #[test]
    fn test_architectural_rules() {
        Arkitect::init_logger();
        let domain_deps = vec!["base64", "chrono", "std"];

        let infra_deps = [vec!["axum", "sqlx", "sentry"], domain_deps.clone()].concat();
        let project = Project::from_current_crate();

        let rules = Box::new(
            ArchitecturalRules::define()
                .rules_for_module("api::app")
                .it_must_not_depend_on(&["api::interfaces"])
                .rules_for_module("api::domain")
                .it_may_depend_on(&domain_deps),
        )
        .rules_for_module("api::infra")
        .it_may_depend_on(&infra_deps)
        .build();

        let result = Arkitect::ensure_that(project).complies_with(rules);

        assert!(
            result.is_ok(),
            "Detected {} violations",
            result.err().unwrap().len()
        );
    }
}
