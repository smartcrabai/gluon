//! Application error types for the gluon framework.

use std::fmt;

use axum::response::{IntoResponse, Response};
use http::StatusCode;

/// A single field validation error.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

/// Application level error type returned from controllers and use cases.
#[derive(Debug)]
pub enum AppError {
    /// Requested resource was not found.
    NotFound,
    /// Caller is not authenticated.
    Unauthorized,
    /// Caller is authenticated but lacks permission.
    Forbidden,
    /// One or more fields failed validation.
    Validation(Vec<FieldError>),
    /// Operation conflicts with the current state.
    Conflict(String),
    /// Request is malformed or otherwise invalid.
    BadRequest(String),
    /// Unhandled internal failure.
    Internal(Box<dyn std::error::Error + Send + Sync>),
}

/// Convenience [`Result`] alias whose error variant is [`AppError`].
pub type Result<T> = std::result::Result<T, AppError>;

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => f.write_str("resource not found"),
            Self::Unauthorized => f.write_str("unauthorized"),
            Self::Forbidden => f.write_str("forbidden"),
            Self::Validation(fields) => {
                write!(f, "validation failed ({} field(s))", fields.len())
            }
            Self::Conflict(message) => write!(f, "conflict: {message}"),
            Self::BadRequest(message) => write!(f, "bad request: {message}"),
            Self::Internal(source) => write!(f, "internal error: {source}"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Internal(source) => Some(&**source),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Internal(Box::new(value))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        // For `Internal`, log the source and return only a generic message to
        // avoid leaking implementation details (DB error text, file paths,
        // OTLP endpoints, etc.) in the response body.
        let body = match &self {
            Self::Internal(source) => {
                tracing::error!(error = %source, "internal server error");
                "internal server error".to_string()
            }
            Self::Validation(fields) => match serde_json::to_string(fields) {
                Ok(json) => json,
                Err(_) => "validation failed".to_string(),
            },
            other => other.to_string(),
        };
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn body_string(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn status_of(err: AppError) -> StatusCode {
        err.into_response().status()
    }

    #[test]
    fn status_codes_map_to_variants() {
        assert_eq!(status_of(AppError::NotFound), StatusCode::NOT_FOUND);
        assert_eq!(status_of(AppError::Unauthorized), StatusCode::UNAUTHORIZED);
        assert_eq!(status_of(AppError::Forbidden), StatusCode::FORBIDDEN);
        assert_eq!(
            status_of(AppError::Validation(vec![])),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            status_of(AppError::Conflict("x".into())),
            StatusCode::CONFLICT
        );
        assert_eq!(
            status_of(AppError::BadRequest("x".into())),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status_of(AppError::Internal(Box::new(std::io::Error::other("boom")))),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn internal_error_body_does_not_leak_source() {
        let secret = "DATABASE_URL=postgres://user:pass@db/secret";
        let err = AppError::Internal(Box::new(std::io::Error::other(secret)));
        let body = body_string(err.into_response()).await;
        assert!(!body.contains(secret), "internal source leaked: {body}");
        assert!(body.contains("internal server error"), "body: {body}");
    }

    #[tokio::test]
    async fn validation_body_serializes_field_errors() {
        let err = AppError::Validation(vec![FieldError {
            field: "email".into(),
            message: "must be present".into(),
        }]);
        let body = body_string(err.into_response()).await;
        assert!(body.contains("\"field\":\"email\""), "body: {body}");
        assert!(
            body.contains("\"message\":\"must be present\""),
            "body: {body}"
        );
    }

    #[test]
    fn from_io_error_wraps_in_internal() {
        let err: AppError = std::io::Error::other("boom").into();
        assert!(matches!(err, AppError::Internal(_)));
    }

    #[test]
    fn display_format_per_variant() {
        assert!(
            AppError::NotFound
                .to_string()
                .starts_with("resource not found")
        );
        assert!(
            AppError::Unauthorized
                .to_string()
                .starts_with("unauthorized")
        );
        assert!(AppError::Forbidden.to_string().starts_with("forbidden"));
        assert!(
            AppError::Validation(vec![FieldError {
                field: "f".into(),
                message: "m".into(),
            }])
            .to_string()
            .starts_with("validation failed (1 field(s))")
        );
        assert!(
            AppError::Conflict("msg".into())
                .to_string()
                .starts_with("conflict: msg")
        );
        assert!(
            AppError::BadRequest("msg".into())
                .to_string()
                .starts_with("bad request: msg")
        );
        assert!(
            AppError::Internal(Box::new(std::io::Error::other("boom")))
                .to_string()
                .starts_with("internal error: ")
        );
    }

    #[test]
    fn source_returns_none_for_non_internal() {
        use std::error::Error;
        assert!(AppError::NotFound.source().is_none());
        assert!(AppError::Unauthorized.source().is_none());
        assert!(AppError::Forbidden.source().is_none());
        assert!(AppError::Validation(vec![]).source().is_none());
        assert!(AppError::Conflict("x".into()).source().is_none());
        assert!(AppError::BadRequest("x".into()).source().is_none());
    }

    #[test]
    fn source_returns_some_for_internal() {
        use std::error::Error;
        let inner_msg = "underlying io failure";
        let err = AppError::Internal(Box::new(std::io::Error::other(inner_msg)));
        let source = err.source().expect("Internal should expose a source");
        assert!(
            source.to_string().contains(inner_msg),
            "source display: {source}",
        );
    }

    #[tokio::test]
    async fn conflict_body_includes_message() {
        let body = body_string(AppError::Conflict("email taken".into()).into_response()).await;
        assert!(body.contains("email taken"), "body: {body}");
    }

    #[tokio::test]
    async fn bad_request_body_includes_message() {
        let body = body_string(AppError::BadRequest("missing field".into()).into_response()).await;
        assert!(body.contains("missing field"), "body: {body}");
    }
}
