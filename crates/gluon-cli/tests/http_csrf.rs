//! HTTP-level CSRF middleware probe. Confirms that a state-changing request
//! without a valid token is rejected with 403 before it reaches the route.

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
async fn csrf_blocks_state_changing_without_token() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    run_gluon(&app, &["g", "controller", "api/csrf-probe", "--api"]);

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
        .cookie_store(true)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build reqwest client");

    wait_until_ready(&client, &base).await;

    // GET /api/csrf-probe -- safe method, sets the session cookie.
    let resp = client
        .get(format!("{base}/api/csrf-probe"))
        .send()
        .await
        .expect("GET /api/csrf-probe");
    assert_eq!(resp.status().as_u16(), 200, "GET status");

    // POST with no token at all -> 403.
    let resp = client
        .post(format!("{base}/api/csrf-probe"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body("payload=x")
        .send()
        .await
        .expect("POST without token");
    assert_eq!(
        resp.status().as_u16(),
        403,
        "POST without token should be 403"
    );

    // POST with a wrong token -> 403.
    let resp = client
        .post(format!("{base}/api/csrf-probe"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body("_csrf=not-the-real-token&payload=x")
        .send()
        .await
        .expect("POST with wrong token");
    assert_eq!(
        resp.status().as_u16(),
        403,
        "POST with wrong token should be 403"
    );

    drop(guard);
}
