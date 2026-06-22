//! HTTP redirect response helpers.

use axum::http::header::LOCATION;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::error::AppError;

/// A redirect response. Wraps a status code and a `Location` header.
///
/// Use [`Redirect::to`] for a temporary `303 See Other` redirect or
/// [`Redirect::permanent`] for a `301 Moved Permanently` redirect.
pub struct Redirect {
    status: StatusCode,
    location: String,
}

impl Redirect {
    /// Constructs a `303 See Other` redirect to the given URL. Use this after
    /// processing a form `POST` to redirect the browser to a `GET` page.
    #[must_use]
    pub fn to(url: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SEE_OTHER,
            location: url.into(),
        }
    }

    /// Constructs a `301 Moved Permanently` redirect to the given URL.
    #[must_use]
    pub fn permanent(url: impl Into<String>) -> Self {
        Self {
            status: StatusCode::MOVED_PERMANENTLY,
            location: url.into(),
        }
    }
}

impl IntoResponse for Redirect {
    fn into_response(self) -> Response {
        match HeaderValue::from_str(&self.location) {
            Ok(header) => {
                let mut response = self.status.into_response();
                response.headers_mut().insert(LOCATION, header);
                response
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    location = %self.location,
                    "invalid redirect Location header"
                );
                AppError::Internal(Box::new(err)).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_sets_303_and_location() {
        let response = Redirect::to("/dashboard").into_response();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(LOCATION).unwrap().to_str().unwrap(),
            "/dashboard"
        );
    }

    #[test]
    fn permanent_sets_301() {
        let response = Redirect::permanent("/new").into_response();
        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(
            response.headers().get(LOCATION).unwrap().to_str().unwrap(),
            "/new"
        );
    }

    #[test]
    fn invalid_location_becomes_500_internal_error() {
        // Embedded newlines are rejected by `HeaderValue::from_str`.
        let response = Redirect::to("/oops\r\nX-Smuggle: 1").into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(response.headers().get(LOCATION).is_none());
    }

    #[test]
    fn empty_url_still_303_with_empty_location() {
        let response = Redirect::to("").into_response();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(LOCATION).unwrap().to_str().unwrap(),
            ""
        );
    }

    #[test]
    fn url_with_query_string_preserved() {
        let response = Redirect::to("/users?page=2").into_response();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(LOCATION).unwrap().to_str().unwrap(),
            "/users?page=2"
        );
    }
}
