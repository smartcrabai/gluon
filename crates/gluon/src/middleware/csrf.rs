//! CSRF protection middleware.
//!
//! Validates a session-bound CSRF token on state-changing HTTP methods
//! (anything outside `GET` / `HEAD` / `OPTIONS`). For safe methods, the
//! middleware ensures a token exists in the session, generating one if
//! necessary, and exposes it via request extensions so templates can embed it
//! in forms.
//!
//! Submitted tokens are accepted from either the `x-csrf-token` request header
//! or the `_csrf` field of an `application/x-www-form-urlencoded` body. The
//! request body is always buffered up to [`MAX_REQUEST_BODY_BYTES`] and then
//! restored, so downstream handlers always see the original payload.

use axum::body::{Body, Bytes, to_bytes};
use axum::extract::Request;
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;
use rand::RngExt;
use subtle::ConstantTimeEq;
use tower_sessions::Session;

const CSRF_SESSION_KEY: &str = "_gluon_csrf_token";
const CSRF_FORM_FIELD: &str = "_csrf";
const CSRF_HEADER: &str = "x-csrf-token";

/// Upper bound on body bytes the middleware will buffer when restoring a
/// state-changing request. Bodies larger than this fail closed with
/// `413 Payload Too Large`. The value (4 MiB) covers common form posts;
/// larger uploads should bypass this middleware or be gated behind explicit
/// per-route configuration.
const MAX_REQUEST_BODY_BYTES: usize = 4 * 1024 * 1024;

/// CSRF token value exposed on request extensions for downstream handlers and
/// view layers (e.g. form helpers).
#[derive(Debug, Clone)]
pub struct CsrfToken(String);

impl CsrfToken {
    /// Returns the token as a string slice for embedding into form fields or
    /// response headers.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Axum middleware function performing CSRF validation against the session.
///
/// # Errors
///
/// Returns:
/// - [`StatusCode::INTERNAL_SERVER_ERROR`] if the session backend fails.
/// - [`StatusCode::FORBIDDEN`] if a state-changing request is missing or
///   carries a mismatched token.
/// - [`StatusCode::PAYLOAD_TOO_LARGE`] if the request body exceeds
///   [`MAX_REQUEST_BODY_BYTES`].
pub async fn csrf_middleware(
    session: Session,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = ensure_session_token(&session).await?;

    if is_safe_method(request.method()) {
        let mut request = request;
        request.extensions_mut().insert(CsrfToken(token));
        return Ok(next.run(request).await);
    }

    let (parts, body) = request.into_parts();
    let body_bytes = to_bytes(body, MAX_REQUEST_BODY_BYTES)
        .await
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;

    let submitted = extract_header_token(&parts.headers).or_else(|| {
        if is_form_content_type(&parts.headers) {
            extract_form_token(&body_bytes)
        } else {
            None
        }
    });

    let Some(submitted) = submitted else {
        return Err(StatusCode::FORBIDDEN);
    };

    if !constant_time_eq(submitted.as_bytes(), token.as_bytes()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut request = Request::from_parts(parts, Body::from(body_bytes));
    request.extensions_mut().insert(CsrfToken(token));
    Ok(next.run(request).await)
}

/// Loads the CSRF token from the session, generating and persisting a new one
/// when absent.
async fn ensure_session_token(session: &Session) -> Result<String, StatusCode> {
    let existing: Option<String> = session
        .get(CSRF_SESSION_KEY)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(token) = existing {
        return Ok(token);
    }

    let token = generate_token();
    session
        .insert(CSRF_SESSION_KEY, &token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(token)
}

fn is_safe_method(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

fn extract_header_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(CSRF_HEADER)?.to_str().ok()?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn is_form_content_type(headers: &HeaderMap) -> bool {
    let Some(raw) = headers.get(header::CONTENT_TYPE) else {
        return false;
    };
    let Ok(value) = raw.to_str() else {
        return false;
    };
    value.split(';').next().is_some_and(|mime| {
        mime.trim()
            .eq_ignore_ascii_case("application/x-www-form-urlencoded")
    })
}

fn extract_form_token(bytes: &Bytes) -> Option<String> {
    for (key, value) in form_urlencoded::parse(bytes.as_ref()) {
        if key == CSRF_FORM_FIELD {
            return Some(value.into_owned());
        }
    }
    None
}

/// Constant-time comparison. Delegates to `subtle::ConstantTimeEq` which
/// handles both content and length without an early-return branch that would
/// leak the expected token length via timing.
fn constant_time_eq(lhs: &[u8], rhs: &[u8]) -> bool {
    lhs.ct_eq(rhs).into()
}

/// Generates a 32-byte CSPRNG-backed token encoded as lowercase hexadecimal.
fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    encode_hex(&bytes)
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0F) as usize] as char);
    }
    out
}
