#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

//! Dogfood test for `gluon::testing::TestClient` and `gluon::testing::container`.

use std::any::Any;
use std::sync::Arc;

use axum::{Router, routing::get};
use gluon::{Container, ContainerBuilder, Inject, testing};

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

trait MissingDependency: Send + Sync {}

async fn missing_dependency(_dependency: Inject<dyn MissingDependency>) -> &'static str {
    "unreachable"
}

#[tokio::test]
async fn missing_injected_binding_returns_500_without_panicking() {
    let router: Router<Arc<Container>> = Router::new().route("/missing", get(missing_dependency));
    let client =
        testing::TestClient::new(router, ContainerBuilder::new().build()).expect("client boots");

    let response = client.server().get("/missing").await;
    response.assert_status_internal_server_error();
    response.assert_text("internal server error");
}
