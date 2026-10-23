//! Destroy-loop edge cases that are too noisy for the main e2e suite.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::allow_attributes
)]

mod common;

use common::{fresh_app, gluon_bin, run_gluon, run_gluon_yes};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Sleep until just after the next wall-clock second so that two back-to-back
/// generator calls share the same `%Y%m%d%H%M%S` timestamp prefix.
fn sleep_until_next_second() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch");
    let remaining = 1_000_000_000 - now.subsec_nanos();
    std::thread::sleep(Duration::from_nanos(u64::from(remaining)) + Duration::from_millis(5));
}

#[test]
fn migrations_in_same_second_collide() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");
    let migrations = app.join("migrations");

    for _ in 0..5 {
        sleep_until_next_second();
        run_gluon(&app, &["g", "migration", "foo"]);
        let output = Command::new(gluon_bin())
            .args(["g", "migration", "foo"])
            .current_dir(&app)
            .output()
            .expect("spawn gluon");
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("refusing to overwrite"), "stderr: {stderr}");
            return;
        }
        // Slow runner straddled a second boundary: clean up and retry.
        for entry in std::fs::read_dir(&migrations).expect("read migrations") {
            let path = entry.expect("dir entry").path();
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().contains("_foo."))
            {
                std::fs::remove_file(&path).expect("remove migration");
            }
        }
    }
    panic!("back-to-back `gluon g migration foo` never collided within 5 attempts");
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

const USECASE_FILES: &[&str] = &[
    "src/usecases/check_cancel.rs",
    "src/usecases/mod.rs",
    "src/wiring.rs",
];
const DOMAIN_FILES: &[&str] = &[
    "src/domain/check_cancel/mod.rs",
    "src/domain/check_cancel/entity.rs",
    "src/domain/check_cancel/value_objects.rs",
    "src/domain/check_cancel/repository.rs",
    "src/domain/check_cancel/error.rs",
    "src/infrastructure/persistence/check_cancel_repository.rs",
    "src/infrastructure/mocks/check_cancel_repository.rs",
    "src/domain/mod.rs",
    "src/infrastructure/persistence/mod.rs",
    "src/infrastructure/mocks/mod.rs",
    "src/wiring.rs",
];
const DTO_FILES: &[&str] = &["src/dto/check_cancel.rs", "src/dto/mod.rs"];
const DESTROY_CASES: &[(&str, &[&str])] = &[
    ("usecase", USECASE_FILES),
    ("domain", DOMAIN_FILES),
    ("dto", DTO_FILES),
];

#[test]
fn destroy_declined_confirmation_preserves_generated_files_and_registrations() {
    for &(kind, files) in DESTROY_CASES {
        assert_destroy_cancelled_without_changes(kind, files, Some(b"n\n"));
    }
}

#[test]
fn destroy_eof_preserves_generated_files_and_registrations() {
    for &(kind, files) in DESTROY_CASES {
        assert_destroy_cancelled_without_changes(kind, files, None);
    }
}

#[test]
fn destroy_later_decline_keeps_every_target_unchanged() {
    assert_destroy_cancelled_without_changes("domain", DOMAIN_FILES, Some(b"y\nn\n"));
}

fn assert_destroy_cancelled_without_changes(kind: &str, files: &[&str], input: Option<&[u8]>) {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");
    run_gluon(&app, &["g", kind, "check_cancel"]);

    let before: Vec<_> = files
        .iter()
        .map(|relative| {
            let path = app.join(*relative);
            assert!(path.is_file(), "expected generated file {}", path.display());
            (
                path.clone(),
                std::fs::read(&path).expect("read generated file"),
            )
        })
        .collect();

    let output = run_gluon_with_stdin(&app, &["d", kind, "check_cancel"], input);
    assert!(
        output.status.success(),
        "gluon destroy {kind} failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    for (path, expected) in before {
        assert_eq!(
            std::fs::read(&path).expect("read file after cancelled destroy"),
            expected,
            "{} changed after cancelled destroy",
            path.display()
        );
    }
}

fn run_gluon_with_stdin(
    app: &std::path::Path,
    args: &[&str],
    input: Option<&[u8]>,
) -> std::process::Output {
    let mut child = Command::new(gluon_bin())
        .args(args)
        .current_dir(app)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gluon");
    if let Some(input) = input {
        child
            .stdin
            .as_mut()
            .expect("piped stdin")
            .write_all(input)
            .expect("write stdin");
    }
    drop(child.stdin.take());
    child.wait_with_output().expect("wait for gluon")
}
