use crate::{auth, env::Env, handlers, state::AppState};
use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post},
};
use std::sync::Arc;

pub fn routes(env: &Env) -> Router<AppState> {
    Router::new()
        .nest("/feature_flags", feature_flags(env))
        .nest("/users", users())
}
pub fn feature_flags(env: &Env) -> Router<AppState> {
    let password = env.feature_flags_web_password.clone();
    // Password-protect the HTML admin routes while leaving the JSON API open.
    let web = Router::new()
        .route(
            "/web",
            get(handlers::feature_flags::get_web).post(handlers::feature_flags::create_web),
        )
        .route("/web/{id}", post(handlers::feature_flags::update_web))
        .layer(from_fn_with_state(
            Arc::<str>::from(password),
            auth::require_web_password,
        ));

    Router::new()
        .route("/", get(handlers::feature_flags::get))
        .merge(web)
}
pub fn users() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::users::create).get(handlers::users::get))
        .route(
            "/{id}",
            get(handlers::users::get_by_id)
                .put(handlers::users::update)
                .delete(handlers::users::delete),
        )
}
