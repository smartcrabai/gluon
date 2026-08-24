//! Process and port helpers shared by the black-box HTTP integration tests.

#![allow(clippy::expect_used, clippy::missing_panics_doc, dead_code)]

use std::io::BufReader;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

pub fn gluon_bin() -> &'static str {
    env!("CARGO_BIN_EXE_gluon")
}

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("workspace root")
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn fix_paths(cargo_toml: &Path) {
    let root = workspace_root();
    let gluon_path = root.join("crates/gluon");
    let build_path = root.join("crates/gluon-build");
    let content = std::fs::read_to_string(cargo_toml).expect("read Cargo.toml");
    let fixed = content
        .replace("../gluon/crates/gluon-build", &toml_path(&build_path))
        .replace("../gluon/crates/gluon", &toml_path(&gluon_path));
    std::fs::write(cargo_toml, fixed).expect("write Cargo.toml");
}

pub fn run_gluon(app: &Path, args: &[&str]) {
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

pub fn fresh_app(tmp: &Path, name: &str) -> PathBuf {
    run_gluon(tmp, &["new", name, "--no-git", "--no-install"]);
    let app = tmp.join(name);
    fix_paths(&app.join("Cargo.toml"));
    app
}

pub fn run_cargo_build(app: &Path) {
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

pub async fn wait_until_ready(client: &reqwest::Client, base: &str) {
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

/// Bind a random free TCP port, then immediately drop the listener so the
/// child process can grab the same port.
///
/// Note this is inherently racy: another process may claim the port between
/// release and the child's bind. Callers should treat an early child exit as a
/// possible bind collision and retry on a fresh port.
pub fn pick_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("read ephemeral port").port()
}

/// Drain process output in a background thread so a noisy server cannot block
/// on a full pipe while the test probes it.
pub fn drain_to_void<R: std::io::Read + Send + 'static>(reader: R) {
    thread::spawn(move || {
        let _ = std::io::copy(&mut BufReader::new(reader), &mut std::io::sink());
    });
}

/// Guard that kills the child process when dropped, so an `assert!` failure
/// never leaves a stray server bound to the test port.
pub struct ChildGuard(Option<Child>);

impl ChildGuard {
    pub fn new(child: Child) -> Self {
        Self(Some(child))
    }

    /// Non-blocking exit check; returns `Some(status)` once the child exits.
    pub fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.0
            .as_mut()
            .expect("child guard still owns process")
            .try_wait()
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
