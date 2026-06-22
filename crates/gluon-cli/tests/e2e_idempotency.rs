//! Idempotency / post-condition checks for the gluon CLI generators.
//!
//! These tests intentionally avoid `cargo build`. They scaffold a project,
//! exercise each generator/destroy combination, and only inspect the file
//! tree and a few canonical text files. The full-build coverage lives in
//! `e2e.rs`.

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

fn run_gluon_capture(app: &Path, args: &[&str]) -> (bool, String, String) {
    let output = Command::new(gluon_bin())
        .args(args)
        .current_dir(app)
        .output()
        .expect("spawn gluon");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn run_gluon_expect_failure(app: &Path, args: &[&str]) -> String {
    let (success, stdout, stderr) = run_gluon_capture(app, args);
    assert!(
        !success,
        "gluon {args:?} unexpectedly succeeded\nstdout: {stdout}"
    );
    stderr
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
fn generate_usecase_twice_refuses_overwrite() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    run_gluon(&app, &["g", "usecase", "list_users"]);
    let stderr = run_gluon_expect_failure(&app, &["g", "usecase", "list_users"]);
    assert!(stderr.contains("refusing to overwrite"), "stderr: {stderr}");
}

#[test]
fn destroy_then_generate_restores_wiring_byte_equal() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    let wiring_path = app.join("src/wiring.rs");
    let usecase_mod_path = app.join("src/usecases/mod.rs");
    let domain_mod_path = app.join("src/domain/mod.rs");

    let wiring_before = std::fs::read(&wiring_path).unwrap();
    let usecase_mod_before = std::fs::read(&usecase_mod_path).unwrap();
    let domain_mod_before = std::fs::read(&domain_mod_path).unwrap();

    run_gluon(&app, &["g", "usecase", "list_users"]);
    run_gluon(&app, &["g", "domain", "user"]);

    run_gluon_yes(&app, &["d", "usecase", "list_users"]);
    run_gluon_yes(&app, &["d", "domain", "user"]);

    let wiring_after = std::fs::read(&wiring_path).unwrap();
    let usecase_mod_after = std::fs::read(&usecase_mod_path).unwrap();
    let domain_mod_after = std::fs::read(&domain_mod_path).unwrap();

    assert_eq!(
        wiring_before, wiring_after,
        "wiring.rs differs after destroy round-trip"
    );
    assert_eq!(
        usecase_mod_before, usecase_mod_after,
        "usecases/mod.rs differs after destroy round-trip"
    );
    assert_eq!(
        domain_mod_before, domain_mod_after,
        "domain/mod.rs differs after destroy round-trip"
    );
}

#[test]
fn controller_api_flag_skips_tsx() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    run_gluon(&app, &["g", "controller", "api/health", "--api"]);
    assert!(app.join("app/api/health/route.rs").is_file());
    assert!(!app.join("app/api/health/page.rs").exists());
    assert!(!app.join("app/api/health/page.tsx").exists());
}

#[test]
fn controller_without_api_creates_both() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    run_gluon(&app, &["g", "controller", "users"]);
    assert!(app.join("app/users/page.rs").is_file());
    assert!(app.join("app/users/page.tsx").is_file());
    assert!(!app.join("app/users/route.rs").exists());
}

#[test]
fn new_no_install_skips_cargo_lock() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    assert!(!app.join("Cargo.lock").exists());
    assert!(!app.join(".git").exists());
}

#[test]
fn routes_with_no_app_dir_fails() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let stderr = run_gluon_expect_failure(tmp.path(), &["routes"]);
    assert!(
        stderr.contains("app/ directory not found"),
        "stderr: {stderr}"
    );
}

#[test]
fn db_seed_reports_not_implemented() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    let stderr = run_gluon_expect_failure(&app, &["db", "seed"]);
    assert!(stderr.contains("not yet implemented"), "stderr: {stderr}");
}

#[test]
fn destroy_unknown_target_is_idempotent_with_nothing_to_remove() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    let (success, stdout, _stderr) = run_gluon_capture(&app, &["d", "usecase", "foo"]);
    assert!(
        success,
        "gluon d usecase foo should succeed when nothing exists"
    );
    assert!(stdout.contains("nothing to remove"), "stdout: {stdout}");
}
