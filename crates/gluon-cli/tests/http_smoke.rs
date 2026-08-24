//! HTTP smoke test: scaffold a project, build it, spawn it, and probe a few
//! routes over real HTTP.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::allow_attributes
)]

use std::process::{Command, Stdio};
use std::time::Duration;

mod common;
use common::{
    ChildGuard, drain_to_void, fresh_app, pick_port, run_cargo_build, run_gluon, wait_until_ready,
};
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_smoke_serves_basic_routes() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    run_gluon(&app, &["g", "controller", "users"]);
    run_gluon(&app, &["g", "controller", "api/health", "--api"]);

    run_cargo_build(&app);

    let port = pick_port();
    let bind = format!("127.0.0.1:{port}");
    let base = format!("http://{bind}");

    let mut child = Command::new("cargo")
        .args(["run", "--quiet"])
        .current_dir(&app)
        .env("GLUON_TELEMETRY_DISABLED", "1")
        .env("GLUON_INSECURE_COOKIE", "1")
        .env("GLUON_BIND", &bind)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn cargo run");

    if let Some(stdout) = child.stdout.take() {
        drain_to_void(stdout);
    }
    if let Some(stderr) = child.stderr.take() {
        drain_to_void(stderr);
    }

    let guard = ChildGuard::new(child);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build reqwest client");

    wait_until_ready(&client, &base).await;

    // 1. GET /
    let resp = client.get(format!("{base}/")).send().await.expect("GET /");
    assert_eq!(resp.status().as_u16(), 200, "GET / status");
    let body = resp.text().await.expect("body");
    assert!(
        body.contains("Hello, gluon") || body.contains("HomeProps") || !body.is_empty(),
        "GET / body unexpected: {body}"
    );

    // 2. GET /users
    let resp = client
        .get(format!("{base}/users"))
        .send()
        .await
        .expect("GET /users");
    assert_eq!(resp.status().as_u16(), 200, "GET /users status");
    let body = resp.text().await.expect("users body");
    assert!(!body.is_empty(), "GET /users body should not be empty");

    // 3. GET /api/health
    let resp = client
        .get(format!("{base}/api/health"))
        .send()
        .await
        .expect("GET /api/health");
    assert_eq!(resp.status().as_u16(), 200, "GET /api/health status");
    let body = resp.text().await.expect("health body");
    assert!(body.contains("\"ok\":true"), "GET /api/health body: {body}");

    // 4. GET /nonexistent
    let resp = client
        .get(format!("{base}/nonexistent"))
        .send()
        .await
        .expect("GET /nonexistent");
    assert_eq!(resp.status().as_u16(), 404, "GET /nonexistent status");

    drop(guard);
}
