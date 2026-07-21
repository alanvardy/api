use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};

use crate::app::{
    error::AppError,
    models::{CreateUser, UpdateUser, User},
    state::AppState,
};
pub mod images;
pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), AppError> {
    let now = Utc::now();
    let user = sqlx::query_as!(
        User,
        r#"INSERT INTO users (name, email, created_at, updated_at) VALUES (?, ?, ?, ?) RETURNING id, name, email,
            created_at AS "created_at: DateTime<Utc>",
            updated_at AS "updated_at: DateTime<Utc>""#,
        payload.name,
        payload.email,
        now,
        now,
    )
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(user)))
}

pub async fn get(State(state): State<AppState>) -> Result<Json<Vec<User>>, AppError> {
    let users = sqlx::query_as!(
        User,
        r#"
        SELECT id, name, email,
            created_at AS "created_at: DateTime<Utc>",
            updated_at AS "updated_at: DateTime<Utc>"
        FROM users"#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(users))
}
pub async fn get_by_id(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<User>, AppError> {
    let user = sqlx::query_as!(
        User,
        r#"SELECT id, name, email,
            created_at AS "created_at: DateTime<Utc>",
            updated_at AS "updated_at: DateTime<Utc>"
        FROM users WHERE id = ?"#,
        id
    )
    .fetch_optional(&state.db)
    .await?;

    user.map(Json).ok_or(AppError::NotFound)
}

pub async fn update(
    Path(id): Path<i64>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<User>, AppError> {
    let user = sqlx::query_as!(
        User,
        r#"UPDATE users
         SET name = ?, email = ?
         WHERE id = ?
         RETURNING id, name, email,
            created_at AS "created_at: DateTime<Utc>",
            updated_at AS "updated_at: DateTime<Utc>""#,
        payload.name,
        payload.email,
        id
    )
    .fetch_optional(&state.db)
    .await?;

    user.map(Json).ok_or(AppError::NotFound)
}

pub async fn delete(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query!("DELETE FROM users WHERE id = ?", id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        Ok(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}
#[cfg(test)]
mod tests {
    use crate::test::*;
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn create_user_and_verify_exists(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let create_response = client
            .post(format!("http://{addr}/users"))
            .header("content-type", "application/json")
            .body(r#"{"name":"Alice","email":"alice@example.com"}"#)
            .send()
            .await
            .expect("request to create user should succeed");

        assert_eq!(create_response.status(), reqwest::StatusCode::CREATED);

        let created: serde_json::Value = create_response
            .json()
            .await
            .expect("response should be valid JSON");
        let id = created["id"]
            .as_i64()
            .expect("created user should have an id");

        let get_response = client
            .get(format!("http://{addr}/users/{id}"))
            .send()
            .await
            .expect("request to fetch user should succeed");

        assert_eq!(get_response.status(), reqwest::StatusCode::OK);

        let fetched: serde_json::Value = get_response
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(fetched["id"], id);
        assert_eq!(fetched["name"], "Alice");
        assert_eq!(fetched["email"], "alice@example.com");
    }

    #[sqlx::test]
    async fn get_missing_user_returns_not_found_with_error_body(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/users/999"))
            .send()
            .await
            .expect("request to fetch missing user should complete");

        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

        let body: serde_json::Value = response
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(body["error"], "resource not found");
    }

    #[sqlx::test]
    async fn get_all_users_returns_created_users(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        // Create two users
        let create_response = client
            .post(format!("http://{addr}/users"))
            .header("content-type", "application/json")
            .body(r#"{"name":"Alice","email":"alice@example.com"}"#)
            .send()
            .await
            .expect("request to create first user should succeed");
        assert_eq!(create_response.status(), reqwest::StatusCode::CREATED);

        let create_response = client
            .post(format!("http://{addr}/users"))
            .header("content-type", "application/json")
            .body(r#"{"name":"Bob","email":"bob@example.com"}"#)
            .send()
            .await
            .expect("request to create second user should succeed");
        assert_eq!(create_response.status(), reqwest::StatusCode::CREATED);

        // Fetch all users
        let response = client
            .get(format!("http://{addr}/users"))
            .send()
            .await
            .expect("request to fetch all users should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let users: serde_json::Value = response
            .json()
            .await
            .expect("response should be valid JSON");

        let array = users.as_array().expect("response should be an array");
        assert_eq!(array.len(), 2);

        let names: Vec<&str> = array.iter().map(|u| u["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"Alice"));
        assert!(names.contains(&"Bob"));
    }

    #[sqlx::test]
    async fn get_all_users_empty_when_no_users(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/users"))
            .send()
            .await
            .expect("request to fetch users should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let users: serde_json::Value = response
            .json()
            .await
            .expect("response should be valid JSON");

        assert!(
            users
                .as_array()
                .expect("response should be an array")
                .is_empty()
        );
    }
}
