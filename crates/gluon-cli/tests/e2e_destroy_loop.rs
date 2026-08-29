//! Destroy-loop edge cases that are too noisy for the main e2e suite.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::allow_attributes
)]

mod common;

use common::{fresh_app, run_gluon, run_gluon_expect_failure, run_gluon_yes};

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
