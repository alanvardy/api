use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app()).await.unwrap();
}

fn app() -> Router {
    Router::new().route("/", get(|| async { "Hello, World!" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    // Verifies the server can bind a port, accept a connection, and respond
    // to a request, confirming the whole stack actually runs end to end.
    #[tokio::test]
    async fn server_runs_and_responds() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind listener");
        let addr = listener.local_addr().expect("failed to get local addr");

        tokio::spawn(async move {
            axum::serve(listener, app()).await.unwrap();
        });

        let mut stream = TcpStream::connect(addr)
            .await
            .expect("failed to connect to server");
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .expect("failed to send request");

        let mut response = String::new();
        let mut buf = [0u8; 1024];
        loop {
            let n = stream
                .read(&mut buf)
                .await
                .expect("failed to read response");
            if n == 0 {
                break;
            }
            response.push_str(&String::from_utf8_lossy(&buf[..n]));
        }

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("Hello, World!"));
    }
}
