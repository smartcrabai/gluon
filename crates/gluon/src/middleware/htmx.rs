use axum::{
    body::Body,
    extract::{FromRequestParts, Request},
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use std::convert::Infallible;

tokio::task_local! {
    pub(crate) static CURRENT_HTMX_REQUEST: bool;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HtmxRequest {
    pub is_htmx: bool,
}

pub async fn htmx_middleware(mut request: Request<Body>, next: Next) -> Response {
    let is_htmx = request
        .headers()
        .get("hx-request")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("true"));
    request.extensions_mut().insert(HtmxRequest { is_htmx });
    CURRENT_HTMX_REQUEST.scope(is_htmx, next.run(request)).await
}

impl<S: Send + Sync> FromRequestParts<S> for HtmxRequest {
    type Rejection = Infallible;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> {
        std::future::ready(Ok(parts
            .extensions
            .get::<HtmxRequest>()
            .copied()
            .unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request as HttpRequest;

    /// Mimics the body of `htmx_middleware` without constructing a real
    /// `Next` (which is awkward in axum 0.8 without a layered Tower service).
    fn observed_flag(headers: &[(&str, &str)]) -> bool {
        let mut builder = HttpRequest::builder().uri("/");
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        let mut request = builder.body(Body::empty()).expect("build request");

        let is_htmx = request
            .headers()
            .get("hx-request")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.eq_ignore_ascii_case("true"));
        request.extensions_mut().insert(HtmxRequest { is_htmx });

        request
            .extensions()
            .get::<HtmxRequest>()
            .copied()
            .unwrap_or_default()
            .is_htmx
    }

    #[test]
    fn flag_is_true_when_header_present() {
        assert!(observed_flag(&[("HX-Request", "true")]));
    }

    #[test]
    fn flag_is_false_when_header_absent() {
        assert!(!observed_flag(&[]));
    }

    #[test]
    fn flag_is_false_when_header_value_is_false() {
        assert!(!observed_flag(&[("HX-Request", "false")]));
    }

    #[tokio::test]
    async fn extractor_defaults_when_middleware_was_skipped() {
        // No middleware -> no extension -> extractor returns default.
        let mut parts = HttpRequest::builder()
            .uri("/")
            .body(())
            .expect("build request")
            .into_parts()
            .0;
        let extracted: HtmxRequest = HtmxRequest::from_request_parts(&mut parts, &())
            .await
            .expect("extractor is Infallible");
        assert!(!extracted.is_htmx);
    }
}
