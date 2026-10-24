//! Plan section 5.4: drive the OAuth HTTP bridge against a real local server.
#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{Router, body::Body, response::IntoResponse, routing::get};
use gluon::auth::oauth::{OAuthHttpClient, ReqwestOAuthClient};
use http::{Request, StatusCode};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

struct Server {
    base: String,
    task: tokio::task::JoinHandle<()>,
    redirected_requests: Arc<AtomicUsize>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn start() -> Self {
        let redirected_requests = Arc::new(AtomicUsize::new(0));
        let counter = redirected_requests.clone();
        let app = Router::new()
            .route(
                "/redirect",
                get(|| async {
                    (
                        StatusCode::FOUND,
                        [("location", "/target")],
                        "redirect response",
                    )
                        .into_response()
                }),
            )
            .route(
                "/target",
                get(move || {
                    let counter = counter.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        "must not be fetched"
                    }
                }),
            )
            .route("/exact", get(|| async { Body::from(vec![b'x'; 1024]) }))
            .route("/oversized", get(|| async { Body::from(vec![b'x'; 1025]) }))
            .route(
                "/never",
                get(|| async { std::future::pending::<String>().await }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            base,
            task,
            redirected_requests,
        }
    }
    fn request(&self, path: &str) -> Request<Vec<u8>> {
        Request::builder()
            .uri(format!("{}{path}", self.base))
            .body(vec![])
            .unwrap()
    }
}

#[tokio::test]
async fn oauth_http_redirect_is_not_followed() {
    let server = Server::start().await;
    let client = ReqwestOAuthClient::new(Duration::from_secs(2), 1024);
    let result = client.execute(server.request("/redirect")).await;
    // The adapter may return the redirect for the protocol layer to reject, or
    // reject it itself. Following it and returning a successful 200 is forbidden.
    if let Ok(response) = result {
        assert!(response.status().is_redirection());
    }
    assert_eq!(server.redirected_requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn oauth_http_response_limit_allows_exact_boundary_but_rejects_one_byte_more() {
    let server = Server::start().await;
    let client = ReqwestOAuthClient::new(Duration::from_secs(2), 1024);
    let exact = client.execute(server.request("/exact")).await.unwrap();
    assert_eq!(exact.status(), StatusCode::OK);
    assert_eq!(exact.body().len(), 1024);
    assert!(client.execute(server.request("/oversized")).await.is_err());
}

#[tokio::test]
async fn oauth_http_timeout_bounds_a_nonresponding_provider() {
    let server = Server::start().await;
    let client = ReqwestOAuthClient::new(Duration::from_millis(50), 1024);
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        client.execute(server.request("/never")),
    )
    .await
    .expect("OAuth HTTP client must enforce its configured timeout");
    assert!(result.is_err());
}
