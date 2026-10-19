//! Destroy-loop edge cases that are too noisy for the main e2e suite.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::allow_attributes
)]

mod common;

use common::{fresh_app, gluon_bin, run_gluon, run_gluon_yes};
use std::process::Command;
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
