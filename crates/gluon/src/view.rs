//! View response type for rendering templated pages.
//!
//! `View<P>` resolves its template and inherited layouts from task-local values
//! scoped by generated page wrappers, which also supply the embedded source
//! map; `GLUON_DEV_TEMPLATES=1` selects live files from disk. Boot's outer
//! request middleware scopes application rendering options in `CURRENT_RENDER_CONFIG`.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Response};
use serde::Serialize;

use crate::error::{AppError, AppErrorResponse};

/// Internal error type used to wrap view rendering failures into
/// [`AppError::Internal`], which expects a boxed [`std::error::Error`].
#[derive(Debug)]
struct ViewError(String);

impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ViewError {}

/// A view response carrying serializable props and an optional template path.
pub struct View<P> {
    props: P,
    template: Option<PathBuf>,
}

impl<P: Serialize> View<P> {
    /// Construct a view from props alone. The template path is resolved by the
    /// build.rs-generated handler wrapper via the [`CURRENT_TEMPLATE`]
    /// task-local.
    pub fn new(props: P) -> Self {
        Self {
            props,
            template: None,
        }
    }

    /// Render the view as an HTML fragment without a document wrapper.
    ///
    /// This is used by Server-Sent Events, whose payloads are inserted into an
    /// existing document rather than returned as standalone pages.
    pub(crate) fn render_fragment(self) -> crate::Result<String> {
        self.render(true)
    }

    fn render(self, fragment: bool) -> crate::Result<String> {
        let template = self
            .template
            .or_else(|| CURRENT_TEMPLATE.try_with(Clone::clone).ok().flatten())
            .ok_or_else(|| {
                AppError::Internal(Box::new(ViewError(
                    "View has no template path; cannot render".to_string(),
                )))
            })?;

        let props = serde_json::to_value(&self.props).map_err(|err| {
            AppError::Internal(Box::new(ViewError(format!(
                "failed to serialize props: {err}"
            ))))
        })?;

        let mut config = CURRENT_RENDER_CONFIG
            .try_with(Clone::clone)
            .unwrap_or_default();
        config.base_dir = template.parent().map(std::path::Path::to_path_buf);
        config.fragment = fragment;
        let layouts = if fragment {
            Vec::new()
        } else {
            CURRENT_LAYOUTS.try_with(Clone::clone).unwrap_or_default()
        };

        let context = CURRENT_SOURCE_MAP
            .try_with(Clone::clone)
            .ok()
            .flatten()
            .map_or_else(jsxrs::CheckedRenderContext::new, |source_map| {
                jsxrs::CheckedRenderContext::new().with_source_map(source_map)
            });

        jsxrs::render_with_layouts_and_context(&template, &layouts, &props, &config, &context)
            .map_err(|err| {
                AppError::Internal(Box::new(ViewError(format!(
                    "failed to render template: {err}"
                ))))
            })
    }
}

tokio::task_local! {
    /// Template path injected by the generated page handler wrapper.
    ///
    /// Each page handler enters this scope so `View::new(props)` resolves the
    /// matching TSX file without requiring an explicit path.
    pub static CURRENT_TEMPLATE: Option<PathBuf>;
    /// Layout paths for the current page, ordered from the app root inward;
    /// scoped alongside `CURRENT_TEMPLATE` by the generated page wrapper.
    pub static CURRENT_LAYOUTS: Vec<PathBuf>;
    /// Embedded TSX sources for the current request, absent in development mode.
    pub static CURRENT_SOURCE_MAP: Option<Arc<jsxrs::SourceMap>>;
    /// Rendering options scoped by Boot's outer request middleware.
    pub static CURRENT_RENDER_CONFIG: jsxrs::RenderConfig;
}

/// Route-specific rendering inputs supplied by the generated application router.
#[doc(hidden)]
pub struct PageRenderContext {
    /// Page template path relative to the application manifest directory.
    pub template: Option<PathBuf>,
    /// Ancestor layouts ordered from the app root inward.
    pub layouts: Vec<PathBuf>,
    /// Directory containing `_error/<status>.tsx` templates.
    pub error_dir: PathBuf,
    /// Optional root layout used when rendering page errors.
    pub root_layout: Option<PathBuf>,
    /// Compile-time TSX source set, absent when development disk mode is active.
    pub source_map: Arc<jsxrs::SourceMap>,
    /// Application manifest directory used by development disk mode.
    pub source_root: PathBuf,
}

/// Runs a generated page handler with its template and inherited layout context.
pub async fn render_page_request(
    request: Request,
    next: Next,
    context: PageRenderContext,
) -> Response {
    let PageRenderContext {
        template,
        layouts,
        error_dir,
        root_layout,
        source_map,
        source_root,
    } = context;
    let is_htmx = crate::middleware::htmx::is_htmx_fragment(request.headers());
    let html_accepted = accepts_html(request.headers());
    let dev_mode = dev_templates_enabled();
    let template = template.map(|path| resolve_template_path(path, &source_root, dev_mode));
    let layouts = layouts
        .into_iter()
        .map(|path| resolve_template_path(path, &source_root, dev_mode))
        .collect();
    let error_dir = resolve_template_path(error_dir, &source_root, dev_mode);
    let root_layout = root_layout.map(|path| resolve_template_path(path, &source_root, dev_mode));
    let source_map = (!dev_mode).then_some(source_map);
    let scoped_layouts = if is_htmx { Vec::new() } else { layouts };
    let response = crate::middleware::htmx::CURRENT_HTMX_REQUEST
        .scope(
            is_htmx,
            CURRENT_SOURCE_MAP.scope(
                source_map.clone(),
                CURRENT_TEMPLATE.scope(
                    template,
                    CURRENT_LAYOUTS.scope(scoped_layouts, next.run(request)),
                ),
            ),
        )
        .await;
    render_error_response(
        response,
        &error_dir,
        root_layout.as_deref().filter(|_| !is_htmx),
        html_accepted,
        is_htmx,
        source_map.as_ref(),
    )
}

/// Runs a generated route handler with its sibling TSX template, when present.
///
/// Unlike page routes, a route template does not inherit page layouts or render
/// `AppError` responses through `_error` templates.
pub async fn render_route_request(
    request: Request,
    next: Next,
    template: PathBuf,
    source_map: Arc<jsxrs::SourceMap>,
    source_root: PathBuf,
) -> Response {
    let dev_mode = dev_templates_enabled();
    let template = resolve_template_path(template, &source_root, dev_mode);
    let source_map = (!dev_mode).then_some(source_map);
    CURRENT_SOURCE_MAP
        .scope(
            source_map,
            CURRENT_TEMPLATE.scope(
                Some(template),
                CURRENT_LAYOUTS.scope(Vec::new(), next.run(request)),
            ),
        )
        .await
}

fn dev_templates_enabled() -> bool {
    std::env::var_os("GLUON_DEV_TEMPLATES").as_deref() == Some(std::ffi::OsStr::new("1"))
}

fn resolve_template_path(path: PathBuf, source_root: &Path, dev_mode: bool) -> PathBuf {
    if dev_mode {
        source_root.join(path)
    } else {
        path
    }
}

fn accepts_html(headers: &HeaderMap) -> bool {
    let mut html = None;
    let mut text_wildcard = None;
    let mut wildcard = None;
    for value in &headers.get_all(header::ACCEPT) {
        let Ok(value) = value.to_str() else {
            continue;
        };
        for item in value.split(',') {
            let mut parts = item.split(';');
            let Some(media_type) = parts.next().map(str::trim) else {
                continue;
            };
            let mut quality = 1.0;
            for parameter in parts {
                if let Some((name, value)) = parameter.trim().split_once('=')
                    && name.trim().eq_ignore_ascii_case("q")
                {
                    quality = value.trim().parse::<f32>().unwrap_or(0.0);
                }
            }
            let accepted = quality > 0.0;
            let slot = if media_type.eq_ignore_ascii_case("text/html") {
                &mut html
            } else if media_type.eq_ignore_ascii_case("text/*") {
                &mut text_wildcard
            } else if media_type.eq_ignore_ascii_case("*/*") {
                &mut wildcard
            } else {
                continue;
            };
            *slot = Some(slot.as_ref().copied().unwrap_or(false) || accepted);
        }
    }
    html.or(text_wildcard).or(wildcard).unwrap_or(false)
}

fn render_error_response(
    mut response: Response,
    error_dir: &Path,
    root_layout: Option<&Path>,
    html_accepted: bool,
    fragment: bool,
    source_map: Option<&Arc<jsxrs::SourceMap>>,
) -> Response {
    let Some(error) = response.extensions().get::<AppErrorResponse>() else {
        return response;
    };
    let status = response.status();
    let message = error.message.clone();
    add_vary_headers(
        &mut response,
        &[
            "Accept",
            "HX-Request",
            "HX-Boosted",
            "HX-History-Restore-Request",
        ],
    );
    if !html_accepted
        || status == StatusCode::UNPROCESSABLE_ENTITY
        || !(status.is_client_error() || status.is_server_error())
    {
        return response;
    }
    let requested = error_dir.join(format!("{}.tsx", status.as_u16()));
    let fallback = error_dir.join("500.tsx");
    let props = serde_json::json!({
        "status": status.as_u16(),
        "message": message,
    });
    let layouts = root_layout
        .into_iter()
        .map(Path::to_path_buf)
        .collect::<Vec<_>>();
    let mut config = CURRENT_RENDER_CONFIG
        .try_with(Clone::clone)
        .unwrap_or_default();
    config.fragment = fragment;
    let context = source_map.map_or_else(jsxrs::CheckedRenderContext::new, |source_map| {
        jsxrs::CheckedRenderContext::new().with_source_map(Arc::clone(source_map))
    });
    let html = if source_map.is_some() {
        match jsxrs::render_with_layouts_and_context(
            &requested, &layouts, &props, &config, &context,
        ) {
            Ok(html) => html,
            Err(jsxrs::JsxrsError::FileNotFound(path))
                if path == requested && status != StatusCode::INTERNAL_SERVER_ERROR =>
            {
                let Ok(html) = jsxrs::render_with_layouts_and_context(
                    &fallback, &layouts, &props, &config, &context,
                ) else {
                    return response;
                };
                html
            }
            Err(_) => return response,
        }
    } else {
        let template = if requested.is_file() {
            requested
        } else if status != StatusCode::INTERNAL_SERVER_ERROR && fallback.is_file() {
            fallback
        } else {
            return response;
        };
        config.base_dir = template.parent().map(Path::to_path_buf);
        let Ok(html) =
            jsxrs::render_with_layouts_and_context(&template, &layouts, &props, &config, &context)
        else {
            return response;
        };
        html
    };
    let (mut parts, _) = response.into_parts();
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    Response::from_parts(parts, Body::from(html))
}

fn add_vary_headers(response: &mut Response, names: &[&'static str]) {
    for &name in names {
        let present = response
            .headers()
            .get_all(header::VARY)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(','))
            .any(|value| value.trim().eq_ignore_ascii_case(name));
        if !present {
            response
                .headers_mut()
                .append(header::VARY, HeaderValue::from_static(name));
        }
    }
}

impl<P: Serialize> IntoResponse for View<P> {
    fn into_response(self) -> Response {
        let fragment = crate::middleware::htmx::CURRENT_HTMX_REQUEST
            .try_with(|is_htmx| *is_htmx)
            .unwrap_or(false);

        match self.render(fragment) {
            Ok(html) => {
                let mut response = Html(html).into_response();
                add_vary_headers(
                    &mut response,
                    &["HX-Request", "HX-Boosted", "HX-History-Restore-Request"],
                );
                response
            }
            Err(error) => error.into_response(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use http::StatusCode;
    use serde::Serialize;
    use std::io::Write;
    use std::path::Path;
    use tempfile::tempdir;

    #[derive(Serialize)]
    struct Greeting {
        greeting: &'static str,
    }

    fn write_template(path: &Path, body: &str) -> PathBuf {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(body.as_bytes()).unwrap();
        path.to_path_buf()
    }

    fn write_min_page(dir: &Path, body: &str) -> PathBuf {
        write_template(&dir.join("page.tsx"), body)
    }

    const MIN_PAGE: &str =
        "export default function Page(props) { return <main>{props.greeting}</main>; }\n";

    async fn body_string(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn assert_vary_headers(response: &Response, names: &[&str]) {
        for name in names {
            let count = response
                .headers()
                .get_all(header::VARY)
                .iter()
                .filter_map(|value| value.to_str().ok())
                .flat_map(|value| value.split(','))
                .filter(|value| value.trim().eq_ignore_ascii_case(name))
                .count();
            assert_eq!(count, 1, "expected one Vary token for {name}");
        }
    }

    fn assert_error_vary_headers(response: &Response) {
        assert_vary_headers(
            response,
            &[
                "Accept",
                "HX-Request",
                "HX-Boosted",
                "HX-History-Restore-Request",
            ],
        );
    }

    async fn render_with_config(template: PathBuf, config: jsxrs::RenderConfig) -> String {
        CURRENT_TEMPLATE
            .scope(
                Some(template),
                CURRENT_RENDER_CONFIG.scope(config, async {
                    body_string(View::new(Greeting { greeting: "hello" }).into_response()).await
                }),
            )
            .await
    }

    #[tokio::test]
    async fn pretty_render_config_changes_document_formatting() {
        let dir = tempdir().unwrap();
        let template = write_min_page(dir.path(), MIN_PAGE);
        let compact = render_with_config(template.clone(), jsxrs::RenderConfig::default()).await;
        let pretty = render_with_config(
            template,
            jsxrs::RenderConfig {
                pretty: true,
                ..jsxrs::RenderConfig::default()
            },
        )
        .await;

        assert!(!compact.contains('\n'), "compact output: {compact}");
        assert!(pretty.contains('\n'), "pretty output: {pretty}");
    }

    #[tokio::test]
    async fn tailwind_render_config_injects_generated_css() {
        let dir = tempdir().unwrap();
        let template = write_min_page(
            dir.path(),
            r#"export default function Page(props) { return <main className="bg-blue-500 text-white p-4">{props.greeting}</main>; }"#,
        );
        let html = render_with_config(
            template,
            jsxrs::RenderConfig {
                tailwind: true,
                ..jsxrs::RenderConfig::default()
            },
        )
        .await;

        assert!(
            html.contains("<style>"),
            "generated stylesheet missing: {html}"
        );
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
    async fn template_missing_returns_500() {
        // No CURRENT_TEMPLATE scope and no explicit template -> 500.
        let response = View::new(Greeting { greeting: "hi" }).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn task_local_template_renders_html() {
        let dir = tempdir().unwrap();
        let template = write_min_page(dir.path(), MIN_PAGE);

        let response = CURRENT_TEMPLATE
            .scope(Some(template), async move {
                View::new(Greeting { greeting: "hello" }).into_response()
            })
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_vary_headers(
            &response,
            &["HX-Request", "HX-Boosted", "HX-History-Restore-Request"],
        );
        let body = body_string(response).await;
        assert!(
            body.contains("hello"),
            "expected body to contain 'hello', got: {body}"
        );
    }

    #[tokio::test]
    async fn embedded_views_render_relative_components_after_source_removal() {
        const PAGE: &str = r#"import Greeting from "./components/greeting"; export default function Page(props) { return <main><Greeting label={props.greeting} /></main>; }"#;
        const COMPONENT: &str =
            "export default function Greeting(props) { return <strong>{props.label}</strong>; }";
        let dir = tempdir().unwrap();
        write_template(&dir.path().join("app/page.tsx"), PAGE);
        write_template(&dir.path().join("app/components/greeting.tsx"), COMPONENT);
        std::fs::remove_dir_all(dir.path().join("app")).unwrap();

        let source_map = Arc::new(jsxrs::SourceMap::new(vec![
            ("app/page.tsx", PAGE),
            ("app/components/greeting.tsx", COMPONENT),
        ]));
        let response = CURRENT_SOURCE_MAP
            .scope(
                Some(source_map),
                CURRENT_TEMPLATE.scope(Some(PathBuf::from("app/page.tsx")), async {
                    View::new(Greeting {
                        greeting: "embedded",
                    })
                    .into_response()
                }),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            body_string(response)
                .await
                .contains("<strong>embedded</strong>")
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn dev_mode_reads_live_template_files() {
        let dir = tempdir().unwrap();
        let live_template = "export default function Page() { return <p>live from disk</p>; }";
        write_template(&dir.path().join("app/page.tsx"), live_template);
        let previous = std::env::var_os("GLUON_DEV_TEMPLATES");
        unsafe {
            std::env::set_var("GLUON_DEV_TEMPLATES", "1");
        }
        let _restore = RestoreDevTemplates(previous);
        let source_map = Arc::new(jsxrs::SourceMap::new(vec![(
            "app/page.tsx",
            "export default function Page() { return <p>embedded source</p>; }",
        )]));
        let root = dir.path().to_path_buf();
        let middleware_source_map = Arc::clone(&source_map);
        let app = axum::Router::new()
            .route(
                "/",
                axum::routing::get(|| async {
                    View::new(Greeting { greeting: "unused" }).into_response()
                }),
            )
            .layer(axum::middleware::from_fn(move |request, next| {
                let middleware_source_map = Arc::clone(&middleware_source_map);
                let source_root = root.clone();
                async move {
                    render_page_request(
                        request,
                        next,
                        PageRenderContext {
                            template: Some(PathBuf::from("app/page.tsx")),
                            layouts: Vec::new(),
                            error_dir: PathBuf::from("app/_error"),
                            root_layout: None,
                            source_map: middleware_source_map,
                            source_root,
                        },
                    )
                    .await
                }
            }));
        let response = tower::ServiceExt::oneshot(
            app,
            axum::http::Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_string(response).await.contains("live from disk"));
    }

    #[tokio::test]
    async fn invalid_template_path_returns_500() {
        let bogus = PathBuf::from("/definitely/does/not/exist/page.tsx");
        let response = CURRENT_TEMPLATE
            .scope(Some(bogus), async move {
                View::new(Greeting { greeting: "hi" }).into_response()
            })
            .await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn task_local_isolation() {
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();
        // Each template embeds a literal marker so we can tell which file was
        // rendered by inspecting the response body.
        let page_a = write_min_page(
            dir_a.path(),
            "export default function Page(props) { return <main>A:{props.greeting}</main>; }\n",
        );
        let page_b = write_min_page(
            dir_b.path(),
            "export default function Page(props) { return <main>B:{props.greeting}</main>; }\n",
        );

        let task_a = tokio::spawn(CURRENT_TEMPLATE.scope(Some(page_a), async move {
            let response = View::new(Greeting { greeting: "alpha" }).into_response();
            let status = response.status();
            let body = body_string(response).await;
            (status, body)
        }));
        let task_b = tokio::spawn(CURRENT_TEMPLATE.scope(Some(page_b), async move {
            let response = View::new(Greeting { greeting: "beta" }).into_response();
            let status = response.status();
            let body = body_string(response).await;
            (status, body)
        }));

        let (status_a, body_a) = task_a.await.unwrap();
        let (status_b, body_b) = task_b.await.unwrap();

        assert_eq!(status_a, StatusCode::OK);
        assert_eq!(status_b, StatusCode::OK);
        // Each task must see only its own template's marker, never the other's.
        assert!(body_a.contains("A:alpha"), "task A body: {body_a}");
        assert!(!body_a.contains("B:"), "task A leaked B marker: {body_a}");
        assert!(body_b.contains("B:beta"), "task B body: {body_b}");
        assert!(!body_b.contains("A:"), "task B leaked A marker: {body_b}");
    }

    #[tokio::test]
    async fn embedded_error_templates_support_status_and_fallback_lookup() {
        let status_templates = Arc::new(jsxrs::SourceMap::new(vec![
            (
                "app/layout.tsx",
                "export default function Root(props) { return <main>{props.children}</main>; }",
            ),
            (
                "app/_error/404.tsx",
                "export default function NotFound(props) { return <p>status {props.status}</p>; }",
            ),
            (
                "app/_error/500.tsx",
                "export default function Error(props) { return <p>fallback {props.status}</p>; }",
            ),
        ]));
        let response = render_error_response(
            AppError::NotFound.into_response(),
            Path::new("app/_error"),
            Some(Path::new("app/layout.tsx")),
            true,
            false,
            Some(&status_templates),
        );
        assert!(body_string(response).await.contains("status 404"));

        let fallback_templates = Arc::new(jsxrs::SourceMap::new(vec![(
            "app/_error/500.tsx",
            "export default function Error(props) { return <p>fallback {props.status}</p>; }",
        )]));
        let response = render_error_response(
            AppError::NotFound.into_response(),
            Path::new("app/_error"),
            None,
            true,
            false,
            Some(&fallback_templates),
        );
        assert!(body_string(response).await.contains("fallback 404"));
    }

    #[tokio::test]
    async fn nested_layouts_merge_head_and_keep_render_options() {
        let dir = tempdir().unwrap();
        let root = write_template(
            &dir.path().join("layout.tsx"),
            "export default function Root(props) { return <><Head><meta name=\"root\" content=\"yes\" /></Head><main>root({props.greeting}){props.children}</main></>; }",
        );
        let nested = write_template(
            &dir.path().join("(shop)/layout.tsx"),
            "export default function Nested(props) { return <><Head><meta name=\"nested\" content=\"yes\" /></Head><section>nested({props.greeting}){props.children}</section></>; }",
        );
        let page = write_template(
            &dir.path().join("(shop)/page.tsx"),
            "export default function Page(props) { return <><Head><title>Page title</title><meta name=\"page\" content=\"yes\" /></Head><article className=\"bg-blue-500\">{props.greeting}</article></>; }",
        );
        let config = jsxrs::RenderConfig {
            pretty: true,
            tailwind: true,
            ..jsxrs::RenderConfig::default()
        };
        let response = CURRENT_RENDER_CONFIG
            .scope(
                config,
                CURRENT_TEMPLATE.scope(
                    Some(page),
                    CURRENT_LAYOUTS.scope(vec![root, nested], async {
                        View::new(Greeting { greeting: "hello" }).into_response()
                    }),
                ),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        let root_pos = body.find("root(hello)").unwrap();
        let nested_pos = body.find("nested(hello)").unwrap();
        let page_pos = body.find("hello</article>").unwrap();
        assert!(root_pos < nested_pos && nested_pos < page_pos, "{body}");
        assert!(body.contains("<title>Page title</title>"), "{body}");
        assert!(body.contains("name=\"root\""), "{body}");
        assert!(body.contains("name=\"nested\""), "{body}");
        assert!(body.contains("name=\"page\""), "{body}");
        assert!(body.contains("<style>"), "{body}");
        assert!(body.contains("bg-blue-500"), "{body}");
        assert!(
            body.starts_with("<!DOCTYPE html>\n<html>\n"),
            "expected pretty HTML: {body}"
        );
    }

    #[tokio::test]
    async fn layout_preserves_non_object_page_props() {
        let dir = tempdir().unwrap();
        let layout = write_template(
            &dir.path().join("layout.tsx"),
            "export default function Layout(props) { return <main>{props.pageProps}{props.children}</main>; }",
        );
        let page = write_min_page(
            dir.path(),
            "export default function Page() { return <p>page</p>; }",
        );
        let response = CURRENT_TEMPLATE
            .scope(
                Some(page),
                CURRENT_LAYOUTS.scope(vec![layout], async {
                    View::new("kept under pageProps").into_response()
                }),
            )
            .await;
        let body = body_string(response).await;
        assert!(body.contains("kept under pageProps"), "{body}");
        assert!(body.contains("<p>page</p>"), "{body}");
    }

    #[tokio::test]
    async fn htmx_view_renders_a_layout_free_fragment() {
        let dir = tempdir().unwrap();
        let page = write_min_page(
            dir.path(),
            "export default function Page() { return <p>fragment</p>; }",
        );
        let response = crate::middleware::htmx::CURRENT_HTMX_REQUEST
            .scope(
                true,
                CURRENT_TEMPLATE.scope(
                    Some(page),
                    CURRENT_LAYOUTS.scope(Vec::new(), async {
                        View::new(Greeting { greeting: "unused" }).into_response()
                    }),
                ),
            )
            .await;
        let body = body_string(response).await;
        assert!(body.contains("<p>fragment</p>"), "{body}");
        assert!(!body.contains("layout:"), "{body}");
        assert!(!body.contains("<!DOCTYPE html>"), "{body}");
    }

    #[tokio::test]
    async fn html_app_errors_render_templates_through_only_the_root_layout() {
        let dir = tempdir().unwrap();
        let error_dir = dir.path().join("_error");
        let root = write_template(
            &dir.path().join("layout.tsx"),
            "export default function Root(props) { return <main>root:{props.children}</main>; }",
        );
        write_template(
            &dir.path().join("(nested)/layout.tsx"),
            "export default function Nested(props) { return <section>nested:{props.children}</section>; }",
        );
        write_template(
            &error_dir.join("404.tsx"),
            "export default function NotFound(props) { return <p>{props.status}:{props.message}</p>; }",
        );
        let response = render_error_response(
            AppError::NotFound.into_response(),
            &error_dir,
            Some(&root),
            true,
            false,
            None,
        );
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert_error_vary_headers(&response);
        let body = body_string(response).await;
        assert!(body.contains("root:"), "{body}");
        assert!(body.contains("404:resource not found"), "{body}");
        assert!(!body.contains("nested:"), "{body}");
    }

    #[tokio::test]
    async fn app_errors_keep_plain_or_json_bodies_when_html_is_not_accepted() {
        let dir = tempdir().unwrap();
        let plain_error = AppError::NotFound.into_response();
        let response = render_error_response(plain_error, dir.path(), None, false, false, None);
        assert_error_vary_headers(&response);
        assert_ne!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert!(body_string(response).await.contains("resource not found"));

        let response = render_error_response(
            AppError::Validation(vec![]).into_response(),
            dir.path(),
            None,
            true,
            false,
            None,
        );
        assert_error_vary_headers(&response);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        assert_eq!(body_string(response).await, "[]");
    }

    #[tokio::test]
    async fn missing_error_template_falls_back_to_500_template() {
        let dir = tempdir().unwrap();
        let error_dir = dir.path().join("_error");
        write_template(
            &error_dir.join("500.tsx"),
            "export default function Error(props) { return <p>fallback {props.status}</p>; }",
        );
        let response = render_error_response(
            AppError::NotFound.into_response(),
            &error_dir,
            None,
            true,
            false,
            None,
        );
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_error_vary_headers(&response);
        assert!(body_string(response).await.contains("fallback 404"));
    }

    #[tokio::test]
    async fn missing_error_templates_keep_plain_response_and_vary_headers() {
        let dir = tempdir().unwrap();
        let response = render_error_response(
            AppError::NotFound.into_response(),
            &dir.path().join("_error"),
            None,
            true,
            false,
            None,
        );
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_error_vary_headers(&response);
        assert_ne!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert!(body_string(response).await.contains("resource not found"));
    }

    #[tokio::test]
    async fn failed_error_template_render_keeps_plain_response_and_vary_headers() {
        let dir = tempdir().unwrap();
        let error_dir = dir.path().join("_error");
        write_template(&error_dir.join("404.tsx"), "not valid TSX");
        let response = render_error_response(
            AppError::NotFound.into_response(),
            &error_dir,
            None,
            true,
            false,
            None,
        );
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_error_vary_headers(&response);
        assert_ne!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert!(body_string(response).await.contains("resource not found"));
    }

    #[test]
    fn html_accept_negotiation_respects_specific_quality() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            "application/json, */*;q=0.5".parse().unwrap(),
        );
        assert!(accepts_html(&headers));
        headers.insert(header::ACCEPT, "text/html;q=0, */*;q=1".parse().unwrap());
        assert!(!accepts_html(&headers));
        headers.insert(header::ACCEPT, "application/json".parse().unwrap());
        assert!(!accepts_html(&headers));
    }
}
