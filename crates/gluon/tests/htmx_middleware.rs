#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

//! Integration tests that drive `htmx_middleware` through an actual
//! `axum::Router` + `Next` chain, complementing the in-crate unit tests that
//! exercise the flag bookkeeping in isolation.

use std::path::PathBuf;

use axum::{Router, middleware::from_fn, routing::get};
use axum_test::TestServer;
use gluon::AppError;
use gluon::View;
use gluon::middleware::{HtmxRequest, htmx::htmx_middleware};
use gluon::view::render_page_request;
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

#[tokio::test]
async fn boosted_request_keeps_the_htmx_extractor_flag() {
    let response = server()
        .get("/")
        .add_header("HX-Request", "true")
        .add_header("HX-Boosted", "true")
        .await;
    response.assert_status_ok();
    response.assert_text("true");
}

#[tokio::test]
async fn history_restore_keeps_the_htmx_extractor_flag() {
    let response = server()
        .get("/")
        .add_header("HX-Request", "true")
        .add_header("HX-History-Restore-Request", "true")
        .await;
    response.assert_status_ok();
    response.assert_text("true");
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

async fn missing_page() -> AppError {
    AppError::NotFound
}

fn view_server(template: PathBuf, layout: PathBuf, error_dir: PathBuf) -> TestServer {
    let page_layer = from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let template = template.clone();
            let layout = layout.clone();
            let error_dir = error_dir.clone();
            async move {
                render_page_request(
                    request,
                    next,
                    Some(template),
                    vec![layout.clone()],
                    error_dir,
                    Some(layout),
                )
                .await
            }
        },
    );
    let app = Router::new()
        .route("/", get(page))
        .route("/missing", get(missing_page))
        .layer(page_layer)
        .layer(from_fn(htmx_middleware));
    TestServer::new(app)
}

#[tokio::test]
async fn htmx_fragment_excludes_layout_but_boosted_and_history_restore_render_documents() {
    let dir = tempfile::tempdir().expect("tempdir");
    let template = dir.path().join("page.tsx");
    let layout = dir.path().join("layout.tsx");
    let error_dir = dir.path().join("_error");
    std::fs::write(
        &template,
        "export default function Page(props) { return <article>{props.message}</article>; }\n",
    )
    .expect("write template");
    std::fs::write(
        &layout,
        "export default function Layout(props) { return <main>layout:{props.children}</main>; }\n",
    )
    .expect("write layout");
    std::fs::create_dir_all(&error_dir).expect("create error directory");
    std::fs::write(
        error_dir.join("404.tsx"),
        "export default function NotFound(props) { return <p>{props.status}:{props.message}</p>; }\n",
    )
    .expect("write error template");
    let server = view_server(template, layout, error_dir);

    let document = server.get("/").await;
    document.assert_status_ok();
    assert!(
        document.text().contains("<!DOCTYPE html>"),
        "{}",
        document.text()
    );
    assert!(document.text().contains("layout:"), "{}", document.text());

    let fragment = server.get("/").add_header("HX-Request", "true").await;
    fragment.assert_status_ok();
    assert!(fragment.text().contains("fragment-marker"));
    assert!(!fragment.text().contains("layout:"), "{}", fragment.text());
    assert!(
        !fragment.text().contains("<!DOCTYPE html>"),
        "{}",
        fragment.text()
    );

    let error_fragment = server
        .get("/missing")
        .add_header("Accept", "text/html")
        .add_header("HX-Request", "true")
        .await;
    error_fragment.assert_status_not_found();
    assert!(!error_fragment.text().contains("layout:"));
    assert!(!error_fragment.text().contains("<!DOCTYPE html>"));

    for navigation_header in ["HX-Boosted", "HX-History-Restore-Request"] {
        let response = server
            .get("/")
            .add_header("HX-Request", "true")
            .add_header(navigation_header, "true")
            .await;
        response.assert_status_ok();
        assert!(response.text().contains("layout:"), "{}", response.text());
        assert!(
            response.text().contains("<!DOCTYPE html>"),
            "{}",
            response.text()
        );

        let error = server
            .get("/missing")
            .add_header("Accept", "text/html")
            .add_header("HX-Request", "true")
            .add_header(navigation_header, "true")
            .await;
        error.assert_status_not_found();
        assert!(error.text().contains("layout:"), "{}", error.text());
        assert!(error.text().contains("<!DOCTYPE html>"), "{}", error.text());
    }
}
