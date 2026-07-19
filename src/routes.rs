use crate::{handlers, state::AppState};
use axum::{
    Router,
    routing::{get, post},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/feature_flags", feature_flags())
        .nest("/users", users())
}
pub fn feature_flags() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::feature_flags::get))
        .route(
            "/web",
            get(handlers::feature_flags::get_web).post(handlers::feature_flags::create_web),
        )
        .route("/web/{id}", post(handlers::feature_flags::update_web))
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
