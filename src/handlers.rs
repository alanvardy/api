use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};

use crate::{
    models::{CreateUser, FeatureFlag, UpdateUser, User},
    state::AppState,
};

pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    let user = sqlx::query_as!(
        User,
        "INSERT INTO users (name, email) VALUES (?, ?) RETURNING id, name, email",
        payload.name,
        payload.email
    )
    .fetch_one(&state.db)
    .await
    .unwrap();

    (StatusCode::CREATED, Json(user))
}

pub async fn get_users(State(state): State<AppState>) -> Json<Vec<User>> {
    let users = sqlx::query_as!(User, "SELECT id, name, email FROM users")
        .fetch_all(&state.db)
        .await
        .unwrap();

    Json(users)
}

pub async fn get_feature_flags(State(state): State<AppState>) -> Json<Vec<FeatureFlag>> {
    let feature_flags = sqlx::query_as!(
        FeatureFlag,
        r#"SELECT id, name, enabled,
            created_at AS "created_at: DateTime<Utc>",
            updated_at AS "updated_at: DateTime<Utc>"
           FROM feature_flags"#
    )
    .fetch_all(&state.db)
    .await
    .unwrap();

    Json(feature_flags)
}

pub async fn get_user_by_id(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<User>, StatusCode> {
    let user = sqlx::query_as!(User, "SELECT id, name, email FROM users WHERE id = ?", id)
        .fetch_optional(&state.db)
        .await
        .unwrap();

    match user {
        Some(user) => Ok(Json(user)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn update_user(
    Path(id): Path<i64>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<User>, StatusCode> {
    let user = sqlx::query_as!(
        User,
        "UPDATE users
         SET name = ?, email = ?
         WHERE id = ?
         RETURNING id, name, email",
        payload.name,
        payload.email,
        id
    )
    .fetch_optional(&state.db)
    .await
    .unwrap();

    match user {
        Some(user) => Ok(Json(user)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn delete_user(Path(id): Path<i64>, State(state): State<AppState>) -> StatusCode {
    let result = sqlx::query!("DELETE FROM users WHERE id = ?", id)
        .execute(&state.db)
        .await
        .unwrap();

    if result.rows_affected() == 0 {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::NO_CONTENT
    }
}
