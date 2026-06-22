#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

//! Integration tests that drive `htmx_middleware` through an actual
//! `axum::Router` + `Next` chain, complementing the in-crate unit tests that
//! exercise the flag bookkeeping in isolation.

use axum::{Router, middleware::from_fn, routing::get};
use axum_test::TestServer;
use gluon::middleware::HtmxRequest;
use gluon::middleware::htmx::htmx_middleware;

async fn handler(htmx: HtmxRequest) -> String {
    htmx.is_htmx.to_string()
}

fn server() -> TestServer {
    let app = Router::new()
        .route("/", get(handler))
        .layer(from_fn(htmx_middleware));
    TestServer::new(app).expect("test server boots")
}

#[tokio::test]
async fn flag_is_false_without_header() {
    let response = server().get("/").await;
    response.assert_status_ok();
    response.assert_text("false");
}

#[tokio::test]
async fn flag_is_true_with_header_present() {
    let response = server().get("/").add_header("HX-Request", "true").await;
    response.assert_status_ok();
    response.assert_text("true");
}

#[tokio::test]
async fn flag_is_true_case_insensitive() {
    // `HeaderMap::contains_key` is case-insensitive, so an oddly-cased header
    // name must still be recognised by the middleware.
    let response = server().get("/").add_header("Hx-Request", "true").await;
    response.assert_status_ok();
    response.assert_text("true");
}
