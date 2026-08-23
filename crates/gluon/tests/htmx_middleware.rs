#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

//! Integration tests that drive `htmx_middleware` through an actual
//! `axum::Router` + `Next` chain, complementing the in-crate unit tests that
//! exercise the flag bookkeeping in isolation.

use std::path::PathBuf;

use axum::{Router, middleware::from_fn, routing::get};
use axum_test::TestServer;
use gluon::View;
use gluon::middleware::HtmxRequest;
use gluon::middleware::htmx::htmx_middleware;
use gluon::view::CURRENT_TEMPLATE;
use http::header;
use serde::Serialize;

async fn handler(htmx: HtmxRequest) -> String {
    htmx.is_htmx.to_string()
}

fn server() -> TestServer {
    let app = Router::new()
        .route("/", get(handler))
        .layer(from_fn(htmx_middleware));
    TestServer::new(app)
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

#[tokio::test]
async fn flag_is_false_with_false_header_value() {
    let response = server().get("/").add_header("HX-Request", "false").await;
    response.assert_status_ok();
    response.assert_text("false");
}

#[derive(Serialize)]
struct PageProps {
    message: &'static str,
}

async fn page() -> View<PageProps> {
    View::new(PageProps {
        message: "fragment-marker",
    })
}

fn view_server(template: PathBuf) -> TestServer {
    let template_layer = from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let template = template.clone();
            async move {
                CURRENT_TEMPLATE
                    .scope(Some(template), next.run(request))
                    .await
            }
        },
    );
    let app = Router::new()
        .route("/", get(page).layer(template_layer))
        .layer(from_fn(htmx_middleware));
    TestServer::new(app)
}

#[tokio::test]
async fn htmx_view_returns_fragment_while_normal_request_returns_document() {
    let dir = tempfile::tempdir().expect("tempdir");
    let template = dir.path().join("page.tsx");
    std::fs::write(
        &template,
        "export default function Page(props) { return <main>{props.message}</main>; }\n",
    )
    .expect("write template");
    let server = view_server(template);

    let document = server.get("/").await;
    document.assert_status_ok();
    assert_eq!(document.header(header::VARY), "HX-Request");
    assert!(
        document.text().contains("<!DOCTYPE html>"),
        "{}",
        document.text()
    );

    let fragment = server.get("/").add_header("HX-Request", "true").await;
    fragment.assert_status_ok();
    assert_eq!(fragment.header(header::VARY), "HX-Request");
    assert!(fragment.text().contains("fragment-marker"));
    assert!(
        !fragment.text().contains("<!DOCTYPE html>"),
        "{}",
        fragment.text()
    );
}
