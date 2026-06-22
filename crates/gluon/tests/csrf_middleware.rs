//! Integration tests for [`gluon::middleware::csrf`].
//!
//! These tests wire `csrf_middleware` behind a `SessionManagerLayer` backed by
//! an in-memory store, then drive the resulting router through `axum-test`'s
//! `TestServer`. The handler echoes the CSRF token (on GET) or the raw request
//! body (on POST) so we can both observe the token issued to a fresh session
//! and confirm the middleware restores the body verbatim before calling the
//! inner handler.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::Extension;
use axum::body::Bytes;
use axum::http::{Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, options, post};
use axum::{Router, middleware};
use axum_test::TestServer;
use gluon::middleware::CsrfToken;
use gluon::middleware::csrf::csrf_middleware;
use tower_sessions::cookie::SameSite;
use tower_sessions::{MemoryStore, SessionManagerLayer};

const SESSION_COOKIE_NAME: &str = "gluon_sid";

/// Returns the CSRF token from the request extensions in the response body.
async fn show_token(Extension(token): Extension<CsrfToken>) -> String {
    token.as_str().to_owned()
}

/// Echoes the raw request body back to the caller so tests can assert that
/// the middleware reconstructs the body byte-for-byte before invoking the
/// handler.
async fn echo_body(body: Bytes) -> impl IntoResponse {
    body
}

/// Replies to `OPTIONS /` with `204 No Content` so we can observe that the
/// CSRF middleware passes the safe method straight through.
async fn options_handler() -> StatusCode {
    StatusCode::NO_CONTENT
}

/// Builds the router under test. POST `/` and POST `/echo` both echo the body
/// so each scenario can choose which path to drive.
fn build_app() -> Router {
    let session_layer = SessionManagerLayer::new(MemoryStore::default())
        .with_name(SESSION_COOKIE_NAME)
        .with_same_site(SameSite::Lax)
        .with_http_only(true)
        .with_secure(false);

    Router::new()
        .route(
            "/",
            get(show_token)
                .post(echo_body)
                .merge(options(options_handler)),
        )
        .route("/echo", post(echo_body))
        .layer(middleware::from_fn(csrf_middleware))
        .layer(session_layer)
}

fn server() -> TestServer {
    let mut server = TestServer::new(build_app()).unwrap();
    server.save_cookies();
    server
}

#[tokio::test]
async fn get_passes_through_and_sets_token() {
    let server = server();

    let response = server.get("/").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let token = response.text();
    assert_eq!(
        token.len(),
        64,
        "csrf token should be 32 random bytes encoded as hex"
    );
    assert!(
        token.chars().all(|c| c.is_ascii_hexdigit()),
        "csrf token should be lowercase hex: {token}"
    );

    // The session cookie must be set on first contact so subsequent requests
    // can be linked back to the same server-side token.
    let cookies = response.cookies();
    assert!(
        cookies.get(SESSION_COOKIE_NAME).is_some(),
        "expected {SESSION_COOKIE_NAME} cookie to be set on first response"
    );
}

#[tokio::test]
async fn same_session_reuses_token() {
    let server = server();

    let first = server.get("/").await.text();
    let second = server.get("/").await.text();

    assert_eq!(
        first, second,
        "csrf token must be stable across requests in the same session"
    );
}

#[tokio::test]
async fn post_without_token_is_forbidden() {
    let server = server();

    // Prime the session so a token exists server-side -- we want the 403 to
    // come from the missing submitted token, not from a missing session
    // token.
    let _ = server.get("/").await;

    let response = server
        .post("/")
        .bytes(Bytes::from_static(b"payload=hello"))
        .content_type("application/x-www-form-urlencoded")
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn post_with_valid_form_token_passes() {
    let server = server();

    let token = server.get("/").await.text();
    let body = format!("_csrf={token}&payload=hello");

    let response = server
        .post("/")
        .bytes(Bytes::from(body.clone()))
        .content_type("application/x-www-form-urlencoded")
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(
        response.text(),
        body,
        "handler must observe the original form body byte-for-byte"
    );
}

#[tokio::test]
async fn post_with_valid_header_token_passes() {
    let server = server();

    let token = server.get("/").await.text();
    let raw_body: &[u8] = br#"{"x":1}"#;

    let response = server
        .post("/echo")
        .bytes(Bytes::from_static(raw_body))
        .content_type("application/json")
        .add_header("x-csrf-token", token)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(
        response.as_bytes().as_ref(),
        raw_body,
        "JSON body must reach the handler exactly as sent"
    );
}

#[tokio::test]
async fn post_with_mismatched_token_is_forbidden() {
    let server = server();

    let _ = server.get("/").await;

    let response = server
        .post("/")
        .bytes(Bytes::from_static(b"_csrf=wrong-token&payload=hello"))
        .content_type("application/x-www-form-urlencoded")
        .await;

    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn post_exceeding_max_body_returns_413() {
    let server = server();

    let _ = server.get("/").await;

    // The middleware buffers up to MAX_REQUEST_BODY_BYTES (4 MiB) inclusive,
    // so a body of MAX + 1 bytes must trip the 413 path.
    let mut body = Vec::with_capacity(4 * 1024 * 1024 + 16);
    body.extend_from_slice(b"payload=");
    body.resize(4 * 1024 * 1024 + 1, b'a');

    let response = server
        .post("/")
        .bytes(Bytes::from(body))
        .content_type("application/x-www-form-urlencoded")
        .await;

    assert_eq!(response.status_code(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn head_and_options_are_safe() {
    let server = server();

    let head = server.method(Method::HEAD, "/").await;
    assert!(
        head.status_code().is_success(),
        "HEAD / should pass through the csrf middleware, got {}",
        head.status_code()
    );

    let options = server.method(Method::OPTIONS, "/").await;
    let status = options.status_code();
    assert!(
        status.is_success() || status == StatusCode::NO_CONTENT,
        "OPTIONS / should be treated as a safe method, got {status}"
    );
}
