#[cfg(test)]
use crate::{app, env::Env};
#[cfg(test)]
use sqlx::{Pool, Sqlite};
#[cfg(test)]
use std::net::SocketAddr;

#[cfg(test)]
pub const WEB_USERNAME: &str = "admin";
#[cfg(test)]
pub const WEB_PASSWORD: &str = "test-password";

#[cfg(test)]
pub async fn start_app(pool: Pool<Sqlite>) -> SocketAddr {
    // Bind to an OS-assigned port and run the real server in the background,
    // so the test exercises the app over HTTP rather than calling handlers directly.
    let env = Env {
        feature_flags_web_password: WEB_PASSWORD.into(),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app(pool, &env)).await.unwrap();
    });

    address
}
