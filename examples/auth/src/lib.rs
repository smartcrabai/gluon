//! Shared screenless application assembly used by the executable and process tests.
use axum::{Router, routing::get};
use gluon::{Boot, auth::AuthConfig};

/// Run the example with explicitly supplied authentication adapters.
///
/// # Errors
/// Returns startup, configuration, database or server errors from Boot.
pub async fn run(config: AuthConfig) -> gluon::Result<()> {
    Boot::new()
        .with_auth(config)
        .with_router(Router::new().route("/health", get(|| async { "sample-auth" })))
        .run()
        .await
}
