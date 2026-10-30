//! View response type for rendering templated pages.
//!
//! `View<P>` resolves its template and inherited layouts from task-local values
//! scoped by generated page wrappers. Boot's outer `ViewConfig` middleware
//! scopes application rendering options in `CURRENT_RENDER_CONFIG`.

use std::fmt;
use std::path::{Path, PathBuf};

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
    /// Rendering options scoped by Boot's outer `ViewConfig` middleware.
    pub static CURRENT_RENDER_CONFIG: jsxrs::RenderConfig;
}

/// Runs a generated page handler with its template and inherited layout context.
pub async fn render_page_request(
    request: Request,
    next: Next,
    template: Option<PathBuf>,
    layouts: Vec<PathBuf>,
    error_dir: PathBuf,
    root_layout: Option<PathBuf>,
) -> Response {
    let is_htmx = crate::middleware::htmx::is_htmx_fragment(request.headers());
    let html_accepted = accepts_html(request.headers());
    let layouts = if is_htmx { Vec::new() } else { layouts };
    let response = crate::middleware::htmx::CURRENT_HTMX_REQUEST
        .scope(
            is_htmx,
            CURRENT_TEMPLATE.scope(template, CURRENT_LAYOUTS.scope(layouts, next.run(request))),
        )
        .await;
    render_error_response(
        response,
        &error_dir,
        root_layout.as_deref().filter(|_| !is_htmx),
        html_accepted,
        is_htmx,
    )
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
) -> Response {
    let Some(error) = response.extensions().get::<AppErrorResponse>() else {
        return response;
    };
    let status = response.status();
    let message = error.message.clone();
    add_error_vary_headers(&mut response);
    if !html_accepted
        || status == StatusCode::UNPROCESSABLE_ENTITY
        || !(status.is_client_error() || status.is_server_error())
    {
        return response;
    }
    let template = error_dir.join(format!("{}.tsx", status.as_u16()));
    let template = if template.is_file() {
        template
    } else {
        let fallback = error_dir.join("500.tsx");
        if !fallback.is_file() {
            return response;
        }
        fallback
    };
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
    config.base_dir = template.parent().map(Path::to_path_buf);
    config.fragment = fragment;
    let Ok(html) = jsxrs::render_with_layouts(&template, &layouts, &props, &config) else {
        return response;
    };
    let (mut parts, _) = response.into_parts();
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    Response::from_parts(parts, Body::from(html))
}

fn add_error_vary_headers(response: &mut Response) {
    for name in ["Accept", "HX-Request"] {
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
        let template = self
            .template
            .or_else(|| CURRENT_TEMPLATE.try_with(Clone::clone).ok().flatten());
        let Some(template) = template else {
            return AppError::Internal(Box::new(ViewError(
                "View has no template path; cannot render".to_string(),
            )))
            .into_response();
        };

        let props = match serde_json::to_value(&self.props) {
            Ok(value) => value,
            Err(err) => {
                return AppError::Internal(Box::new(ViewError(format!(
                    "failed to serialize props: {err}"
                ))))
                .into_response();
            }
        };
        let mut config = CURRENT_RENDER_CONFIG
            .try_with(Clone::clone)
            .unwrap_or_default();
        config.base_dir = template.parent().map(Path::to_path_buf);
        config.fragment = crate::middleware::htmx::CURRENT_HTMX_REQUEST
            .try_with(|is_htmx| *is_htmx)
            .unwrap_or(false);
        let layouts = CURRENT_LAYOUTS.try_with(Clone::clone).unwrap_or_default();

        match jsxrs::render_with_layouts(&template, &layouts, &props, &config) {
            Ok(html) => {
                let mut response = Html(html).into_response();
                response
                    .headers_mut()
                    .append(header::VARY, HeaderValue::from_static("HX-Request"));
                response
            }
            Err(err) => AppError::Internal(Box::new(ViewError(format!(
                "failed to render template: {err}"
            ))))
            .into_response(),
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

    fn assert_error_vary_headers(response: &Response) {
        for name in ["Accept", "HX-Request"] {
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
        let body = body_string(response).await;
        assert!(
            body.contains("hello"),
            "expected body to contain 'hello', got: {body}"
        );
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
        let response = render_error_response(plain_error, dir.path(), None, false, false);
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
