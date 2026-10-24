//! Thin HTTP adapters for the shared authentication service.
use super::{Auth, AuthContext, AuthError, UserId};
use crate::{Container, middleware::csrf::CsrfToken};
use axum::{
    Extension, Json, Router,
    extract::{Path, Query, Request},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post},
};
use http::{HeaderValue, Method, header};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

type Output = Result<Json<Value>, AuthError>;

pub(super) fn router(auth: Arc<Auth>) -> Router<Arc<Container>> {
    let mut router = Router::new()
        .route("/csrf", get(csrf))
        .route("/session", get(session))
        .route("/logout", post(logout));
    if auth.config.email_otp.is_some() {
        router = router
            .route("/email-otp/request", post(otp_request))
            .route("/email-otp/verify", post(otp_verify));
    }
    if auth.config.jwt.is_some() {
        router = router.route("/token", post(token));
    }
    if auth.config.passkey.is_some() {
        router = router
            .route("/passkeys/registration/start", post(registration_start))
            .route("/passkeys/registration/finish", post(registration_finish))
            .route("/passkeys/authentication/start", post(authentication_start))
            .route(
                "/passkeys/authentication/finish",
                post(authentication_finish),
            )
            .route("/passkeys", get(passkeys))
            .route("/passkeys/{id}", delete(passkey_delete));
    }
    // Register concrete provider routes, never expose unconfigured methods.
    for provider in &auth.config.oauth {
        let base = format!("/oauth/{}", provider.name());
        router = router
            .route(&format!("{base}/start"), post(oauth_start))
            .route(&format!("{base}/link"), post(oauth_link))
            .route(&format!("{base}/callback"), get(oauth_callback));
    }
    router
        .layer(Extension(auth.clone()))
        .layer(middleware::from_fn(move |request: Request, next: Next| {
            let auth = auth.clone();
            async move {
                let origins = &auth.config.allowed_origins;
                let origin = request
                    .headers()
                    .get(header::ORIGIN)
                    .cloned()
                    .filter(|value| {
                        value
                            .to_str()
                            .is_ok_and(|value| origins.iter().any(|allowed| allowed == value))
                    });
                let preflight = request.method() == Method::OPTIONS;
                let mut response = if preflight {
                    Response::new(axum::body::Body::empty())
                } else {
                    next.run(request).await
                };
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                if let Some(origin) = origin {
                    response
                        .headers_mut()
                        .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
                    response.headers_mut().insert(
                        header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                        HeaderValue::from_static("true"),
                    );
                    response
                        .headers_mut()
                        .insert(header::VARY, HeaderValue::from_static("Origin"));
                    if preflight {
                        response.headers_mut().insert(
                            header::ACCESS_CONTROL_ALLOW_METHODS,
                            HeaderValue::from_static("GET, HEAD, POST, DELETE, OPTIONS"),
                        );
                        response.headers_mut().insert(
                            header::ACCESS_CONTROL_ALLOW_HEADERS,
                            HeaderValue::from_static("content-type, x-csrf-token"),
                        );
                    }
                }
                response
            }
        }))
}

pub(super) fn public_router(auth: Arc<Auth>) -> Router<Arc<Container>> {
    if auth.config.jwt.is_none() {
        return Router::new();
    }
    Router::new().route(
        "/jwks",
        get(move || {
            let auth = auth.clone();
            async move {
                (
                    [(header::CACHE_CONTROL, "public, max-age=60")],
                    Json(auth.jwks()),
                )
            }
        }),
    )
}
async fn csrf(Extension(token): Extension<CsrfToken>) -> Json<Value> {
    Json(json!({"token": token.as_str()}))
}
async fn session(Extension(auth): Extension<Arc<Auth>>, context: AuthContext) -> Output {
    Ok(Json(json!({"user": auth.current_user(&context).await?})))
}
async fn logout(Extension(auth): Extension<Arc<Auth>>, context: AuthContext) -> Output {
    auth.logout(&context).await?;
    Ok(Json(json!({"ok": true})))
}
async fn token(Extension(auth): Extension<Arc<Auth>>, context: AuthContext) -> Output {
    Ok(Json(json!({"token": auth.token(&context).await?})))
}
#[derive(Deserialize)]
struct EmailRequest {
    email: String,
}
async fn otp_request(
    Extension(auth): Extension<Arc<Auth>>,
    context: AuthContext,
    Json(body): Json<EmailRequest>,
) -> Result<Json<super::OtpChallenge>, AuthError> {
    Ok(Json(auth.request_email_otp(&context, &body.email).await?))
}
#[derive(Deserialize)]
struct OtpVerify {
    challenge_id: String,
    code: String,
}
async fn otp_verify(
    Extension(auth): Extension<Arc<Auth>>,
    context: AuthContext,
    Json(body): Json<OtpVerify>,
) -> Output {
    Ok(Json(
        json!({"user": auth.verify_email_otp(&context, &body.challenge_id, &body.code).await?}),
    ))
}
async fn registration_start(
    Extension(auth): Extension<Arc<Auth>>,
    context: AuthContext,
) -> Result<Json<super::AuthFlow>, AuthError> {
    auth.start_passkey_registration(&context).await.map(Json)
}
#[derive(Deserialize)]
struct Finish {
    flow_id: String,
    response: Value,
}
async fn registration_finish(
    Extension(auth): Extension<Arc<Auth>>,
    context: AuthContext,
    Json(body): Json<Finish>,
) -> Output {
    auth.finish_passkey_registration(&context, &body.flow_id, body.response)
        .await?;
    Ok(Json(json!({"ok": true})))
}
#[derive(Deserialize)]
struct AuthenticationStart {
    user: Option<UserId>,
}
async fn authentication_start(
    Extension(auth): Extension<Arc<Auth>>,
    context: AuthContext,
    Json(body): Json<AuthenticationStart>,
) -> Result<Json<super::AuthFlow>, AuthError> {
    auth.start_passkey_authentication(&context, body.user.as_ref())
        .await
        .map(Json)
}
async fn authentication_finish(
    Extension(auth): Extension<Arc<Auth>>,
    context: AuthContext,
    Json(body): Json<Finish>,
) -> Output {
    Ok(Json(
        json!({"user": auth.finish_passkey_authentication(&context, &body.flow_id, body.response).await?}),
    ))
}
async fn passkeys(Extension(auth): Extension<Arc<Auth>>, context: AuthContext) -> Output {
    Ok(Json(json!({"passkeys": auth.passkeys(&context).await?})))
}
async fn passkey_delete(
    Extension(auth): Extension<Arc<Auth>>,
    context: AuthContext,
    Path(id): Path<String>,
) -> Output {
    auth.delete_passkey(&context, &id).await?;
    Ok(Json(json!({"ok": true})))
}
fn provider(context: &axum::extract::OriginalUri) -> Result<&str, AuthError> {
    context.0.path().split('/').rev().nth(1).ok_or(AuthError {
        code: "invalid_request",
    })
}
async fn oauth_start(
    Extension(auth): Extension<Arc<Auth>>,
    context: AuthContext,
    uri: axum::extract::OriginalUri,
) -> Output {
    Ok(Json(json!(
        auth.start_oauth(&context, provider(&uri)?, false).await?
    )))
}
async fn oauth_link(
    Extension(auth): Extension<Arc<Auth>>,
    context: AuthContext,
    uri: axum::extract::OriginalUri,
) -> Output {
    Ok(Json(json!(
        auth.start_oauth(&context, provider(&uri)?, true).await?
    )))
}
#[derive(Deserialize)]
struct Callback {
    state: String,
    #[serde(default)]
    code: String,
}
async fn oauth_callback(
    Extension(auth): Extension<Arc<Auth>>,
    context: AuthContext,
    uri: axum::extract::OriginalUri,
    Query(body): Query<Callback>,
) -> Output {
    Ok(Json(
        json!({"user": auth.finish_oauth(&context, provider(&uri)?, &body.state, &body.code).await?}),
    ))
}
