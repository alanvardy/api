use axum::{
    Json,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Serialize;

// Application error for JSON API handlers. Each variant maps to an HTTP status
// and a client-safe message; underlying causes are logged, never serialized.
pub enum AppError {
    NotFound,
    BadRequest(&'static str),
    Storage,
    Database(sqlx::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

// Translates a database error into the status and client-facing message it
// should produce, logging the underlying cause for unexpected failures.
fn database_response(err: sqlx::Error) -> (StatusCode, &'static str) {
    match &err {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            (StatusCode::CONFLICT, "resource already exists")
        }
        _ => {
            tracing::error!(error = %err, "database error");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "resource not found"),
            AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            AppError::Storage => (StatusCode::INTERNAL_SERVER_ERROR, "internal server error"),
            AppError::Database(err) => database_response(err),
        };

        (
            status,
            Json(ErrorBody {
                error: message.to_string(),
            }),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

// Application error for the HTML admin handlers. Renders an HTML page instead
// of JSON so failures stay consistent with the feature-flag management UI.
pub enum WebError {
    Database(sqlx::Error),
    Template(minijinja::Error),
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            WebError::Database(err) => database_response(err),
            WebError::Template(err) => {
                tracing::error!(error = %err, "template render error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
            }
        };

        (status, Html(render_error_page(status, message))).into_response()
    }
}

impl From<sqlx::Error> for WebError {
    fn from(err: sqlx::Error) -> Self {
        WebError::Database(err)
    }
}

impl From<minijinja::Error> for WebError {
    fn from(err: minijinja::Error) -> Self {
        WebError::Template(err)
    }
}

fn render_error_page(status: StatusCode, message: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Error {code}</title>
<style>
body {{ font-family: system-ui, -apple-system, "Segoe UI", sans-serif; margin: 0; padding: 3rem 1.5rem; background: #f4f6fb; color: #1e293b; }}
.card {{ max-width: 480px; margin: 0 auto; background: #fff; border: 1px solid #e2e8f0; border-radius: 12px; padding: 2rem; text-align: center; }}
h1 {{ margin: 0 0 0.5rem; font-size: 1.5rem; }}
p {{ color: #64748b; margin: 0 0 1.5rem; }}
a {{ color: #4f46e5; text-decoration: none; font-weight: 600; }}
</style>
</head>
<body>
<div class="card">
<h1>Error {code}</h1>
<p>{message}</p>
<a href="/feature_flags/web">Back to feature flags</a>
</div>
</body>
</html>"#,
        code = status.as_u16(),
        message = message,
    )
}
