//! Black-box HTTP tests for the checked-in example applications.

#![allow(clippy::expect_used, clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use reqwest::header::VARY;
use serde_json::{Value, json};

mod common;
use common::{ChildGuard, drain_to_void, pick_port};

/// `CARGO_MANIFEST_DIR` is `<root>/crates/gluon-cli`, so the repo root is fixed
/// at compile time; no need to probe the filesystem for it.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("gluon-cli lives two levels below the repository root")
        .to_path_buf()
}

struct MovedSources(Vec<(PathBuf, PathBuf)>);

impl Drop for MovedSources {
    fn drop(&mut self) {
        for (source, moved) in self.0.iter().rev() {
            if moved.exists() {
                let _ = std::fs::rename(moved, source);
            }
        }
    }
}

fn restore_stale_moved_sources(member: &Path) {
    for name in ["app", "public"] {
        let source = member.join(name);
        let moved = member.join(format!(".{name}.embedded-test-moved"));
        if !source.exists() && moved.exists() {
            std::fs::rename(&moved, &source).unwrap_or_else(|error| {
                panic!("restore {} from test backup: {error}", source.display())
            });
        }
    }
}

fn move_app_and_public_sources(member: &Path) -> MovedSources {
    let mut moved_sources = MovedSources(Vec::new());
    for name in ["app", "public"] {
        let source = member.join(name);
        if source.exists() {
            let moved = member.join(format!(".{name}.embedded-test-moved"));
            assert!(
                !moved.exists(),
                "stale embedded test source directory: {}",
                moved.display()
            );
            std::fs::rename(&source, &moved).unwrap_or_else(|error| {
                panic!("move {} out of runtime path: {error}", source.display())
            });
            moved_sources.0.push((source, moved));
        }
    }
    moved_sources
}

fn compile_examples(workspace: &Path) {
    let output = Command::new("cargo")
        .args(["build", "--workspace", "--quiet"])
        .current_dir(workspace)
        .output()
        .expect("spawn cargo build for examples workspace");
    assert!(
        output.status.success(),
        "cargo build failed in {}\nstdout:\n{}\nstderr:\n{}",
        workspace.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Spawn the already-compiled example binary directly. Spawning `cargo run`
/// instead would leave the real server orphaned whenever the guard kills only
/// its immediate child (`cargo`), so the server would keep holding its port.
fn spawn_example(workspace: &Path, name: &str, bind: &str) -> ChildGuard {
    let member = workspace.join(name);
    // Binary names follow the member package names, e.g. dir `pages` -> bin
    // `sample-pages`.
    let bin = workspace
        .join("target")
        .join("debug")
        .join(format!("sample-{name}{}", std::env::consts::EXE_SUFFIX));
    let mut child = Command::new(&bin)
        .current_dir(&member)
        .env_remove("DATABASE_URL")
        .env_remove("GLUON_DEV_TEMPLATES")
        .env("GLUON_BIND", bind)
        .env("GLUON_TELEMETRY_DISABLED", "1")
        .env("GLUON_INSECURE_COOKIE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn {} in {}: {error}", bin.display(), member.display()));

    if let Some(stdout) = child.stdout.take() {
        drain_to_void(stdout);
    }
    if let Some(stderr) = child.stderr.take() {
        drain_to_void(stderr);
    }
    ChildGuard::new(child)
}

/// Returns `Ok(())` once the probe URL responds successfully, or `Err` with a
/// reason if the example process exits before becoming ready.
async fn try_wait_until_ready(
    child: &mut ChildGuard,
    client: &reqwest::Client,
    probe_url: &str,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_mins(1);
    loop {
        if let Some(status) = child.try_wait().expect("check example process") {
            return Err(format!(
                "example process exited before readiness with {status}"
            ));
        }
        if let Ok(response) = client.get(probe_url).send().await
            && response.status().is_success()
        {
            return Ok(());
        }
        assert!(
            Instant::now() < deadline,
            "example server failed to become ready within 60s at {probe_url}"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build reqwest client")
}

async fn response_json(response: reqwest::Response, context: &str) -> Value {
    response
        .json()
        .await
        .unwrap_or_else(|error| panic!("decode {context} JSON: {error}"))
}

/// Spawn an example member, wait until its probe URL responds, and return
/// the guarded process, an HTTP client, and the server base URL.
async fn spawn_ready(
    workspace: &Path,
    name: &str,
    probe: &str,
) -> (ChildGuard, reqwest::Client, String) {
    // pick_port releases its listener before the child starts, so another
    // process can claim the port in between.
    // If the child exits early (typically EADDRINUSE), retry on a fresh port
    // before giving up so concurrent CI jobs don't produce flaky failures.
    let http = client();
    let mut last_error = String::new();
    for attempt in 1..=3 {
        let bind = format!("127.0.0.1:{}", pick_port());
        let base = format!("http://{bind}");
        let mut child = spawn_example(workspace, name, &bind);
        match try_wait_until_ready(&mut child, &http, &format!("{base}{probe}")).await {
            Ok(()) => return (child, http, base),
            Err(error) => {
                eprintln!("{name}: start attempt {attempt} failed: {error}");
                last_error = error;
                drop(child);
            }
        }
    }
    panic!("{name} failed to start after 3 attempts: {last_error}");
}

async fn run_pages(workspace: &Path) {
    let _moved_sources = move_app_and_public_sources(&workspace.join("pages"));
    let (_child, http, base) = spawn_ready(workspace, "pages", "/").await;

    let response = http
        .get(format!("{base}/"))
        .send()
        .await
        .expect("GET pages /");
    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("pages home body");
    assert!(body.contains("sample-pages-home"), "home body: {body}");
    assert!(
        body.contains("<span>sample-pages-home</span>"),
        "embedded component output: {body}"
    );
    assert!(
        body.contains("embedded-root:"),
        "embedded root layout: {body}"
    );

    let response = http
        .get(format!("{base}/users/42"))
        .send()
        .await
        .expect("GET pages /users/42");
    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("pages user body");
    assert!(body.contains("sample-pages-user:42"), "user body: {body}");
    assert!(
        body.contains("embedded-root:") && body.contains("embedded-user:"),
        "embedded nested layouts: {body}"
    );

    let response = http
        .get(format!("{base}/feed"))
        .send()
        .await
        .expect("GET pages route template");
    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("pages route body");
    assert!(
        body.contains("<span>sample-pages-route</span>"),
        "embedded route template component: {body}"
    );
    assert!(
        !body.contains("embedded-root:"),
        "route templates must not inherit page layouts: {body}"
    );

    let response = http
        .get(format!("{base}/errors"))
        .send()
        .await
        .expect("GET pages embedded error");
    assert_eq!(response.status(), 404);
    let body = response.text().await.expect("pages error body");
    assert!(
        body.contains("embedded-root:") && body.contains("embedded-error:404"),
        "embedded error page and root layout: {body}"
    );

    let response = http
        .get(format!("{base}/public/marker.txt"))
        .send()
        .await
        .expect("GET pages public marker");
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "text/plain");
    assert_eq!(response.headers()["cache-control"], "public, max-age=3600");
    let body = response.bytes().await.expect("pages marker bytes");
    assert!(
        body.as_ref() == b"sample-pages-public-marker\n"
            || body.as_ref() == b"sample-pages-public-marker\r\n",
        "marker body: {body:?}"
    );

    let response = http
        .get(format!("{base}/does-not-exist"))
        .send()
        .await
        .expect("GET pages unknown path");
    assert_eq!(response.status(), 404);
}

async fn run_api(workspace: &Path) {
    let (_child, http, base) = spawn_ready(workspace, "api", "/api/health").await;

    let response = http
        .get(format!("{base}/api/health"))
        .send()
        .await
        .expect("GET api /api/health");
    assert_eq!(response.status(), 200);
    assert_eq!(
        response_json(response, "health").await,
        json!({"service": "sample-api", "ok": true})
    );

    let response = http
        .get(format!("{base}/api/items/widget?label=Blue%20Widget"))
        .send()
        .await
        .expect("GET api item");
    assert_eq!(response.status(), 200);
    assert_eq!(
        response_json(response, "item").await,
        json!({"id": "widget", "label": "Blue Widget"})
    );

    // `label` is a required query parameter; omitting it must fail extraction.
    let response = http
        .get(format!("{base}/api/items/widget"))
        .send()
        .await
        .expect("GET api item without required label");
    assert_eq!(response.status(), 400);

    let response = http
        .get(format!("{base}/api/unknown"))
        .send()
        .await
        .expect("GET api unknown route");
    assert_eq!(response.status(), 404);
}

async fn run_sessions(workspace: &Path) {
    let (_child, http, base) = spawn_ready(workspace, "sessions", "/token").await;

    let response = http
        .get(format!("{base}/token"))
        .send()
        .await
        .expect("GET sessions /token");
    assert_eq!(response.status(), 200);
    let token_body = response_json(response, "token").await;
    let token = token_body
        .get("token")
        .and_then(Value::as_str)
        .filter(|token| !token.is_empty())
        .expect("token response contains a non-empty token")
        .to_owned();

    let response = http
        .get(format!("{base}/counter"))
        .send()
        .await
        .expect("GET sessions /counter");
    assert_eq!(response.status(), 200);
    assert_eq!(
        response_json(response, "initial counter").await,
        json!({"count": 0})
    );

    let response = http
        .post(format!("{base}/counter"))
        .send()
        .await
        .expect("POST sessions /counter without CSRF token");
    assert_eq!(response.status(), 403);

    let response = http
        .post(format!("{base}/counter"))
        .header("x-csrf-token", &token)
        .send()
        .await
        .expect("POST sessions /counter with CSRF token");
    assert_eq!(response.status(), 200);
    assert_eq!(
        response_json(response, "incremented counter").await,
        json!({"count": 1})
    );

    let response = http
        .get(format!("{base}/counter"))
        .send()
        .await
        .expect("GET sessions /counter after increment");
    assert_eq!(response.status(), 200);
    assert_eq!(
        response_json(response, "final counter").await,
        json!({"count": 1})
    );
}

async fn run_htmx(workspace: &Path) {
    let (_child, http, base) = spawn_ready(workspace, "htmx", "/").await;

    let response = http
        .get(format!("{base}/"))
        .send()
        .await
        .expect("GET htmx /");
    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get(VARY)
            .and_then(|value| value.to_str().ok()),
        Some("HX-Request")
    );
    let body = response.text().await.expect("htmx document body");
    assert!(body.contains("sample-htmx"), "document body: {body}");
    assert!(
        body.contains("<!DOCTYPE html>"),
        "document body lacks doctype: {body}"
    );

    let response = http
        .get(format!("{base}/"))
        .header("HX-Request", "true")
        .send()
        .await
        .expect("GET htmx fragment");
    assert_eq!(response.status(), 200);
    assert_eq!(
        response
            .headers()
            .get(VARY)
            .and_then(|value| value.to_str().ok()),
        Some("HX-Request")
    );
    let body = response.text().await.expect("htmx fragment body");
    assert!(body.contains("sample-htmx"), "fragment body: {body}");
    assert!(
        !body.contains("<!DOCTYPE html>"),
        "fragment unexpectedly has doctype: {body}"
    );
}

async fn run_di(workspace: &Path) {
    let (_child, http, base) = spawn_ready(workspace, "di", "/greet/Ready").await;

    let response = http
        .get(format!("{base}/greet/Ada"))
        .send()
        .await
        .expect("GET di greeting");
    assert_eq!(response.status(), 200);
    assert_eq!(
        response_json(response, "greeting").await,
        json!({"greeting": "Hello, Ada!"})
    );
}

fn auth_command(workspace: &Path, key: &Path, bind: &str) -> Command {
    let bin = workspace
        .join("target/debug")
        .join(format!("sample-auth{}", std::env::consts::EXE_SUFFIX));
    let mut command = Command::new(bin);
    command
        .current_dir(workspace.join("auth"))
        .env_remove("DATABASE_URL")
        .env_remove("AUTH_RP_ID")
        .env_remove("AUTH_ORIGIN")
        .env("AUTH_STORE", "memory-development")
        .env("AUTH_SMTP_SERVER", "smtp.example.test")
        .env("AUTH_SMTP_USERNAME", "fixture")
        .env("AUTH_SMTP_PASSWORD", "fixture")
        .env("AUTH_SMTP_FROM", "auth@example.test")
        .env("AUTH_SMTP_PORT", "587")
        .env(
            "AUTH_OTP_SECRET",
            "test-only-independent-otp-secret-32-bytes",
        )
        .env("AUTH_JWT_PRIVATE_KEY_FILE", key)
        .env("AUTH_JWT_KID", "smoke-key")
        .env("AUTH_JWT_ISSUER", "https://auth.example.test")
        .env("AUTH_JWT_AUDIENCE", "smoke-api")
        .env("GLUON_BIND", bind)
        .env("GLUON_INSECURE_COOKIE", "1")
        .env("GLUON_TELEMETRY_DISABLED", "1")
        .env(
            "SECRET_KEY_BASE",
            "test-only-cookie-key-0000000000000000000000000000000000000000000000",
        )
        .env_remove("AUTH_MIGRATE_ON_START")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

async fn run_auth(workspace: &Path) {
    // Public test-only key. Never use this material in an application.
    let key = tempfile::NamedTempFile::new().expect("temporary JWT key");
    std::fs::write(key.path(), b"-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIEmyZSI2Yqo54etzjA/kMeF9rVI5ixzZ1YKPQEFh9XKt\n-----END PRIVATE KEY-----\n").expect("write test key");
    let bind = format!("127.0.0.1:{}", pick_port());
    let base = format!("http://{bind}");
    let mut command = auth_command(workspace, key.path(), &bind);
    let mut child = command.spawn().expect("spawn screenless auth example");
    if let Some(stdout) = child.stdout.take() {
        drain_to_void(stdout);
    }
    if let Some(stderr) = child.stderr.take() {
        drain_to_void(stderr);
    }
    let mut child = ChildGuard::new(child);
    let http = client();
    try_wait_until_ready(&mut child, &http, &format!("{base}/health"))
        .await
        .expect("auth process readiness");
    // Use a fresh client: public JWKS must not need or create a session.
    let jwks = client()
        .get(format!("{base}/auth/jwks"))
        .send()
        .await
        .expect("public JWKS");
    assert_eq!(jwks.status(), 200);
    assert!(!jwks.headers().contains_key("set-cookie"));
    let jwks = response_json(jwks, "JWKS").await;
    assert_eq!(jwks["keys"][0]["kid"], "smoke-key");
    assert_eq!(jwks["keys"][0]["kty"], "OKP");
    assert!(jwks["keys"][0].get("d").is_none());
    let csrf = http
        .get(format!("{base}/auth/csrf"))
        .send()
        .await
        .expect("auth CSRF");
    assert_eq!(csrf.status(), 200);
    assert_eq!(
        csrf.headers()
            .get("cache-control")
            .expect("auth Cache-Control"),
        "no-store"
    );
    let csrf = response_json(csrf, "auth CSRF").await;
    let token = csrf["token"].as_str().expect("CSRF token");
    assert_ne!(token, "");
    let denied = http
        .post(format!("{base}/auth/token"))
        .send()
        .await
        .expect("token without CSRF");
    assert_eq!(denied.status(), 403);
    let anonymous = http
        .post(format!("{base}/auth/token"))
        .header("x-csrf-token", token)
        .send()
        .await
        .expect("anonymous JWT request");
    assert_eq!(anonymous.status(), 401);
    let session = http
        .get(format!("{base}/auth/session"))
        .send()
        .await
        .expect("anonymous session");
    assert_eq!(session.status(), 200);
    assert_eq!(
        response_json(session, "anonymous session").await,
        json!({"user": null})
    );
    let disabled = http
        .post(format!("{base}/auth/oauth/github/start"))
        .header("x-csrf-token", token)
        .send()
        .await
        .expect("unconfigured provider");
    assert_eq!(disabled.status(), 404);
    // Missing PostgreSQL must fail rather than silently switch to Memory.
    let rejected = command
        .env("AUTH_STORE", "postgres")
        .output()
        .expect("launch missing-database configuration");
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("DATABASE_URL"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checked_in_examples_match_their_contracts() {
    let workspace = repository_root().join("examples");
    restore_stale_moved_sources(&workspace.join("pages"));
    compile_examples(&workspace);

    // Keep servers sequential: each owns a temporary port and session state,
    // and ChildGuard tears it down even if an assertion panics.
    run_pages(&workspace).await;
    run_api(&workspace).await;
    run_sessions(&workspace).await;
    run_htmx(&workspace).await;
    run_di(&workspace).await;
    run_auth(&workspace).await;
}
