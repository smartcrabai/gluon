//! End-to-end tests for the gluon CLI.
//!
//! Each scenario materializes a fresh gluon application in a temporary
//! directory, runs the real `gluon` binary against it, and confirms the
//! generated workspace still compiles after every `generate` / `destroy`
//! step. The full-lifecycle test uses `cargo check` between steps for speed
//! and a final `cargo build` for the strongest signal.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

use common::{
    fresh_app, gluon_bin, run_cargo_build, run_gluon, run_gluon_expect_failure, run_gluon_yes,
    workspace_root,
};

fn run_cargo_check(app: &Path) {
    let output = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(app)
        .output()
        .expect("spawn cargo check");
    assert!(
        output.status.success(),
        "cargo check failed in {}\nstdout: {}\nstderr: {}",
        app.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn new_agent_flags_install_expected_files() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let expected_skill = std::fs::read_to_string(workspace_root().join("skills/gluon/SKILL.md"))
        .expect("read skill");

    run_gluon(
        tmp.path(),
        &["new", "claude_only", "--claude", "--no-git", "--no-install"],
    );
    let claude_only = tmp.path().join("claude_only");
    assert!(claude_only.join(".claude/skills/gluon/SKILL.md").is_file());
    assert!(!claude_only.join(".agents").exists());
    assert!(!claude_only.join("AGENTS.md").exists());
    assert_eq!(
        std::fs::read_to_string(claude_only.join("CLAUDE.md")).expect("read CLAUDE.md"),
        expected_skill
    );

    run_gluon(
        tmp.path(),
        &["new", "agents_only", "--agents", "--no-git", "--no-install"],
    );
    let agents_only = tmp.path().join("agents_only");
    assert!(agents_only.join(".agents/skills/gluon/SKILL.md").is_file());
    assert!(!agents_only.join(".claude").exists());
    assert_eq!(
        std::fs::read_to_string(agents_only.join("AGENTS.md")).expect("read AGENTS.md"),
        expected_skill
    );
    assert!(!agents_only.join("CLAUDE.md").exists());

    run_gluon(
        tmp.path(),
        &[
            "new",
            "both",
            "--claude",
            "--agents",
            "--no-git",
            "--no-install",
        ],
    );
    let both = tmp.path().join("both");
    assert!(both.join(".agents/skills/gluon/SKILL.md").is_file());
    assert_eq!(
        std::fs::read_to_string(both.join("AGENTS.md")).expect("read AGENTS.md"),
        expected_skill
    );
    assert_eq!(
        std::fs::read_to_string(both.join(".claude/skills/gluon/SKILL.md"))
            .expect("read linked skill"),
        expected_skill
    );
    assert_eq!(
        std::fs::read_to_string(both.join("CLAUDE.md")).expect("read CLAUDE.md"),
        "@AGENTS.md\n"
    );
    let link = both.join(".claude/skills/gluon");
    #[cfg(unix)]
    {
        assert!(
            std::fs::symlink_metadata(&link)
                .expect("read skill link")
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            std::fs::read_link(&link).expect("read skill link target"),
            PathBuf::from("../../.agents/skills/gluon")
        );
    }
    assert!(link.join("SKILL.md").is_file());
}

/// Walks the project through every generator and a full destroy round-trip,
/// asserting that each step keeps the workspace compilable.
#[test]
fn full_lifecycle_builds_after_each_generate() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    // Baseline.
    run_cargo_check(&app);

    // g controller (page)
    run_gluon(&app, &["g", "controller", "users"]);
    assert!(app.join("app/users/page.rs").is_file());
    assert!(app.join("app/users/page.tsx").is_file());
    run_cargo_check(&app);

    // g controller (api)
    run_gluon(&app, &["g", "controller", "api/health", "--api"]);
    assert!(app.join("app/api/health/route.rs").is_file());
    run_cargo_check(&app);

    // g usecase: file + mod.rs marker + wiring bind
    run_gluon(&app, &["g", "usecase", "list_users"]);
    assert!(app.join("src/usecases/list_users.rs").is_file());
    let mods = std::fs::read_to_string(app.join("src/usecases/mod.rs")).unwrap();
    assert!(mods.contains("pub mod list_users;"), "mod.rs: {mods}");
    let wiring = std::fs::read_to_string(app.join("src/wiring.rs")).unwrap();
    assert!(
        wiring.contains("<gluon:bind:usecase:list_users>"),
        "wiring.rs: {wiring}"
    );
    run_cargo_check(&app);

    // g domain: entity, value_objects, repository, infra impl, mock, bind
    run_gluon(
        &app,
        &[
            "g",
            "domain",
            "user",
            "--field",
            "name:UserName",
            "--field",
            "email:Email",
            "--field",
            "samples:Vec<u32>",
        ],
    );
    assert!(app.join("src/domain/user/entity.rs").is_file());
    assert!(app.join("src/domain/user/value_objects.rs").is_file());
    assert!(app.join("src/domain/user/repository.rs").is_file());
    assert!(
        app.join("src/infrastructure/persistence/user_repository.rs")
            .is_file()
    );
    assert!(
        app.join("src/infrastructure/mocks/user_repository.rs")
            .is_file()
    );
    let wiring = std::fs::read_to_string(app.join("src/wiring.rs")).unwrap();
    assert!(wiring.contains("<gluon:bind:domain:user>"));
    run_cargo_check(&app);

    // g dto
    run_gluon(&app, &["g", "dto", "user_dto"]);
    assert!(app.join("src/dto/user_dto.rs").is_file());
    let dto_mod = std::fs::read_to_string(app.join("src/dto/mod.rs")).unwrap();
    assert!(dto_mod.contains("pub mod user_dto;"));
    run_cargo_check(&app);

    // g migration (does not affect build, but verify the file shape)
    run_gluon(&app, &["g", "migration", "create_users"]);
    let migration_files: Vec<_> = std::fs::read_dir(app.join("migrations"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .ends_with("_create_users.up.sql")
        })
        .collect();
    assert_eq!(migration_files.len(), 1, "expected one up.sql");

    // g resource: index/new/show/edit pages + api routes
    run_gluon(&app, &["g", "resource", "posts"]);
    for relative in [
        "app/posts/page.rs",
        "app/posts/new/page.rs",
        "app/posts/[id]/page.rs",
        "app/posts/[id]/edit/page.rs",
        "app/api/posts/route.rs",
        "app/api/posts/[id]/route.rs",
    ] {
        assert!(app.join(relative).is_file(), "missing {relative}");
    }
    run_cargo_check(&app);

    // Final full build before teardown.
    run_cargo_build(&app);

    // Destroy round-trip in reverse.
    run_gluon_yes(&app, &["d", "resource", "posts"]);
    assert!(!app.join("app/posts/page.rs").exists());
    assert!(!app.join("app/api/posts/route.rs").exists());

    run_gluon_yes(&app, &["d", "controller", "users"]);
    run_gluon_yes(&app, &["d", "controller", "api/health"]);
    run_gluon_yes(&app, &["d", "usecase", "list_users"]);
    run_gluon_yes(&app, &["d", "domain", "user"]);
    run_gluon_yes(&app, &["d", "dto", "user_dto"]);
    run_gluon_yes(&app, &["d", "migration", "create_users"]);

    // wiring.rs must be back to a clean state.
    let wiring = std::fs::read_to_string(app.join("src/wiring.rs")).unwrap();
    assert!(!wiring.contains("<gluon:bind:usecase:"));
    assert!(!wiring.contains("<gluon:bind:domain:"));

    run_cargo_build(&app);
}

#[test]
fn validation_rejects_unsafe_inputs() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    let stderr = run_gluon_expect_failure(&app, &["g", "controller", "../../etc/passwd"]);
    assert!(stderr.contains("invalid route segment"), "stderr: {stderr}");

    let stderr = run_gluon_expect_failure(&app, &["g", "usecase", "foo;}; fn evil()"]);
    assert!(stderr.contains("invalid usecase name"), "stderr: {stderr}");

    let stderr = run_gluon_expect_failure(
        &app,
        &["g", "domain", "user", "--field", "id:String;} fn bad"],
    );
    assert!(stderr.contains("invalid field type"), "stderr: {stderr}");

    let stderr = run_gluon_expect_failure(&app, &["g", "domain", "1user"]);
    assert!(
        stderr.contains("must start with a letter"),
        "stderr: {stderr}"
    );
}

#[test]
fn destroy_migration_uses_exact_match() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    run_gluon(&app, &["g", "migration", "create_users"]);
    // Timestamps include UTC seconds, so sleep briefly to avoid collision.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    run_gluon(&app, &["g", "migration", "add_users"]);

    // "users" must NOT match either suffix-only.
    let stderr = run_gluon_expect_failure(&app, &["d", "migration", "users"]);
    assert!(stderr.contains("no migration matched"), "stderr: {stderr}");

    // Both migrations are still present.
    let sql_count = std::fs::read_dir(app.join("migrations"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".sql"))
        .count();
    assert_eq!(sql_count, 4, "expected 2 migrations * (up+down)");
}

#[test]
fn routes_command_lists_generated_routes() {
    let tmp = tempfile::tempdir().expect("create tempdir");
    let app = fresh_app(tmp.path(), "myapp");

    run_gluon(&app, &["g", "controller", "users"]);
    run_gluon(&app, &["g", "controller", "api/health", "--api"]);

    let output = Command::new(gluon_bin())
        .args(["routes"])
        .current_dir(&app)
        .output()
        .expect("spawn gluon routes");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("/users"), "{stdout}");
    assert!(stdout.contains("/api/health"), "{stdout}");
    assert!(stdout.lines().any(|l| l.starts_with("GET")));
}
