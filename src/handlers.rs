use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    models::{CreateUser, UpdateUser, User},
    state::AppState,
};

pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (name, email)
         VALUES (?, ?)
         RETURNING id, name, email",
    )
    .bind(payload.name)
    .bind(payload.email)
    .fetch_one(&state.db)
    .await
    .unwrap();

    (StatusCode::CREATED, Json(user))
}

pub async fn get_users(State(state): State<AppState>) -> Json<Vec<User>> {
    let users = sqlx::query_as::<_, User>("SELECT id, name, email FROM users")
        .fetch_all(&state.db)
        .await
        .unwrap();

    Json(users)
}

pub async fn get_user_by_id(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<User>, StatusCode> {
    let user = sqlx::query_as::<_, User>("SELECT id, name, email FROM users WHERE id = ?")
        .bind(id)
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
    let user = sqlx::query_as::<_, User>(
        "UPDATE users
         SET name = ?, email = ?
         WHERE id = ?
         RETURNING id, name, email",
    )
    .bind(payload.name)
    .bind(payload.email)
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .unwrap();

    match user {
        Some(user) => Ok(Json(user)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn delete_user(Path(id): Path<i64>, State(state): State<AppState>) -> StatusCode {
    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .unwrap();

    if result.rows_affected() == 0 {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::NO_CONTENT
    }
}
