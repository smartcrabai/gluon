use std::collections::HashMap;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Router, routing::get};
use tower::ServiceExt;
use tower_http::services::ServeFile;

type EmbeddedAssets = HashMap<&'static str, (&'static [u8], HeaderValue)>;

/// Creates the default static-file service from compile-time embedded assets.
///
/// Asset keys are exact paths relative to `public/`; directories and unknown
/// paths do not resolve to index files.
pub fn embedded_service(assets: &'static [(&'static str, &'static [u8])]) -> Router {
    let assets = assets
        .iter()
        .map(|(path, contents)| (*path, (*contents, content_type(FsPath::new(path)))))
        .collect::<HashMap<_, _>>();
    Router::new()
        .route("/{*path}", get(serve_embedded))
        .with_state(Arc::new(assets))
}

/// Creates the development disk service for `public/`.
///
/// Canonical path containment prevents symlinks or traversal from exposing
/// files outside `public/`. Directory index responses remain disabled.
pub fn service(public_dir: PathBuf) -> Router {
    Router::new()
        .route("/{*path}", get(serve_file))
        .with_state(Arc::new(public_dir))
}

async fn serve_embedded(
    State(assets): State<Arc<EmbeddedAssets>>,
    Path(path): Path<String>,
) -> Response {
    let Some((contents, content_type)) = assets.get(path.as_str()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut response = Response::new(Body::from(*contents));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type.clone());
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );
    response
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

    let mut response = ServeFile::new(&file)
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {})
        .map(Body::new);
    add_asset_headers(&mut response, &file, "public, max-age=0, must-revalidate");
    response
}

fn content_type(path: &FsPath) -> HeaderValue {
    HeaderValue::from_str(mime_guess::from_path(path).first_or_octet_stream().as_ref())
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"))
}

fn add_asset_headers(response: &mut Response, path: &FsPath, cache_control: &'static str) {
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type(path));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
}
