use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Router, routing::get};
use tower::ServiceExt;
use tower_http::services::ServeFile;

/// Creates a static-file service hardened for serving the application's
/// `public/` directory under a route such as `/public`.
///
/// Canonical path containment prevents symlinks or traversal from exposing
/// files outside `public/`. Directory index responses remain disabled.
pub fn service(public_dir: PathBuf) -> Router {
    Router::new()
        .route("/{*path}", get(serve_file))
        .with_state(Arc::new(public_dir))
}

async fn serve_file(
    State(root): State<Arc<PathBuf>>,
    Path(path): Path<String>,
    request: Request,
) -> Response {
    let Ok(root) = tokio::fs::canonicalize(root.as_ref()).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(file) = tokio::fs::canonicalize(root.join(path)).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(metadata) = tokio::fs::metadata(&file).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !file.starts_with(&root) || !metadata.is_file() {
        return StatusCode::NOT_FOUND.into_response();
    }

    ServeFile::new(file)
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {})
        .map(Body::new)
}
