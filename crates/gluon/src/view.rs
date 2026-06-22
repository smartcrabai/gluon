//! View response type for rendering templated pages.
//!
//! `View<P>` carries serializable props and an optional template path that the
//! jsxrs integration uses to render the response body.

use std::fmt;
use std::path::PathBuf;

use axum::response::{Html, IntoResponse, Response};
use serde::Serialize;

use crate::error::AppError;

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
    /// Template path injected by the build.rs-generated handler wrapper.
    ///
    /// Each request enters a `CURRENT_TEMPLATE.scope(Some(path), handler)`
    /// block so that `View::<P>::into_response` can resolve its template from
    /// the surrounding task even when the handler returns `View::new(props)`
    /// without an explicit path.
    pub static CURRENT_TEMPLATE: Option<PathBuf>;
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

        let config = jsxrs::RenderConfig {
            base_dir: template.parent().map(std::path::Path::to_path_buf),
            ..jsxrs::RenderConfig::default()
        };

        match jsxrs::render_file(&template, &props, &config) {
            Ok(html) => Html(html).into_response(),
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

    /// Write a minimal JSX page that renders `props.greeting` into a `<main>`
    /// element. Returns the path of the written file.
    fn write_min_page(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("page.tsx");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(body.as_bytes()).unwrap();
        path
    }

    const MIN_PAGE: &str =
        "export default function Page(props) { return <main>{props.greeting}</main>; }\n";

    async fn body_string(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
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
}
