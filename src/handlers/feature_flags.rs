use axum::{
    Form, Json,
    extract::{Path, State},
    response::{Html, Redirect},
};
use chrono::{DateTime, Utc};

use crate::{
    models::{CreateFeatureFlag, FeatureFlag, UpdateFeatureFlag},
    state::AppState,
};
pub async fn get(State(state): State<AppState>) -> Json<Vec<FeatureFlag>> {
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

pub async fn get_web(State(state): State<AppState>) -> Html<String> {
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

    Html(render_page(&feature_flags))
}

pub async fn create_web(
    State(state): State<AppState>,
    Form(payload): Form<CreateFeatureFlag>,
) -> Redirect {
    let now = Utc::now();
    sqlx::query!(
        "INSERT INTO feature_flags (name, enabled, created_at, updated_at)
         VALUES (?, ?, ?, ?)",
        payload.name,
        payload.enabled,
        now,
        now
    )
    .execute(&state.db)
    .await
    .unwrap();

    Redirect::to("/feature_flags/web")
}

pub async fn update_web(
    Path(id): Path<i64>,
    State(state): State<AppState>,
    Form(payload): Form<UpdateFeatureFlag>,
) -> Redirect {
    let now = Utc::now();
    sqlx::query!(
        "UPDATE feature_flags
         SET name = ?, enabled = ?, updated_at = ?
         WHERE id = ?",
        payload.name,
        payload.enabled,
        now,
        id
    )
    .execute(&state.db)
    .await
    .unwrap();

    Redirect::to("/feature_flags/web")
}

fn render_page(feature_flags: &[FeatureFlag]) -> String {
    let rows = feature_flags
        .iter()
        .map(|flag| {
            let name = html_escape(&flag.name);
            let checked = if flag.enabled { " checked" } else { "" };
            format!(
                r#"<tr>
<td>{id}</td>
<td>{created}</td>
<td>{updated}</td>
<td>{name}</td>
<td>{status}</td>
<td>
<form method="post" action="/feature_flags/web/{id}" class="inline-form">
<input type="text" name="name" value="{name}" required>
<label><input type="checkbox" name="enabled" value="true"{checked}> Enabled</label>
<button type="submit">Save</button>
</form>
</td>
</tr>"#,
                id = flag.id,
                status = if flag.enabled { "Enabled" } else { "Disabled" },
                created = flag.created_at.to_rfc3339(),
                updated = flag.updated_at.to_rfc3339(),
            )
        })
        .collect::<String>();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Feature Flags</title>
<style>
body {{ font-family: system-ui, sans-serif; margin: 2rem; }}
h1 {{ margin-bottom: 1rem; }}
table {{ border-collapse: collapse; width: 100%; }}
th, td {{ border: 1px solid #ccc; padding: 0.5rem 0.75rem; text-align: left; }}
th {{ background: #f5f5f5; }}
tr:nth-child(even) {{ background: #fafafa; }}
form {{ margin: 0; }}
.inline-form {{ display: flex; gap: 0.5rem; align-items: center; }}
.create-form {{ display: flex; gap: 0.75rem; align-items: center; margin-bottom: 1.5rem; }}
label {{ display: flex; gap: 0.25rem; align-items: center; }}
</style>
</head>
<body>
<h1>Feature Flags</h1>
<form method="post" action="/feature_flags/web" class="create-form">
<input type="text" name="name" placeholder="Flag name" required>
<label><input type="checkbox" name="enabled" value="true"> Enabled</label>
<button type="submit">Create flag</button>
</form>
<table>
<thead>
<tr><th>ID</th><th>Created</th><th>Updated</th><th>Name</th><th>Status</th><th>Edit</th></tr>
</thead>
<tbody>
{rows}
</tbody>
</table>
</body>
</html>"#
    )
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use crate::app;
    use chrono::Utc;
    use sqlx::{Pool, Sqlite, SqlitePool};

    const WEB_USERNAME: &str = "admin";
    const WEB_PASSWORD: &str = "test-password";

    async fn start_app(pool: Pool<Sqlite>) -> SocketAddr {
        // Bind to an OS-assigned port and run the real server in the background,
        // so the test exercises the app over HTTP rather than calling handlers directly.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app(pool, WEB_PASSWORD))
                .await
                .unwrap();
        });

        address
    }

    #[sqlx::test]
    async fn get_feature_flags_returns_inserted_flag(pool: SqlitePool) {
        sqlx::query(
            "INSERT INTO feature_flags (name, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind("dark_mode")
        .bind(true)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&pool)
        .await
        .expect("inserting a feature flag should succeed");

        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/feature_flags"))
            .send()
            .await
            .expect("request to fetch feature flags should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let flags: serde_json::Value = response
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(
            flags.as_array().expect("response should be an array").len(),
            1
        );
        assert_eq!(flags[0]["name"], "dark_mode");
        assert_eq!(flags[0]["enabled"], true);
    }

    #[sqlx::test]
    async fn get_feature_flags_web_renders_table(pool: SqlitePool) {
        sqlx::query(
            "INSERT INTO feature_flags (name, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind("dark_mode")
        .bind(true)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&pool)
        .await
        .expect("inserting a feature flag should succeed");

        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/feature_flags/web"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .send()
            .await
            .expect("request to fetch feature flags web page should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.contains("text/html"))
        );

        let body = response
            .text()
            .await
            .expect("response should have a text body");

        assert!(body.contains("<table"));
        assert!(body.contains("dark_mode"));
        assert!(body.contains("Enabled"));
    }

    #[sqlx::test]
    async fn create_feature_flag_web_inserts_flag(pool: SqlitePool) {
        let addr = start_app(pool).await;
        // Disable redirect following so the 303 response can be asserted directly.
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let response = client
            .post(format!("http://{addr}/feature_flags/web"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .header("content-type", "application/x-www-form-urlencoded")
            .body("name=new_flag&enabled=true")
            .send()
            .await
            .expect("request to create feature flag should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::SEE_OTHER);

        let flags: serde_json::Value = client
            .get(format!("http://{addr}/feature_flags"))
            .send()
            .await
            .expect("request to fetch feature flags should succeed")
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(flags[0]["name"], "new_flag");
        assert_eq!(flags[0]["enabled"], true);
    }

    #[sqlx::test]
    async fn create_feature_flag_web_defaults_enabled_to_false(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        client
            .post(format!("http://{addr}/feature_flags/web"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .header("content-type", "application/x-www-form-urlencoded")
            .body("name=off_flag")
            .send()
            .await
            .expect("request to create feature flag should succeed");

        let flags: serde_json::Value = client
            .get(format!("http://{addr}/feature_flags"))
            .send()
            .await
            .expect("request to fetch feature flags should succeed")
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(flags[0]["name"], "off_flag");
        assert_eq!(flags[0]["enabled"], false);
    }

    #[sqlx::test]
    async fn update_feature_flag_web_updates_flag(pool: SqlitePool) {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO feature_flags (name, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?)
             RETURNING id",
        )
        .bind("dark_mode")
        .bind(true)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&pool)
        .await
        .expect("inserting a feature flag should succeed");

        let addr = start_app(pool).await;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let response = client
            .post(format!("http://{addr}/feature_flags/web/{id}"))
            .basic_auth(WEB_USERNAME, Some(WEB_PASSWORD))
            .header("content-type", "application/x-www-form-urlencoded")
            .body("name=light_mode")
            .send()
            .await
            .expect("request to update feature flag should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::SEE_OTHER);

        let flags: serde_json::Value = client
            .get(format!("http://{addr}/feature_flags"))
            .send()
            .await
            .expect("request to fetch feature flags should succeed")
            .json()
            .await
            .expect("response should be valid JSON");

        assert_eq!(flags[0]["name"], "light_mode");
        assert_eq!(flags[0]["enabled"], false);
    }

    #[sqlx::test]
    async fn web_route_without_credentials_is_unauthorized(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/feature_flags/web"))
            .send()
            .await
            .expect("request without credentials should complete");

        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
        assert!(
            response
                .headers()
                .get("www-authenticate")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.contains("Basic"))
        );
    }

    #[sqlx::test]
    async fn web_route_with_wrong_password_is_unauthorized(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/feature_flags/web"))
            .basic_auth(WEB_USERNAME, Some("wrong-password"))
            .send()
            .await
            .expect("request with wrong password should complete");

        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn json_route_does_not_require_credentials(pool: SqlitePool) {
        let addr = start_app(pool).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{addr}/feature_flags"))
            .send()
            .await
            .expect("request to fetch feature flags should succeed");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }
}
