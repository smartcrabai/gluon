#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

use std::{path::PathBuf, sync::Arc, time::Duration};

use axum::{Router, middleware::from_fn, response::IntoResponse, routing::get};
use futures_util::{StreamExt, stream};
use gluon::{
    ContainerBuilder, Result, View,
    middleware::{csrf::csrf_middleware, htmx::htmx_middleware},
    prelude::sse,
    testing::TestClient,
    view::{
        CURRENT_RENDER_CONFIG, CURRENT_SOURCE_MAP, CURRENT_TAILWIND_STYLESHEET, CURRENT_TEMPLATE,
    },
};
use http::header;
use serde::Serialize;
use tower_sessions::{MemoryStore, SessionManagerLayer};

#[derive(Serialize)]
struct EventProps {
    message: String,
}

async fn events() -> Result<impl IntoResponse> {
    let updates = stream::iter([
        ("query-progress", "Query started"),
        ("query-complete", "Query finished"),
    ])
    .then(|(event, message)| async move {
        tokio::time::sleep(Duration::from_millis(25)).await;
        let stylesheet_enabled = CURRENT_TAILWIND_STYLESHEET
            .try_with(Option::is_some)
            .unwrap_or(false);
        sse::view_event(
            event,
            View::new(EventProps {
                message: format!("{message} (stylesheet={stylesheet_enabled})"),
            }),
        )
    });

    Ok(sse::Sse::new(updates).keep_alive(
        sse::KeepAlive::new()
            .interval(Duration::from_millis(5))
            .text("sse-test-alive"),
    ))
}

fn client(template: PathBuf, source_map: Arc<jsxrs::SourceMap>) -> TestClient {
    let template_layer = from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let template = template.clone();
            let source_map = Arc::clone(&source_map);
            async move {
                CURRENT_TAILWIND_STYLESHEET
                    .scope(
                        Some("/_gluon/app.test.css"),
                        CURRENT_TEMPLATE.scope(
                            Some(template),
                            CURRENT_SOURCE_MAP.scope(
                                Some(source_map),
                                CURRENT_RENDER_CONFIG.scope(
                                    jsxrs::RenderConfig {
                                        tailwind: true,
                                        ..jsxrs::RenderConfig::default()
                                    },
                                    next.run(request),
                                ),
                            ),
                        ),
                    )
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

struct RestoreDevTemplates(Option<std::ffi::OsString>);

impl Drop for RestoreDevTemplates {
    fn drop(&mut self) {
        unsafe {
            if let Some(value) = self.0.take() {
                std::env::set_var("GLUON_DEV_TEMPLATES", value);
            } else {
                std::env::remove_var("GLUON_DEV_TEMPLATES");
            }
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn stream_delivers_named_fragment_events_keepalive_and_no_buffer_header() {
    let previous = std::env::var_os("GLUON_DEV_TEMPLATES");
    unsafe {
        std::env::set_var("GLUON_DEV_TEMPLATES", "0");
    }
    let _restore = RestoreDevTemplates(previous);
    let template = PathBuf::from("__gluon_embedded_sse_test__/route.tsx");
    let template_source = "export default function Event(props) { return <p className=\"bg-blue-500\">{props.message}</p>; }\n";
    let source_map = Arc::new(jsxrs::SourceMap::new(vec![(
        "__gluon_embedded_sse_test__/route.tsx",
        template_source,
    )]));
    let client = client(template, source_map);

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
    assert!(
        body.contains("data: <p class=\"bg-blue-500\">Query started (stylesheet=true)</p>"),
        "{body}"
    );
    assert!(
        body.contains("data: <p class=\"bg-blue-500\">Query finished (stylesheet=true)</p>"),
        "{body}"
    );
    assert!(body.contains(": sse-test-alive\n\n"), "{body}");
    assert!(!body.contains("<!DOCTYPE html>"), "{body}");
    assert!(!body.contains("<style>"), "{body}");
    assert!(!body.contains("/_gluon/app.test.css"), "{body}");
}
