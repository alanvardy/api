use crate::app::{auth, env::Env, state::AppState};
use crate::handlers;
use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{delete, get, post},
};
use std::sync::Arc;

pub fn routes(env: &Env) -> Router<AppState> {
    Router::new()
        .nest("/feature_flags", feature_flags(env))
        .nest("/images", images_web(env))
        .nest("/users", users(env))
}
pub fn feature_flags(env: &Env) -> Router<AppState> {
    let password = env.web_password.clone();
    // Password-protect the HTML admin routes while leaving the JSON API open.
    let web = Router::new()
        .route(
            "/web",
            get(handlers::web::feature_flags::get).post(handlers::web::feature_flags::create),
        )
        .route("/web/{id}", post(handlers::web::feature_flags::update))
        .layer(from_fn_with_state(
            Arc::<str>::from(password),
            auth::require_web_password,
        ));

    Router::new()
        .route("/", get(handlers::feature_flags::get))
        .merge(web)
}

pub fn images_web(env: &Env) -> Router<AppState> {
    let password = env.web_password.clone();

    Router::new()
        .route("/web", get(handlers::web::images::get))
        .route(
            "/web/images/{id}/approve",
            post(handlers::web::images::post_approve),
        )
        .route(
            "/web/images/{id}/delete",
            post(handlers::web::images::post_delete),
        )
        .layer(from_fn_with_state(
            Arc::<str>::from(password),
            auth::require_web_password,
        ))
}

pub fn images(env: &Env) -> Router<AppState> {
    let token = env.bearer_token.clone();

    Router::new()
        .route(
            "/",
            post(handlers::users::images::post).get(handlers::users::images::get),
        )
        .route("/{image_id}", delete(handlers::users::images::delete))
        .layer(from_fn_with_state(
            Arc::<str>::from(token),
            auth::require_bearer_token,
        ))
}
pub fn users(env: &Env) -> Router<AppState> {
    Router::new()
        .nest("/{id}/images", images(env))
        .route("/", post(handlers::users::create).get(handlers::users::get))
        .route(
            "/{id}",
            get(handlers::users::get_by_id)
                .put(handlers::users::update)
                .delete(handlers::users::delete),
        )
}
