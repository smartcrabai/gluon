//! Real Boot entry-point contracts. No test-side auth route mounting.
#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "common/auth/mod.rs"]
mod support;
use async_trait::async_trait;
use axum::{
    Json, Router,
    routing::{get, post},
};
use gluon::{
    Boot, Inject, Session,
    auth::{
        Auth, AuthContext, AuthError, CurrentUser, EmailSender, MemoryAuthStore, OtpChallenge,
        OtpEmail,
    },
};
use reqwest::{Client, Response, StatusCode};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Arc,
    time::Duration,
};

#[path = "common/auth/jwt.rs"]
mod jwt_fixture;
struct FileMail(PathBuf);
#[async_trait]
impl EmailSender for FileMail {
    async fn send(&self, email: OtpEmail) -> Result<(), AuthError> {
        std::fs::write(
            &self.0,
            serde_json::to_vec(&json!({"code": email.code, "challenge_id": email.challenge_id}))
                .unwrap(),
        )
        .unwrap();
        Ok(())
    }
}
async fn required(CurrentUser(user): CurrentUser) -> Json<Value> {
    Json(json!({"user": user}))
}
async fn optional(user: Option<CurrentUser>) -> Json<Value> {
    Json(json!({"user": user.map(|u| u.0)}))
}

async fn direct_email_request(
    Inject(auth): Inject<Auth>,
    session: Session,
) -> Result<Json<OtpChallenge>, AuthError> {
    let context = AuthContext {
        session,
        source: "192.0.2.55".parse().unwrap(),
        browser_binding: "boot-direct-api-fixture".into(),
    };
    auth.request_email_otp(&context, "shared-boot@example.test")
        .await
        .map(Json)
}

#[tokio::test]
#[ignore = "subprocess fixture launched only by Boot contract tests"]
async fn boot_child() {
    if std::env::var("GLUON_AUTH_TEST_CHILD").as_deref() != Ok("1") {
        return;
    }
    if std::env::var("GLUON_AUTH_TEST_HTTP").as_deref() == Ok("absent") {
        Boot::new()
            .with_router(Router::new().route("/ready", get(|| async { "ready" })))
            .run()
            .await
            .unwrap();
        return;
    }
    let mut config = support::config(
        Arc::new(support::TestClock::new()),
        Arc::new(support::Mailbox::default()),
    );
    let mode = std::env::var("GLUON_AUTH_TEST_HTTP").unwrap();
    config.mount_path = if mode == "default-mount" {
        "/auth"
    } else {
        "/identity"
    }
    .into();
    config.http_enabled = mode != "0";
    config.email_otp.as_mut().unwrap().sender = Arc::new(FileMail(
        std::env::var_os("GLUON_AUTH_TEST_MAIL").unwrap().into(),
    ));
    config.jwt = Some(jwt_fixture::jwt_config());
    let mut boot = if mode == "override" {
        // Separate stores and no resend interval ensure that only using the
        // replacement service for BOTH paths can exhaust the one-request budget.
        config.email_otp.as_mut().unwrap().resend_interval = time::Duration::ZERO;
        let mut replacement = config.clone();
        replacement.store = Some(Arc::new(MemoryAuthStore::new(1024)));
        replacement.email_otp.as_mut().unwrap().destination_limit = 1;
        let auth = Arc::new(Auth::new(replacement, None).await.unwrap());
        Boot::new()
            .with_auth(config)
            .with_container(move |builder| builder.bind_instance(auth))
    } else {
        Boot::new().with_auth(config)
    };
    boot = boot.with_router(
        Router::new()
            .route("/ready", get(|| async { "ready" }))
            .route("/required", get(required))
            .route("/optional", get(optional))
            .route("/direct-email-request", post(direct_email_request)),
    );
    boot.run().await.unwrap();
}

struct ChildGuard {
    child: Child,
    base: String,
    mail: PathBuf,
    _dir: tempfile::TempDir,
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl ChildGuard {
    async fn start(http: bool) -> Self {
        Self::start_mode(if http { "1" } else { "0" }).await
    }
    async fn start_mode(mode: &str) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let dir = tempfile::tempdir().unwrap();
        let mail = dir.path().join("mail.json");
        let child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "boot_child", "--ignored", "--nocapture"])
            .env("GLUON_AUTH_TEST_CHILD", "1")
            .env("GLUON_AUTH_TEST_HTTP", mode)
            .env("GLUON_AUTH_TEST_MAIL", &mail)
            .env("GLUON_BIND", addr.to_string())
            .env("GLUON_TELEMETRY_DISABLED", "1")
            .env("GLUON_INSECURE_COOKIE", "1")
            .env(
                "SECRET_KEY_BASE",
                "auth-test-secret-key-base-01234567890123456789012345678901234567890123456789",
            )
            .env_remove("DATABASE_URL")
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut guard = Self {
            child,
            base: format!("http://{addr}"),
            mail,
            _dir: dir,
        };
        let client = Client::new();
        for _ in 0..100 {
            assert!(
                guard.child.try_wait().unwrap().is_none(),
                "Boot child exited before serving; auth implementation may still be a stub"
            );
            if client
                .get(format!("{}/ready", guard.base))
                .send()
                .await
                .is_ok()
            {
                return guard;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("Boot did not become ready within five seconds");
    }
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }
}
fn cookie(response: &Response) -> String {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with("gluon_sid="))
        .expect("signed session cookie")
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}
fn no_store(response: &Response) {
    assert!(
        response.headers()["cache-control"]
            .to_str()
            .unwrap()
            .split(',')
            .any(|directive| directive.trim().eq_ignore_ascii_case("no-store"))
    );
}
async fn csrf(client: &Client, child: &ChildGuard, sid: Option<&str>) -> (String, String) {
    let mut request = client.get(child.url("/identity/csrf"));
    if let Some(sid) = sid {
        request = request.header("cookie", sid);
    }
    let response = request.send().await.unwrap();
    assert!(response.status().is_success());
    no_store(&response);
    let next = if response.headers().contains_key("set-cookie") {
        cookie(&response)
    } else {
        sid.unwrap().to_owned()
    };
    let body: Value = response.json().await.unwrap();
    (body["token"].as_str().unwrap().to_owned(), next)
}

#[tokio::test]
async fn boot_otp_rotates_session_csrf_and_preserves_cookie_only_authentication() {
    let child = ChildGuard::start(true).await;
    let client = Client::new(); // Explicit cookies let us replay the invalidated old session.
    assert_eq!(
        client
            .get(child.url("/auth/csrf"))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        client
            .get(child.url("/required"))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert!(
        client
            .get(child.url("/optional"))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()["user"]
            .is_null()
    );
    let (old_csrf, old_sid) = csrf(&client, &child, None).await;
    let (sid, token) =
        verify_email_otp_and_rotate_session(&client, &child, &old_sid, &old_csrf).await;
    assert_cookie_only_token_authentication(&client, &child, &sid, &old_csrf, &token).await;
    let logout = client
        .post(child.url("/identity/logout"))
        .header("cookie", &sid)
        .header("x-csrf-token", token)
        .send()
        .await
        .unwrap();
    assert!(logout.status().is_success());
    no_store(&logout);
    assert_eq!(
        client
            .get(child.url("/required"))
            .header("cookie", &sid)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

async fn verify_email_otp_and_rotate_session(
    client: &Client,
    child: &ChildGuard,
    old_sid: &str,
    old_csrf: &str,
) -> (String, String) {
    let request = client
        .post(child.url("/identity/email-otp/request"))
        .header("cookie", old_sid)
        .header("x-csrf-token", old_csrf)
        .json(&json!({"email": "login@example.test"}))
        .send()
        .await
        .unwrap();
    assert!(request.status().is_success());
    no_store(&request);
    let challenge: Value = request.json().await.unwrap();
    assert_eq!(challenge.as_object().unwrap().len(), 1);
    let mail: Value = serde_json::from_slice(&std::fs::read(&child.mail).unwrap()).unwrap();
    assert_eq!(challenge["challenge_id"], mail["challenge_id"]);
    assert!(
        challenge.get("code").is_none(),
        "OTP must never be returned to HTTP caller"
    );
    let verified = client
        .post(child.url("/identity/email-otp/verify"))
        .header("cookie", old_sid)
        .header("x-csrf-token", old_csrf)
        .json(&json!({"challenge_id": challenge["challenge_id"], "code": mail["code"]}))
        .send()
        .await
        .unwrap();
    assert!(verified.status().is_success());
    no_store(&verified);
    let attributes = verified.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .to_ascii_lowercase();
    assert!(attributes.contains("httponly") && attributes.contains("samesite=lax"));
    let sid = cookie(&verified);
    assert_ne!(sid, old_sid);
    assert_eq!(
        client
            .get(child.url("/required"))
            .header("cookie", old_sid)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let (token, sid) = csrf(client, child, Some(&sid)).await;
    assert_ne!(token, old_csrf);
    assert!(
        client
            .get(child.url("/required"))
            .header("cookie", &sid)
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );
    (sid, token)
}

async fn assert_cookie_only_token_authentication(
    client: &Client,
    child: &ChildGuard,
    sid: &str,
    old_csrf: &str,
    csrf_token: &str,
) {
    let stale = client
        .post(child.url("/identity/token"))
        .header("cookie", sid)
        .header("x-csrf-token", old_csrf)
        .send()
        .await
        .unwrap();
    assert_eq!(stale.status(), StatusCode::FORBIDDEN);
    no_store(&stale);
    let issued = client
        .post(child.url("/identity/token"))
        .header("cookie", sid)
        .header("x-csrf-token", csrf_token)
        .send()
        .await
        .unwrap();
    assert!(issued.status().is_success());
    no_store(&issued);
    let jwt = issued.json::<Value>().await.unwrap()["token"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        client
            .get(child.url("/required"))
            .bearer_auth(&jwt)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .post(child.url("/identity/logout"))
            .header("cookie", sid)
            .bearer_auth(&jwt)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn boot_csrf_rejections_are_not_cached_only_under_the_auth_mount() {
    for (mode, mount) in [("1", "/identity"), ("default-mount", "/auth")] {
        let child = ChildGuard::start_mode(mode).await;
        let client = Client::new();
        let initial = client
            .get(child.url(&format!("{mount}/csrf")))
            .send()
            .await
            .unwrap();
        let sid = cookie(&initial);
        no_store(&initial);
        for token in [None, Some("incorrect-token")] {
            let mut request = client
                .post(child.url(&format!("{mount}/logout")))
                .header("cookie", &sid);
            if let Some(token) = token {
                request = request.header("x-csrf-token", token);
            }
            let response = request.send().await.unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            no_store(&response);
        }
        let ready = client.get(child.url("/ready")).send().await.unwrap();
        assert_eq!(ready.status(), StatusCode::OK);
        assert!(!ready.headers().contains_key("cache-control"));
        let unrelated = client
            .post(child.url("/direct-email-request"))
            .header("cookie", &sid)
            .send()
            .await
            .unwrap();
        assert_eq!(unrelated.status(), StatusCode::FORBIDDEN);
        assert!(!unrelated.headers().contains_key("cache-control"));
        let boundary = client
            .get(child.url(&format!("{mount}-other")))
            .send()
            .await
            .unwrap();
        assert!(!boundary.headers().contains_key("cache-control"));
    }
}

#[tokio::test]
async fn boot_public_jwks_is_cacheable_without_session_or_private_key_material() {
    let child = ChildGuard::start(true).await;
    let client = Client::new();
    for method in [reqwest::Method::GET, reqwest::Method::HEAD] {
        let response = client
            .request(method, child.url("/identity/jwks"))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
        assert!(!response.headers().contains_key("set-cookie"));
        let cache = response.headers()["cache-control"].to_str().unwrap();
        let max_age = cache
            .split(',')
            .find_map(|s| s.trim().strip_prefix("max-age="))
            .unwrap()
            .parse::<u64>()
            .unwrap();
        assert!(
            max_age > 0,
            "JWKS must declare an explicit finite positive cache lifetime"
        );
    }
    let body = client
        .get(child.url("/identity/jwks"))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    let keys = body["keys"].as_array().unwrap();
    assert_eq!(keys.len(), 1, "configured signing key must be published");
    assert_eq!(keys[0]["kid"], "old-key");
    for key in keys {
        assert!(key.get("d").is_none());
    }
}

#[tokio::test]
async fn boot_public_jwks_rejects_unsafe_methods_without_creating_sessions() {
    let child = ChildGuard::start(true).await;
    let client = Client::new();
    for method in [
        reqwest::Method::POST,
        reqwest::Method::PUT,
        reqwest::Method::PATCH,
        reqwest::Method::DELETE,
    ] {
        let response = client
            .request(method, child.url("/identity/jwks"))
            .send()
            .await
            .unwrap();
        assert!(!response.status().is_success());
        assert!(!response.headers().contains_key("set-cookie"));
        assert_ne!(
            response
                .headers()
                .get("cache-control")
                .and_then(|v| v.to_str().ok()),
            Some("no-store")
        );
    }
}

#[tokio::test]
async fn boot_http_disabled_leaves_auth_in_di_but_mounts_no_auth_endpoints() {
    let child = ChildGuard::start(false).await;
    let client = Client::new();
    for path in [
        "/identity/csrf",
        "/identity/session",
        "/identity/jwks",
        "/auth/csrf",
    ] {
        assert_eq!(
            client.get(child.url(path)).send().await.unwrap().status(),
            StatusCode::NOT_FOUND
        );
    }
    // 401 rather than 500 proves Boot registered Auth even without its HTTP adapter.
    assert_eq!(
        client
            .get(child.url("/required"))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert!(
        client
            .get(child.url("/optional"))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()["user"]
            .is_null()
    );
}

#[tokio::test]
async fn boot_with_auth_feature_but_without_with_auth_serves_normally_without_auth_routes() {
    let child = ChildGuard::start_mode("absent").await;
    let client = Client::new();
    let response = client.get(child.url("/ready")).send().await.unwrap();
    assert!(response.status().is_success());
    assert_eq!(response.text().await.unwrap(), "ready");
    for path in [
        "/auth/csrf",
        "/auth/session",
        "/auth/jwks",
        "/identity/csrf",
    ] {
        assert_eq!(
            client.get(child.url(path)).send().await.unwrap().status(),
            StatusCode::NOT_FOUND
        );
    }
}

#[tokio::test]
async fn boot_http_and_injected_direct_api_use_the_with_container_auth_override() {
    let child = ChildGuard::start_mode("override").await;
    let client = Client::new();
    let (token, sid) = csrf(&client, &child, None).await;
    let first = client
        .post(child.url("/identity/email-otp/request"))
        .header("cookie", &sid)
        .header("x-csrf-token", &token)
        .json(&json!({"email": "shared-boot@example.test"}))
        .send()
        .await
        .unwrap();
    assert!(
        first.status().is_success(),
        "first HTTP request consumes replacement service budget"
    );
    let direct = client
        .post(child.url("/direct-email-request"))
        .header("cookie", &sid)
        .header("x-csrf-token", &token)
        .send()
        .await
        .unwrap();
    assert_eq!(
        direct.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "DI-resolved direct API must observe HTTP's consumed budget"
    );
    let repeated = client
        .post(child.url("/identity/email-otp/request"))
        .header("cookie", &sid)
        .header("x-csrf-token", &token)
        .json(&json!({"email": "shared-boot@example.test"}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        repeated.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "HTTP must honor replacement limit=1, not Boot's original larger budget"
    );
}
