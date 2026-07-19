use crate::{auth, handlers, state::AppState};
use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{get, post},
};
use std::sync::Arc;

pub fn routes(web_password: &str) -> Router<AppState> {
    Router::new()
        .nest("/feature_flags", feature_flags(web_password))
        .nest("/users", users())
}
pub fn feature_flags(web_password: &str) -> Router<AppState> {
    // Password-protect the HTML admin routes while leaving the JSON API open.
    let web = Router::new()
        .route(
            "/web",
            get(handlers::feature_flags::get_web).post(handlers::feature_flags::create_web),
        )
        .route("/web/{id}", post(handlers::feature_flags::update_web))
        .layer(from_fn_with_state(
            Arc::<str>::from(web_password),
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
