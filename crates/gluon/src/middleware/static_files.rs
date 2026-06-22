use std::path::PathBuf;

use tower_http::services::ServeDir;

/// Creates a [`ServeDir`] service hardened for serving the application's
/// `public/` directory under a route such as `/public`.
///
/// Disables directory index responses so that the served file set is exactly
/// what the caller placed on disk. Note that `tower-http 0.6` does not expose
/// a knob to disable symlink traversal -- operators must avoid placing
/// untrusted symlinks under `public/`.
#[must_use]
pub fn service(public_dir: PathBuf) -> ServeDir {
    ServeDir::new(public_dir).append_index_html_on_directories(false)
}
