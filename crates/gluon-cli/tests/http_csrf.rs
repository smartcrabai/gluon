//! HTTP-level CSRF middleware probe. Confirms that a state-changing request
//! without a valid token is rejected with 403 before it reaches the route.

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

fn pick_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("local_addr").port()
}

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
