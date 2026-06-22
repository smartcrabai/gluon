//! Destroy-loop edge cases that are too noisy for the main e2e suite.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::allow_attributes
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

fn run_gluon_expect_failure(app: &Path, args: &[&str]) -> String {
    let output = Command::new(gluon_bin())
        .args(args)
        .current_dir(app)
        .output()
        .expect("spawn gluon");
    assert!(
        !output.status.success(),
        "gluon {args:?} unexpectedly succeeded\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn run_gluon_yes(app: &Path, args: &[&str]) {
    use std::io::Write;
    let mut child = Command::new(gluon_bin())
        .args(args)
        .current_dir(app)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gluon");
    if let Some(mut stdin) = child.stdin.take() {
        for _ in 0..50 {
            let _ = stdin.write_all(b"y\n");
        }
    }
    let output = child.wait_with_output().expect("wait gluon");
    assert!(
        output.status.success(),
        "gluon {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fresh_app(tmp: &Path, name: &str) -> PathBuf {
    run_gluon(tmp, &["new", name, "--no-git", "--no-install"]);
    let app = tmp.join(name);
    fix_paths(&app.join("Cargo.toml"));
    app
}

#[test]
fn migrations_in_same_second_collide() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    run_gluon(&app, &["g", "migration", "foo"]);
    // Intentionally no sleep: the timestamp prefix has second granularity,
    // so a back-to-back call must surface a "refusing to overwrite" error.
    let stderr = run_gluon_expect_failure(&app, &["g", "migration", "foo"]);
    assert!(stderr.contains("refusing to overwrite"), "stderr: {stderr}");
}

#[test]
fn destroy_resource_cleans_empty_dirs() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    run_gluon(&app, &["g", "resource", "posts"]);
    run_gluon_yes(&app, &["d", "resource", "posts"]);

    assert!(
        !app.join("app/posts").exists(),
        "app/posts should be removed"
    );
    assert!(
        !app.join("app/api/posts").exists(),
        "app/api/posts should be removed"
    );
}
