//! HTTP smoke test: scaffold a project, build it, spawn it, and probe a few
//! routes over real HTTP.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::allow_attributes
)]

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn gluon_bin() -> &'static str {
    env!("CARGO_BIN_EXE_gluon")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("workspace root")
}

fn fix_paths(cargo_toml: &Path) {
    let root = workspace_root();
    let gluon_path = root.join("crates/gluon");
    let build_path = root.join("crates/gluon-build");
    let content = std::fs::read_to_string(cargo_toml).expect("read Cargo.toml");
    let fixed = content
        .replace(
            "../gluon/crates/gluon-build",
            build_path.to_str().expect("build path utf8"),
        )
        .replace(
            "../gluon/crates/gluon",
            gluon_path.to_str().expect("gluon path utf8"),
        );
    std::fs::write(cargo_toml, fixed).expect("write Cargo.toml");
}

fn run_gluon(app: &Path, args: &[&str]) {
    let output = Command::new(gluon_bin())
        .args(args)
        .current_dir(app)
        .output()
        .expect("spawn gluon");
    assert!(
        output.status.success(),
        "gluon {args:?} failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fresh_app(tmp: &Path, name: &str) -> PathBuf {
    run_gluon(tmp, &["new", name, "--no-git", "--no-install"]);
    let app = tmp.join(name);
    fix_paths(&app.join("Cargo.toml"));
    app
}

fn run_cargo_build(app: &Path) {
    let output = Command::new("cargo")
        .args(["build", "--quiet"])
        .current_dir(app)
        .output()
        .expect("spawn cargo build");
    assert!(
        output.status.success(),
        "cargo build failed in {}\nstdout: {}\nstderr: {}",
        app.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Bind a random free TCP port, then immediately drop the listener so the
/// child process can grab the same port.
fn pick_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("local_addr").port()
}

/// Drains a child's stdio in a background thread so its pipe buffer can never
/// fill up and stall the server.
fn drain_to_void<R: std::io::Read + Send + 'static>(reader: R) {
    thread::spawn(move || {
        let mut buf = BufReader::new(reader);
        let mut line = String::new();
        loop {
            line.clear();
            match buf.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    });
}

/// Guard that kills the child process when dropped, so an `assert!` failure
/// never leaves a stray server bound to the test port.
struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self(Some(child))
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

async fn wait_until_ready(client: &reqwest::Client, base: &str) {
    let deadline = Instant::now() + Duration::from_mins(1);
    let probe_url = format!("{base}/");
    loop {
        if let Ok(resp) = client.get(&probe_url).send().await
            && resp.status().is_success()
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "server failed to become ready within 60s at {base}"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

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
