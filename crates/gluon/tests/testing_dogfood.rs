#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

//! Dogfood test for `gluon::testing::TestClient` and `gluon::testing::container`.

use std::any::Any;
use std::sync::Arc;

use axum::{Router, routing::get};
use gluon::{Container, ContainerBuilder, testing};

async fn ping() -> &'static str {
    "pong"
}

#[tokio::test]
async fn test_client_serves_basic_route() {
    let router: Router<Arc<Container>> = Router::new().route("/ping", get(ping));
    let container = ContainerBuilder::new().build();

    let client = testing::TestClient::new(router, container).expect("client boots");
    let response = client.server().get("/ping").await;
    response.assert_status_ok();
    response.assert_text("pong");
}

#[tokio::test]
async fn container_helper_returns_empty_builder() {
    let container = testing::container().build();
    // Nothing bound -> `try_resolve` should be `None` for any arbitrary type.
    assert!(container.try_resolve::<dyn Any + Send + Sync>().is_none());
}
