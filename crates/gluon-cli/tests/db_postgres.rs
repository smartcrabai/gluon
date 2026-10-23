#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use sqlx::migrate::MigrateDatabase;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

fn gluon_bin() -> &'static str {
    env!("CARGO_BIN_EXE_gluon")
}

fn run_gluon(root: &Path, database_url: &str, args: &[&str]) {
    let output = Command::new(gluon_bin())
        .args(args)
        .current_dir(root)
        .env("DATABASE_URL", database_url)
        .output()
        .expect("spawn gluon");
    assert!(
        output.status.success(),
        "gluon {args:?} failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

async fn failed_migration_rolls_back_and_can_be_retried(root: &Path, database_url: &str) {
    std::fs::write(
        root.join("migrations/20260101000001_create_item_notes.up.sql"),
        "CREATE TABLE item_notes (id BIGSERIAL PRIMARY KEY, body TEXT NOT NULL);\n\
         INSERT INTO item_notes (body) VALUES ('must roll back');\n\
         ALTER TABLE items ADD COLUMN migration_attempt TEXT;\n\
         INSERT INTO items (name, migration_attempt) VALUES ('must roll back', 'changed');\n\
         INSERT INTO item_notes (body) VALUES (NULL);",
    )
    .unwrap();
    std::fs::write(
        root.join("migrations/20260101000001_create_item_notes.down.sql"),
        "DROP TABLE item_notes;",
    )
    .unwrap();

    let output = Command::new(gluon_bin())
        .args(["db", "migrate"])
        .current_dir(root)
        .env("DATABASE_URL", database_url)
        .output()
        .expect("spawn gluon");
    assert!(
        !output.status.success(),
        "gluon db migrate unexpectedly succeeded\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let pool = sqlx::PgPool::connect(database_url).await.unwrap();
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM items ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(names, ["seeded"]);
    let failed_table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name = 'item_notes')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!failed_table_exists);
    let failed_column_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = 'items' \
         AND column_name = 'migration_attempt')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!failed_column_exists);
    let applied: Vec<(i64, bool)> =
        sqlx::query_as("SELECT version, success FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(applied, [(20_260_101_000_000, true)]);
    pool.close().await;

    std::fs::write(
        root.join("migrations/20260101000001_create_item_notes.up.sql"),
        "CREATE TABLE item_notes (id BIGSERIAL PRIMARY KEY, body TEXT NOT NULL);",
    )
    .unwrap();
    run_gluon(root, database_url, &["db", "migrate"]);
    let pool = sqlx::PgPool::connect(database_url).await.unwrap();
    let item_notes_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name = 'item_notes')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(item_notes_exists);
    pool.close().await;
}

#[tokio::test]
async fn database_lifecycle_and_seed_work_against_postgres() {
    let container = Postgres::default().start().await.expect("start postgres");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres port");
    let database_url = format!("postgres://postgres:postgres@127.0.0.1:{port}/gluon_cli_test");
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(root.path().join("migrations")).unwrap();
    std::fs::create_dir_all(root.path().join("db")).unwrap();
    std::fs::write(
        root.path()
            .join("migrations/20260101000000_create_items.up.sql"),
        "CREATE TABLE items (id BIGSERIAL PRIMARY KEY, name TEXT NOT NULL);",
    )
    .unwrap();
    std::fs::write(
        root.path()
            .join("migrations/20260101000000_create_items.down.sql"),
        "DROP TABLE items;",
    )
    .unwrap();
    std::fs::write(
        root.path().join("db/seeds.sql"),
        "INSERT INTO items (name) VALUES ('seeded');",
    )
    .unwrap();

    run_gluon(root.path(), &database_url, &["db", "create"]);
    assert!(
        sqlx::Postgres::database_exists(&database_url)
            .await
            .unwrap()
    );
    run_gluon(root.path(), &database_url, &["db", "rollback"]);

    run_gluon(root.path(), &database_url, &["db", "migrate"]);
    run_gluon(root.path(), &database_url, &["db", "seed"]);
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM items ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(names, ["seeded"]);
    pool.close().await;
    failed_migration_rolls_back_and_can_be_retried(root.path(), &database_url).await;

    run_gluon(root.path(), &database_url, &["db", "rollback"]);
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name IN ('items', 'item_notes') \
         ORDER BY table_name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(tables, ["items"]);
    pool.close().await;

    run_gluon(root.path(), &database_url, &["db", "rollback"]);
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'items')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!exists);
    pool.close().await;

    run_gluon(root.path(), &database_url, &["db", "drop"]);
    assert!(
        !sqlx::Postgres::database_exists(&database_url)
            .await
            .unwrap()
    );
}
