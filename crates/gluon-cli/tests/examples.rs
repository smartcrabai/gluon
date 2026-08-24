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
/// at compile time — no need to probe the filesystem for it.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("gluon-cli lives two levels below the repository root")
        .to_path_buf()
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
    let deadline = Instant::now() + Duration::from_secs(60);
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
    let (_child, http, base) = spawn_ready(workspace, "pages", "/").await;

    let response = http
        .get(format!("{base}/"))
        .send()
        .await
        .expect("GET pages /");
    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("pages home body");
    assert!(body.contains("sample-pages-home"), "home body: {body}");

    let response = http
        .get(format!("{base}/users/42"))
        .send()
        .await
        .expect("GET pages /users/42");
    assert_eq!(response.status(), 200);
    let body = response.text().await.expect("pages user body");
    assert!(body.contains("sample-pages-user:42"), "user body: {body}");

    let response = http
        .get(format!("{base}/public/marker.txt"))
        .send()
        .await
        .expect("GET pages public marker");
    assert_eq!(response.status(), 200);
    let body = response.bytes().await.expect("pages marker bytes");
    assert_eq!(body.as_ref(), b"sample-pages-public-marker\n");

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checked_in_examples_match_their_contracts() {
    let workspace = repository_root().join("examples");
    compile_examples(&workspace);

    // Keep servers sequential: each owns a temporary port and session state,
    // and ChildGuard tears it down even if an assertion panics.
    run_pages(&workspace).await;
    run_api(&workspace).await;
    run_sessions(&workspace).await;
    run_htmx(&workspace).await;
    run_di(&workspace).await;
}
