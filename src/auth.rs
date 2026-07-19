use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderValue, Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use subtle::ConstantTimeEq;

// Username paired with the configured web password for HTTP Basic Auth.
const WEB_AUTH_USERNAME: &str = "admin";

// Rejects HTML admin requests that do not present the configured Basic Auth
// password, so the feature-flag management UI stays private.
pub async fn require_web_password(
    State(password): State<Arc<str>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if is_authorized(request.headers().get(header::AUTHORIZATION), &password) {
        next.run(request).await
    } else {
        unauthorized()
    }
}

fn is_authorized(header: Option<&HeaderValue>, password: &str) -> bool {
    let Some(value) = header.and_then(|value| value.to_str().ok()) else {
        return false;
    };
    let Some(encoded) = value.strip_prefix("Basic ") else {
        return false;
    };
    let Ok(decoded) = STANDARD.decode(encoded) else {
        return false;
    };
    let Ok(credentials) = String::from_utf8(decoded) else {
        return false;
    };
    let Some((user, pass)) = credentials.split_once(':') else {
        return false;
    };

    // Constant-time comparison avoids leaking the secret through response timing.
    let user_ok = user.as_bytes().ct_eq(WEB_AUTH_USERNAME.as_bytes());
    let pass_ok = pass.as_bytes().ct_eq(password.as_bytes());
    (user_ok & pass_ok).into()
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Basic")],
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn basic_header(user: &str, pass: &str) -> HeaderValue {
        let encoded = STANDARD.encode(format!("{user}:{pass}"));
        HeaderValue::from_str(&format!("Basic {encoded}")).unwrap()
    }

    #[test]
    fn accepts_matching_credentials() {
        let header = basic_header(WEB_AUTH_USERNAME, "secret");
        assert!(is_authorized(Some(&header), "secret"));
    }

    #[test]
    fn rejects_wrong_password() {
        let header = basic_header(WEB_AUTH_USERNAME, "wrong");
        assert!(!is_authorized(Some(&header), "secret"));
    }

    #[test]
    fn rejects_wrong_username() {
        let header = basic_header("root", "secret");
        assert!(!is_authorized(Some(&header), "secret"));
    }

    #[test]
    fn rejects_missing_header() {
        assert!(!is_authorized(None, "secret"));
    }

    #[test]
    fn rejects_non_basic_scheme() {
        let header = HeaderValue::from_static("Bearer token");
        assert!(!is_authorized(Some(&header), "secret"));
    }

    #[test]
    fn rejects_malformed_base64() {
        let header = HeaderValue::from_static("Basic not-base64!!");
        assert!(!is_authorized(Some(&header), "secret"));
    }
}
