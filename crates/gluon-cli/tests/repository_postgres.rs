#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::too_many_lines
)]

use std::path::{Path, PathBuf};
use std::process::Command;

use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

fn gluon_bin() -> &'static str {
    env!("CARGO_BIN_EXE_gluon")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn run(root: &Path, program: &str, args: &[&str], database_url: Option<&str>) {
    let mut command = Command::new(program);
    command.args(args).current_dir(root);
    if let Some(database_url) = database_url {
        command.env("DATABASE_URL", database_url);
    }
    let output = command.output().expect("spawn command");
    assert!(
        output.status.success(),
        "{program} {args:?} failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn generated_postgres_repository_supports_crud() {
    let container = Postgres::default().start().await.expect("start postgres");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres port");
    let database_url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let temp = tempfile::tempdir().expect("tempdir");
    run(
        temp.path(),
        gluon_bin(),
        &["new", "repo_app", "--no-git", "--no-install"],
        None,
    );
    let app = temp.path().join("repo_app");
    let cargo_toml = app.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).unwrap();
    let root = workspace_root();
    let build_path = root.join("crates/gluon-build");
    let gluon_path = root.join("crates/gluon");
    let content = content
        .replace(
            "../gluon/crates/gluon-build",
            &build_path.to_string_lossy().replace('\\', "/"),
        )
        .replace(
            "../gluon/crates/gluon",
            &gluon_path.to_string_lossy().replace('\\', "/"),
        );
    std::fs::write(&cargo_toml, content).unwrap();
    run(
        &app,
        gluon_bin(),
        &[
            "g",
            "domain",
            "user",
            "--field",
            "name:UserName",
            "--field",
            "age:u32",
            "--field",
            "select:String",
            "--field",
            "quota:Option<u64>",
        ],
        None,
    );

    std::fs::create_dir_all(app.join("src/bin")).unwrap();
    std::fs::write(
        app.join("src/bin/repository_probe.rs"),
        r#"
#[path = "../domain/mod.rs"]
mod domain;
#[path = "../infrastructure/mod.rs"]
mod infrastructure;

use std::sync::Arc;
use domain::user::{User, UserId, UserName, UserRepository};
use infrastructure::persistence::user_repository::PostgresUserRepository;

#[tokio::main]
async fn main() {
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap()).await.unwrap();
    sqlx::query("CREATE TABLE users (id UUID PRIMARY KEY, name TEXT NOT NULL, age BIGINT NOT NULL, \"select\" TEXT NOT NULL, quota BIGINT)")
        .execute(&pool).await.unwrap();
    let repository = PostgresUserRepository::new(Arc::new(pool));
    let id = UserId(uuid::Uuid::new_v4());
    let mut user = User {
        id: id.clone(),
        name: UserName("Alice".into()),
        age: 30,
        select: "first".into(),
        quota: None,
    };

    repository.save(&user).await.unwrap();
    let found = repository.find(&id).await.unwrap().unwrap();
    assert_eq!(found.name.0, "Alice");
    assert_eq!(found.age, 30);
    assert_eq!(found.select, "first");
    assert_eq!(found.quota, None);

    user.name = UserName("Bob".into());
    user.age = 31;
    user.select = "updated".into();
    user.quota = Some(32);
    repository.save(&user).await.unwrap();
    let listed = repository.list(10).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name.0, "Bob");
    assert_eq!(listed[0].age, 31);
    assert_eq!(listed[0].select, "updated");
    assert_eq!(listed[0].quota, Some(32));

    user.quota = Some(u64::MAX);
    assert!(repository.save(&user).await.is_err());

    repository.delete(&id).await.unwrap();
    assert!(repository.find(&id).await.unwrap().is_none());
}
"#,
    )
    .unwrap();

    run(
        &app,
        "cargo",
        &["run", "--quiet", "--bin", "repository_probe"],
        Some(&database_url),
    );
}
