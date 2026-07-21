#[cfg(test)]
use crate::{app, app::env::Env};
#[cfg(test)]
use sqlx::{Pool, Sqlite};
#[cfg(test)]
use std::net::SocketAddr;

#[cfg(test)]
pub const WEB_USERNAME: &str = "admin";
#[cfg(test)]
pub const WEB_PASSWORD: &str = "test-password";
#[cfg(test)]
pub const SENTRY_DSN: &str = "test-dsn";
#[cfg(test)]
pub const HTTP_PORT: u16 = 0;

#[cfg(test)]
pub async fn start_app(pool: Pool<Sqlite>) -> SocketAddr {
    // Bind to an OS-assigned port and run the real server in the background,
    // so the test exercises the app over HTTP rather than calling handlers directly.
    let aws_config = aws_config::load_from_env().await;

    let env = Env {
        feature_flags_web_password: WEB_PASSWORD.into(),
        aws_config,
        s3_bucket: "test-bucket".into(),
        http_port: HTTP_PORT,
        sentry_dsn: SENTRY_DSN.into(),
    };
    let address = format!("127.0.0.1:{HTTP_PORT}");
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app(pool, &env)).await.unwrap();
    });

    address
}
