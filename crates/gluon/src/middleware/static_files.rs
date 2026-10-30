use std::collections::HashMap;
use std::ffi::OsStr;
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

/// Serves the generated Tailwind stylesheet.
///
/// Development requests regenerate the stylesheet from the current `app/`
/// sources and `[view].tailwind_content` globs; other requests use the build
/// output and can be cached immutably.
pub async fn serve_tailwind_css(
    embedded_css: &'static [u8],
    source_root: PathBuf,
    embedded_tailwind_content: &'static [&'static str],
) -> Response {
    let dev_mode = std::env::var_os("GLUON_DEV_TEMPLATES").as_deref() == Some(OsStr::new("1"));
    let body = if dev_mode {
        match tokio::task::spawn_blocking(move || {
            generate_tailwind_css(&source_root, embedded_tailwind_content)
        })
        .await
        {
            Ok(css) => Body::from(css),
            Err(_) => Body::from(embedded_css),
        }
    } else {
        Body::from(embedded_css)
    };
    let mut response = Response::new(body);
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/css"));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if dev_mode {
            "public, max-age=0, must-revalidate"
        } else {
            "public, max-age=31536000, immutable"
        }),
    );
    response
}

fn generate_tailwind_css(source_root: &FsPath, embedded_tailwind_content: &[&str]) -> String {
    let Ok(canonical_source_root) = source_root.canonicalize() else {
        return String::new();
    };
    let mut sources = Vec::new();
    let app_glob = source_root.join("app").join("**/*");
    append_glob_sources(
        &app_glob.to_string_lossy(),
        &mut sources,
        &["tsx", "jsx", "ts", "js"],
        &canonical_source_root,
    );

    if let Some(patterns) = runtime_tailwind_content(source_root) {
        for pattern in &patterns {
            let path_pattern = source_root.join(pattern);
            append_glob_sources(
                &path_pattern.to_string_lossy(),
                &mut sources,
                &[],
                &canonical_source_root,
            );
        }
    } else {
        for pattern in embedded_tailwind_content {
            let path_pattern = source_root.join(pattern);
            append_glob_sources(
                &path_pattern.to_string_lossy(),
                &mut sources,
                &[],
                &canonical_source_root,
            );
        }
    }

    encre_css::generate(
        sources.iter().map(String::as_str),
        &encre_css::Config::default(),
    )
}

fn runtime_tailwind_content(source_root: &FsPath) -> Option<Vec<String>> {
    let config_source = match std::fs::read_to_string(source_root.join("gluon.toml")) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(_) => return Some(Vec::new()),
    };
    let Ok(config) = toml::from_str::<toml::Value>(&config_source) else {
        return Some(Vec::new());
    };
    let patterns = config
        .get("view")
        .and_then(|view| view.get("tailwind_content"))
        .and_then(toml::Value::as_array);
    Some(
        patterns
            .map(|patterns| {
                patterns
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
    )
}

fn append_glob_sources(
    pattern: &str,
    sources: &mut Vec<String>,
    extensions: &[&str],
    canonical_root: &FsPath,
) {
    let Ok(paths) = glob::glob(pattern) else {
        return;
    };
    sources.extend(
        paths
            .filter_map(Result::ok)
            .filter(|path| {
                path.is_file()
                    && path
                        .canonicalize()
                        .is_ok_and(|canonical_path| canonical_path.starts_with(canonical_root))
                    && (extensions.is_empty()
                        || path.extension().is_some_and(|extension| {
                            extensions.iter().any(|candidate| extension == *candidate)
                        }))
            })
            .filter_map(|path| std::fs::read_to_string(path).ok()),
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use serial_test::serial;
    use tempfile::tempdir;

    struct RestoreDevTemplates(Option<std::ffi::OsString>);

    impl RestoreDevTemplates {
        fn set(value: Option<&str>) -> Self {
            let previous = std::env::var_os("GLUON_DEV_TEMPLATES");
            unsafe {
                match value {
                    Some(value) => std::env::set_var("GLUON_DEV_TEMPLATES", value),
                    None => std::env::remove_var("GLUON_DEV_TEMPLATES"),
                }
            }
            Self(previous)
        }
    }

    impl Drop for RestoreDevTemplates {
        fn drop(&mut self) {
            unsafe {
                match self.0.take() {
                    Some(value) => std::env::set_var("GLUON_DEV_TEMPLATES", value),
                    None => std::env::remove_var("GLUON_DEV_TEMPLATES"),
                }
            }
        }
    }

    async fn response_body(response: Response) -> Vec<u8> {
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec()
    }

    #[tokio::test]
    #[serial]
    async fn production_tailwind_css_uses_immutable_cache_headers() {
        let _restore = RestoreDevTemplates::set(None);
        let response = serve_tailwind_css(b".embedded { color: red; }", PathBuf::new(), &[]).await;

        assert_eq!(response.headers()[header::CONTENT_TYPE], "text/css");
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "public, max-age=31536000, immutable"
        );
        assert_eq!(
            response_body(response).await.as_slice(),
            b".embedded { color: red; }"
        );
    }

    #[tokio::test]
    #[serial]
    async fn development_tailwind_css_requires_revalidation() {
        let _restore = RestoreDevTemplates::set(Some("1"));
        let dir = tempdir().unwrap();
        let response = serve_tailwind_css(b"embedded", dir.path().to_owned(), &[]).await;

        assert_eq!(response.headers()[header::CONTENT_TYPE], "text/css");
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "public, max-age=0, must-revalidate"
        );
    }

    #[tokio::test]
    #[serial]
    async fn development_tailwind_css_regenerates_from_current_sources() {
        let _restore = RestoreDevTemplates::set(Some("1"));
        let dir = tempdir().unwrap();
        let app = dir.path().join("app");
        let content = dir.path().join("content");
        let embedded = dir.path().join("embedded");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::create_dir_all(&content).unwrap();
        std::fs::create_dir_all(&embedded).unwrap();
        std::fs::write(
            dir.path().join("gluon.toml"),
            "[view]\ntailwind_content = [\"content/**/*.html\"]\n",
        )
        .unwrap();
        std::fs::write(app.join("page.tsx"), r#"<div class="text-red-500"></div>"#).unwrap();
        std::fs::write(app.join("page.rs"), r#"let class = "bg-pink-700";"#).unwrap();
        std::fs::write(
            content.join("extra.html"),
            r#"<p class="bg-green-500"></p>"#,
        )
        .unwrap();
        std::fs::write(
            embedded.join("fallback.html"),
            r#"<p class="text-purple-500"></p>"#,
        )
        .unwrap();

        let first = response_body(
            serve_tailwind_css(b"embedded", dir.path().to_owned(), &["embedded/**/*.html"]).await,
        )
        .await;
        let first = String::from_utf8(first).unwrap();
        assert!(first.contains(".text-red-500"));
        assert!(first.contains(".bg-green-500"));
        assert!(!first.contains(".text-purple-500"));
        assert!(!first.contains(".bg-pink-700"));

        std::fs::write(app.join("page.tsx"), r#"<div class="text-blue-500"></div>"#).unwrap();
        std::fs::write(content.join("extra.html"), r#"<p class="bg-blue-500"></p>"#).unwrap();
        let second = response_body(
            serve_tailwind_css(b"embedded", dir.path().to_owned(), &["embedded/**/*.html"]).await,
        )
        .await;
        let second = String::from_utf8(second).unwrap();
        assert!(second.contains(".text-blue-500"));
        assert!(second.contains(".bg-blue-500"));
        assert!(!second.contains(".text-red-500"));
        assert!(!second.contains(".bg-green-500"));
    }

    #[tokio::test]
    #[serial]
    async fn development_tailwind_css_falls_back_to_embedded_globs() {
        let _restore = RestoreDevTemplates::set(Some("1"));
        let dir = tempdir().unwrap();
        let content = dir.path().join("compiled");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(
            content.join("fallback.html"),
            r#"<p class="text-yellow-500"></p>"#,
        )
        .unwrap();

        let body = response_body(
            serve_tailwind_css(b"embedded", dir.path().to_owned(), &["compiled/**/*.html"]).await,
        )
        .await;
        let body = String::from_utf8(body).unwrap();
        assert!(body.contains(".text-yellow-500"));
    }

    #[tokio::test]
    #[serial]
    async fn runtime_config_without_tailwind_content_disables_embedded_globs() {
        let _restore = RestoreDevTemplates::set(Some("1"));
        let dir = tempdir().unwrap();
        let content = dir.path().join("compiled");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(dir.path().join("gluon.toml"), "[view]\ntailwind = true\n").unwrap();
        std::fs::write(
            content.join("fallback.html"),
            r#"<p class="text-yellow-500"></p>"#,
        )
        .unwrap();

        let body = response_body(
            serve_tailwind_css(b"embedded", dir.path().to_owned(), &["compiled/**/*.html"]).await,
        )
        .await;
        let body = String::from_utf8(body).unwrap();
        assert!(!body.contains(".text-yellow-500"));
    }
    #[tokio::test]
    #[serial]
    async fn runtime_tailwind_content_cannot_read_outside_project_root() {
        let _restore = RestoreDevTemplates::set(Some("1"));
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(
            project.join("gluon.toml"),
            "[view]\ntailwind_content = [\"../outside/**/*.html\"]\n",
        )
        .unwrap();
        std::fs::write(
            outside.join("private.html"),
            r#"<p class="text-purple-500"></p>"#,
        )
        .unwrap();

        let body = response_body(serve_tailwind_css(b"embedded", project, &[]).await).await;
        let body = String::from_utf8(body).unwrap();
        assert!(!body.contains(".text-purple-500"), "{body}");
    }
}
