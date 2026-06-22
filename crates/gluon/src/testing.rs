//! Test helpers for gluon applications.

use std::sync::Arc;

use axum::Router;
use axum_test::TestServer;

use crate::{Container, ContainerBuilder};

/// A wrapper around [`axum_test::TestServer`] that hosts the application
/// router with the supplied [`Container`].
pub struct TestClient {
    server: TestServer,
}

impl TestClient {
    /// Creates a test client from a router and container.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying [`axum_test::TestServer`] cannot
    /// be initialised.
    pub fn new(router: Router<Arc<Container>>, container: Container) -> anyhow::Result<Self> {
        let app = router.with_state(Arc::new(container));
        let server = TestServer::new(app)?;
        Ok(Self { server })
    }

    /// Returns a reference to the underlying [`TestServer`].
    #[must_use]
    pub fn server(&self) -> &TestServer {
        &self.server
    }
}

/// Builds an empty [`ContainerBuilder`] suitable for tests; production code
/// should configure bindings as needed.
#[must_use]
pub fn container() -> ContainerBuilder {
    ContainerBuilder::new()
}
