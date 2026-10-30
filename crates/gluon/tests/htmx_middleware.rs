#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

//! Integration tests that drive `htmx_middleware` through an actual
//! `axum::Router` + `Next` chain, complementing the in-crate unit tests that
//! exercise the flag bookkeeping in isolation.

use std::path::PathBuf;
use std::sync::Arc;

use axum::{Router, middleware::from_fn, routing::get};
use axum_test::TestServer;
use gluon::AppError;
use gluon::View;
use gluon::middleware::{HtmxRequest, htmx::htmx_middleware};
use gluon::view::{PageRenderContext, render_page_request};
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

const PAGE_SOURCE: &str =
    "export default function Page(props) { return <article>{props.message}</article>; }\n";
const LAYOUT_SOURCE: &str =
    "export default function Layout(props) { return <main>layout:{props.children}</main>; }\n";
const ERROR_SOURCE: &str =
    "export default function NotFound(props) { return <p>{props.status}:{props.message}</p>; }\n";

fn view_server(source_root: PathBuf) -> TestServer {
    let template = PathBuf::from("app/page.tsx");
    let layout = PathBuf::from("app/layout.tsx");
    let error_dir = PathBuf::from("app/_error");
    let source_map = Arc::new(jsxrs::SourceMap::new(vec![
        ("app/page.tsx", PAGE_SOURCE),
        ("app/layout.tsx", LAYOUT_SOURCE),
        ("app/_error/404.tsx", ERROR_SOURCE),
    ]));
    let page_layer = from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let template = template.clone();
            let layout = layout.clone();
            let error_dir = error_dir.clone();
            let source_map = Arc::clone(&source_map);
            let source_root = source_root.clone();
            async move {
                render_page_request(
                    request,
                    next,
                    PageRenderContext {
                        template: Some(template),
                        layouts: vec![layout.clone()],
                        error_dir,
                        root_layout: Some(layout),
                        source_map,
                        source_root,
                        stylesheet_url: None,
                    },
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
    std::fs::create_dir_all(dir.path().join("app/_error")).expect("create app sources");
    std::fs::write(dir.path().join("app/page.tsx"), PAGE_SOURCE).expect("write template");
    std::fs::write(dir.path().join("app/layout.tsx"), LAYOUT_SOURCE).expect("write layout");
    std::fs::write(dir.path().join("app/_error/404.tsx"), ERROR_SOURCE)
        .expect("write error template");
    let server = view_server(dir.path().to_path_buf());

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
