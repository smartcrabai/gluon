#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

use std::{path::PathBuf, time::Duration};

use axum::{Router, middleware::from_fn, response::IntoResponse, routing::get};
use futures_util::{StreamExt, stream};
use gluon::{
    ContainerBuilder, Result, View,
    middleware::{csrf::csrf_middleware, htmx::htmx_middleware},
    prelude::sse,
    testing::TestClient,
    view::CURRENT_TEMPLATE,
};
use http::header;
use serde::Serialize;
use tower_sessions::{MemoryStore, SessionManagerLayer};

#[derive(Serialize)]
struct EventProps {
    message: &'static str,
}

async fn events() -> Result<impl IntoResponse> {
    let updates = stream::iter([
        ("query-progress", "Query started"),
        ("query-complete", "Query finished"),
    ])
    .then(|(event, message)| async move {
        tokio::time::sleep(Duration::from_millis(25)).await;
        sse::view_event(event, View::new(EventProps { message }))
    });

    Ok(sse::Sse::new(updates).keep_alive(
        sse::KeepAlive::new()
            .interval(Duration::from_millis(5))
            .text("sse-test-alive"),
    ))
}

fn client(template: PathBuf) -> TestClient {
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
        .route("/events", get(events).layer(template_layer))
        .layer(from_fn(htmx_middleware))
        .layer(from_fn(csrf_middleware))
        .layer(
            SessionManagerLayer::new(MemoryStore::default())
                .with_name("gluon_sid")
                .with_secure(false),
        );
    TestClient::new(app, ContainerBuilder::new().build()).expect("test client")
}

#[tokio::test]
async fn stream_delivers_named_fragment_events_keepalive_and_no_buffer_header() {
    let dir = tempfile::tempdir().expect("tempdir");
    let template = dir.path().join("route.tsx");
    std::fs::write(
        &template,
        "export default function Event(props) { return <p>{props.message}</p>; }\n",
    )
    .expect("write TSX template");
    let client = client(template);

    let response = client.server().get("/events").await;
    response.assert_status_ok();
    assert!(
        response
            .header(header::CONTENT_TYPE)
            .to_str()
            .expect("valid content type")
            .starts_with("text/event-stream")
    );
    assert_eq!(response.header("x-accel-buffering"), "no");

    let body = response.text();
    assert!(body.contains("event: query-progress\n"), "{body}");
    assert!(body.contains("data: <p>Query started</p>"), "{body}");
    assert!(body.contains("event: query-complete\n"), "{body}");
    assert!(body.contains("data: <p>Query finished</p>"), "{body}");
    assert!(body.contains(": sse-test-alive\n\n"), "{body}");
    assert!(!body.contains("<!DOCTYPE html>"), "{body}");
}
