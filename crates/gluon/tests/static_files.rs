#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

//! Integration tests confirming that `gluon::middleware::static_files::service`
//! serves files from disk while keeping directory indexing disabled.

use std::fs;

use axum::Router;
use axum_test::TestServer;
use gluon::middleware::static_files;
use tempfile::TempDir;

fn server_for(public_dir: &std::path::Path) -> TestServer {
    let service = static_files::service(public_dir.to_path_buf());
    let app = Router::new().nest_service("/public", service);
    TestServer::new(app)
}

#[tokio::test]
async fn serves_existing_file() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("hello.txt"), "hello world").expect("write hello.txt");

    let response = server_for(dir.path()).get("/public/hello.txt").await;
    response.assert_status_ok();
    response.assert_text("hello world");
}

#[tokio::test]
async fn directory_index_disabled() {
    let dir = TempDir::new().expect("tempdir");
    let sub = dir.path().join("subdir");
    fs::create_dir_all(&sub).expect("mkdir subdir");
    fs::write(sub.join("index.html"), "<h1>nope</h1>").expect("write index.html");

    // `ServeDir::append_index_html_on_directories(false)` should prevent the
    // service from auto-serving `subdir/index.html` for the directory URL.
    let response = server_for(dir.path()).get("/public/subdir/").await;
    assert_eq!(response.status_code(), http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn nonexistent_returns_404() {
    let dir = TempDir::new().expect("tempdir");

    let response = server_for(dir.path()).get("/public/").await;
    assert_eq!(response.status_code(), http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn root_created_after_service_is_available() {
    let parent = TempDir::new().expect("parent tempdir");
    let public = parent.path().join("public");
    let server = server_for(&public);
    fs::create_dir(&public).expect("create public");
    fs::write(public.join("late.txt"), "available").expect("write late file");

    let response = server.get("/public/late.txt").await;
    response.assert_status_ok();
    response.assert_text("available");
}

#[cfg(unix)]
#[tokio::test]
async fn symlink_cannot_escape_public_directory() {
    use std::os::unix::fs::symlink;

    let public = TempDir::new().expect("public tempdir");
    let private = TempDir::new().expect("private tempdir");
    fs::write(private.path().join("secret.txt"), "secret").expect("write secret");
    symlink(
        private.path().join("secret.txt"),
        public.path().join("leak.txt"),
    )
    .expect("create symlink");

    let response = server_for(public.path()).get("/public/leak.txt").await;
    assert_eq!(response.status_code(), http::StatusCode::NOT_FOUND);
    assert!(!response.text().contains("secret"));
}
