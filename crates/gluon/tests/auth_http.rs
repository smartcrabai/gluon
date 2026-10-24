//! HTTP contracts intentionally fail until the compile-only auth API is implemented.
#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "common/auth/mod.rs"]
mod support;
#[path = "common/auth/passkey_device.rs"]
mod passkey_device;

use axum::http::{Method, StatusCode};
use axum::{Json, Router, middleware, routing::get};
use axum_test::TestServer;
use gluon::middleware::{csrf::csrf_middleware, htmx::htmx_middleware};
use gluon::{
    ContainerBuilder,
    auth::{Auth, CurrentUser},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower_sessions::{MemoryStore, SessionManagerLayer};

#[derive(Clone, Debug, Default)]
struct FailingLoadStore {
    memory: MemoryStore,
    fail: Arc<std::sync::atomic::AtomicBool>,
    failures: Arc<std::sync::atomic::AtomicUsize>,
}
#[async_trait::async_trait]
impl tower_sessions::SessionStore for FailingLoadStore {
    async fn create(
        &self,
        record: &mut tower_sessions::session::Record,
    ) -> tower_sessions::session_store::Result<()> {
        self.memory.create(record).await
    }
    async fn save(
        &self,
        record: &tower_sessions::session::Record,
    ) -> tower_sessions::session_store::Result<()> {
        self.memory.save(record).await
    }
    async fn load(
        &self,
        id: &tower_sessions::session::Id,
    ) -> tower_sessions::session_store::Result<Option<tower_sessions::session::Record>> {
        if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
            self.failures
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(tower_sessions::session_store::Error::Backend(
                "injected session backend failure".into(),
            ))
        } else {
            self.memory.load(id).await
        }
    }
    async fn delete(
        &self,
        id: &tower_sessions::session::Id,
    ) -> tower_sessions::session_store::Result<()> {
        self.memory.delete(id).await
    }
}

async fn assert_extractor_session_failure(path: &str) {
    let f = support::Fixture::new().await;
    let store = Arc::new(FailingLoadStore::default());
    let mut context = support::context();
    context.session = tower_sessions::Session::new(None, store.clone(), None);
    f.login(&context, "backend@example.test").await;
    context.session.save().await.unwrap();
    let id = context
        .session
        .id()
        .expect("authenticated session persisted");
    store.fail.store(true, std::sync::atomic::Ordering::SeqCst);
    let session = tower_sessions::Session::new(Some(id), store.clone(), None);
    // Inject a reopened Session directly. CSRF and SessionManager middleware must
    // not consume the failure and return 500 before CurrentUser is exercised.
    let app = Router::new()
        .route("/required", get(required))
        .route("/optional", get(optional))
        .layer(axum::Extension(session))
        .with_state(Arc::new(
            ContainerBuilder::new().bind_instance(f.auth).build(),
        ));
    let app = TestServer::new(app);
    assert_eq!(
        app.get(path).await.status_code(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(
        store.failures.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "extractor must try to restore the persisted session"
    );
}

#[tokio::test]
async fn required_extractor_surfaces_session_backend_failure_as_500() {
    assert_extractor_session_failure("/required").await;
}

#[tokio::test]
async fn optional_extractor_does_not_hide_session_backend_failure_as_anonymous() {
    assert_extractor_session_failure("/optional").await;
}

async fn required(CurrentUser(user): CurrentUser) -> Json<Value> {
    Json(json!({"user": user}))
}
async fn optional(user: Option<CurrentUser>) -> Json<Value> {
    Json(json!({"user": user.map(|u| u.0)}))
}

// The public router is merged AFTER the normal session/CSRF/HTMX stack.
fn server(auth: Arc<Auth>, register: bool) -> TestServer {
    let mut builder = ContainerBuilder::new();
    if register {
        builder = builder.bind_instance(auth.clone());
    }
    let app = Router::new()
        .nest("/auth", auth.router())
        .route("/required", get(required))
        .route("/optional", get(optional))
        .layer(middleware::from_fn(htmx_middleware))
        .layer(middleware::from_fn(csrf_middleware))
        .layer(
            SessionManagerLayer::new(MemoryStore::default())
                .with_name("gluon_sid")
                .with_secure(false),
        )
        .nest("/auth", auth.public_router())
        .with_state(Arc::new(builder.build()));
    let mut server = TestServer::new(app);
    server.save_cookies();
    server
}

#[tokio::test]
async fn required_extractor_rejects_anonymous_session() {
    let f = support::Fixture::new().await;
    let app = server(f.auth.clone(), true);
    assert_eq!(
        app.get("/required").await.status_code(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn optional_extractor_permits_anonymous_session() {
    let f = support::Fixture::new().await;
    let app = server(f.auth, true);
    assert!(app.get("/optional").await.json::<Value>()["user"].is_null());
}

#[tokio::test]
async fn extractors_surface_missing_auth_dependency_as_internal_error() {
    let f = support::Fixture::new().await;
    let missing = server(f.auth, false);
    for path in ["/required", "/optional"] {
        assert_eq!(
            missing.get(path).await.status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}

#[tokio::test]
async fn disabled_methods_are_not_exposed() {
    let f = support::Fixture::new().await;
    let app = server(f.auth, true);
    let csrf = app.get("/auth/csrf").await;
    let token = csrf.json::<Value>()["token"].as_str().unwrap().to_owned();
    for path in [
        "/auth/passkeys/registration/start",
        "/auth/passkeys/authentication/start",
        "/auth/oauth/google/start",
        "/auth/token",
    ] {
        let response = app
            .post(path)
            .add_header("x-csrf-token", token.clone())
            .json(&json!({}))
            .await;
        assert!(
            matches!(
                response.status_code(),
                StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
            ),
            "disabled route {path}"
        );
    }
}

#[tokio::test]
async fn auth_session_and_csrf_responses_are_not_cached() {
    let f = support::Fixture::new().await;
    let app = server(f.auth, true);
    for path in ["/auth/csrf", "/auth/session"] {
        let response = app.get(path).await;
        assert_no_store(&response);
    }
}

fn assert_no_store(response: &axum_test::TestResponse) {
    assert!(
        response.headers()["cache-control"]
            .to_str()
            .unwrap()
            .split(',')
            .any(|directive| directive.trim().eq_ignore_ascii_case("no-store"))
    );
}

async fn passkey_http_fixture() -> (support::Fixture, gluon::auth::AuthContext, TestServer) {
    let clock = Arc::new(support::TestClock::new());
    let mail = Arc::new(support::Mailbox::default());
    let mut config = support::config(clock.clone(), mail.clone());
    config.passkey = Some(gluon::auth::PasskeyConfig {
        rp_id: "app.example.test".into(),
        origin: passkey_device::ORIGIN.into(),
    });
    let f = support::Fixture::with_config(config, clock, mail).await;
    let context = support::context();
    context.session.insert("_gluon_auth_browser_binding", &context.browser_binding).await.unwrap();
    // Supply the same session to the HTTP extractor and direct service reference.
    let app = f.auth.router()
        .layer(axum::Extension(context.session.clone()))
        .with_state(Arc::new(ContainerBuilder::new().bind_instance(f.auth.clone()).build()));
    (f, context, TestServer::new(app))
}

fn assert_flow_response(response: &axum_test::TestResponse, mut expected_options: Value) -> Value {
    assert_eq!(response.status_code(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "application/json");
    assert_no_store(response);
    let body = response.json::<Value>();
    let fields = body.as_object().unwrap();
    assert_eq!(fields.len(), 2);
    assert!(fields.contains_key("flow_id") && fields.contains_key("options"));
    assert!(!body["flow_id"].as_str().unwrap().is_empty());
    assert!(!body["options"]["publicKey"]["challenge"].as_str().unwrap().is_empty());
    // Each start generates fresh randomness. All other nested options must match
    // the direct service result, including arrays, booleans and optional fields.
    expected_options["publicKey"]["challenge"] = body["options"]["publicKey"]["challenge"].clone();
    if expected_options["publicKey"].get("user").is_some() {
        assert!(!body["options"]["publicKey"]["user"]["id"].as_str().unwrap().is_empty());
        expected_options["publicKey"]["user"]["id"] = body["options"]["publicKey"]["user"]["id"].clone();
    }
    assert_eq!(body["options"], expected_options);
    body
}

#[tokio::test]
async fn enabled_passkey_starts_preserve_public_flows_and_nested_options() {
    let (f, context, app) = passkey_http_fixture().await;
    let user = f.login(&context, "http-passkey@example.test").await;
    let reference = f.auth.start_passkey_registration(&context).await.unwrap();
    let registration = assert_flow_response(
        &app.post("/passkeys/registration/start").json(&json!({})).await,
        reference.options,
    );
    let mut device = passkey_device::Device::new();
    let credential = device.register(registration["options"].clone());
    app.post("/passkeys/registration/finish").json(&json!({
        "flow_id": registration["flow_id"], "response": credential,
    })).await.assert_status_ok();

    for identified in [true, false] {
        let reference = f.auth.start_passkey_authentication(
            &context, identified.then_some(&user.id),
        ).await.unwrap();
        let request = if identified { json!({"user": user.id}) } else { json!({}) };
        let flow = assert_flow_response(
            &app.post("/passkeys/authentication/start").json(&request).await,
            reference.options,
        );
        // A signed completion also binds the returned challenge and credential
        // options to the returned flow_id rather than merely checking a schema.
        let assertion = device.authenticate(flow["options"].clone());
        let finished = app.post("/passkeys/authentication/finish").json(&json!({
            "flow_id": flow["flow_id"], "response": assertion,
        })).await;
        assert_eq!(finished.status_code(), StatusCode::OK);
        assert_eq!(finished.json::<Value>(), json!({"user": user}));
    }
}

fn assert_passkey_error(response: &axum_test::TestResponse, status: StatusCode, code: &str) {
    assert_eq!(response.status_code(), status);
    assert_eq!(response.headers()["content-type"], "application/json");
    assert_no_store(response);
    assert_eq!(response.json::<Value>(), json!({"code": code}));
}

#[tokio::test]
async fn enabled_passkey_start_service_errors_keep_status_and_body() {
    let (f, context, app) = passkey_http_fixture().await;
    assert_passkey_error(
        &app.post("/passkeys/registration/start").json(&json!({})).await,
        StatusCode::UNAUTHORIZED, "unauthorized",
    );
    assert_passkey_error(
        &app.post("/passkeys/authentication/start").json(&json!({"user": "missing"})).await,
        StatusCode::BAD_REQUEST, "invalid_passkey",
    );
    let user = f.login(&context, "http-errors@example.test").await;
    f.clock.advance(time::Duration::minutes(6));
    assert_passkey_error(
        &app.post("/passkeys/registration/start").json(&json!({})).await,
        StatusCode::FORBIDDEN, "reauthentication_required",
    );
    f.config.store.as_ref().unwrap().transact(&mut |data| {
        data.passkeys.insert("malformed".into(), gluon::auth::StoredPasskey {
            user: user.id.0.clone(), data: json!({"format": 1}), version: 0,
        });
        Ok(())
    }).await.unwrap();
    assert_passkey_error(
        &app.post("/passkeys/authentication/start").json(&json!({"user": user.id})).await,
        StatusCode::INTERNAL_SERVER_ERROR, "storage_error",
    );
}

#[tokio::test]
async fn direct_api_and_http_share_destination_rate_budget() {
    let clock = Arc::new(support::TestClock::new());
    let mail = Arc::new(support::Mailbox::default());
    let mut config = support::config(clock.clone(), mail.clone());
    config.email_otp.as_mut().unwrap().destination_limit = 1;
    config.email_otp.as_mut().unwrap().resend_interval = time::Duration::ZERO;
    let f = support::Fixture::with_config(config, clock, mail).await;
    let app = server(f.auth.clone(), true);
    f.auth
        .request_email_otp(&support::context(), "shared@example.test")
        .await
        .unwrap();
    let token = app.get("/auth/csrf").await.json::<Value>()["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let response = app
        .post("/auth/email-otp/request")
        .add_header("x-csrf-token", token)
        .json(&json!({"email": "shared@example.test"}))
        .await;
    assert_eq!(response.status_code(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.json::<Value>(), json!({"code": "rate_limited"}));
    assert_no_store(&response);
    assert_eq!(f.mail.messages.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn cors_allows_only_configured_origin() {
    assert_cors_for_configured_origin("https://app.example.test").await;
}

#[tokio::test]
async fn cors_canonicalizes_configured_origin_with_trailing_slash() {
    assert_cors_for_configured_origin("https://app.example.test/").await;
}

async fn assert_cors_for_configured_origin(origin: &str) {
    let clock = Arc::new(support::TestClock::new());
    let mail = Arc::new(support::Mailbox::default());
    let mut config = support::config(clock.clone(), mail.clone());
    config.allowed_origins = vec![origin.into()];
    let f = support::Fixture::with_config(config, clock, mail).await;
    let app = server(f.auth, true);
    for (origin, allowed) in [
        ("https://app.example.test", true),
        ("https://evil.example.test", false),
    ] {
        let response = app
            .method(Method::OPTIONS, "/auth/email-otp/request")
            .add_header("origin", origin)
            .add_header("access-control-request-method", "POST")
            .add_header(
                "access-control-request-headers",
                "content-type,x-csrf-token",
            )
            .await;
        assert!(response.status_code().is_success());
        assert_no_store(&response);
        if allowed {
            assert_eq!(response.headers()["access-control-allow-origin"], origin);
            assert_eq!(
                response.headers()["access-control-allow-credentials"],
                "true"
            );
            assert_eq!(response.headers()["access-control-allow-methods"], "GET, HEAD, POST, DELETE, OPTIONS");
            assert_eq!(response.headers()["access-control-allow-headers"], "content-type, x-csrf-token");
        } else {
            assert!(
                !response
                    .headers()
                    .contains_key("access-control-allow-origin")
            );
            assert!(!response.headers().contains_key("access-control-allow-credentials"));
        }
        let normal = app.get("/auth/session").add_header("origin", origin).await;
        assert!(normal.status_code().is_success());
        assert_no_store(&normal);
        if allowed {
            assert_eq!(normal.headers()["access-control-allow-origin"], origin);
            assert_eq!(normal.headers()["access-control-allow-credentials"], "true");
        } else {
            assert!(!normal.headers().contains_key("access-control-allow-origin"));
            assert!(!normal.headers().contains_key("access-control-allow-credentials"));
        }
    }
}
