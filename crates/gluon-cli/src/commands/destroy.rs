//! Implementation of `gluon destroy` / `gluon d`.
//!
//! Removes files previously produced by `gluon generate`, plus the
//! corresponding entries in `src/wiring.rs` and the per-layer `mod.rs` files.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use heck::ToSnakeCase;

use crate::DestroyKind;
use crate::commands::generate::{validate_identifier, validate_route};

pub fn run(kind: DestroyKind) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    match kind {
        DestroyKind::Controller { route } => destroy_controller(&cwd, &route),
        DestroyKind::Resource { name } => destroy_resource(&cwd, &name),
        DestroyKind::Usecase { name } => destroy_usecase(&cwd, &name),
        DestroyKind::Domain { name } => destroy_domain(&cwd, &name),
        DestroyKind::Dto { name } => destroy_dto(&cwd, &name),
        DestroyKind::Migration { name } => destroy_migration(&cwd, &name),
    }
}

fn destroy_controller(root: &Path, route: &str) -> Result<()> {
    let trimmed = route.trim_matches('/');
    validate_route(trimmed)?;
    let dir = route_to_app_dir(root, trimmed);
    let mut paths = vec![
        dir.join("page.rs"),
        dir.join("page.tsx"),
        dir.join("route.rs"),
    ];
    confirm_and_remove(&mut paths)?;
    remove_empty_dir(&dir);
    Ok(())
}

fn destroy_resource(root: &Path, name: &str) -> Result<()> {
    validate_identifier(name, "resource")?;
    let snake = name.to_snake_case();
    let base = root.join("app").join(&snake);
    let api_base = root.join("app").join("api").join(&snake);
    let mut paths = vec![
        base.join("page.rs"),
        base.join("page.tsx"),
        base.join("new").join("page.rs"),
        base.join("new").join("page.tsx"),
        base.join("[id]").join("page.rs"),
        base.join("[id]").join("page.tsx"),
        base.join("[id]").join("edit").join("page.rs"),
        base.join("[id]").join("edit").join("page.tsx"),
        // JSON endpoints produced by `generate_resource`.
        api_base.join("route.rs"),
        api_base.join("[id]").join("route.rs"),
    ];
    confirm_and_remove(&mut paths)?;
    remove_empty_dir(&base.join("[id]").join("edit"));
    remove_empty_dir(&base.join("[id]"));
    remove_empty_dir(&base.join("new"));
    remove_empty_dir(&base);
    remove_empty_dir(&api_base.join("[id]"));
    remove_empty_dir(&api_base);
    Ok(())
}

fn destroy_usecase(root: &Path, name: &str) -> Result<()> {
    validate_identifier(name, "usecase")?;
    let snake = name.to_snake_case();
    let mut paths = vec![
        root.join("src")
            .join("usecases")
            .join(format!("{snake}.rs")),
        root.join("tests")
            .join("usecases")
            .join(format!("{snake}.rs")),
    ];
    confirm_and_remove(&mut paths)?;
    remove_pub_mod_if_present(
        &root.join("src").join("usecases").join("mod.rs"),
        "usecase-mods",
        &snake,
    )?;
    remove_wiring_bind_if_present(
        &root.join("src").join("wiring.rs"),
        &format!("usecase:{snake}"),
    )?;
    Ok(())
}

fn destroy_domain(root: &Path, name: &str) -> Result<()> {
    validate_identifier(name, "domain")?;
    let snake = name.to_snake_case();
    let domain_dir = root.join("src").join("domain").join(&snake);
    let mut paths = vec![
        domain_dir.join("mod.rs"),
        domain_dir.join("entity.rs"),
        domain_dir.join("value_objects.rs"),
        domain_dir.join("repository.rs"),
        domain_dir.join("error.rs"),
        root.join("src")
            .join("infrastructure")
            .join("persistence")
            .join(format!("{snake}_repository.rs")),
        root.join("src")
            .join("infrastructure")
            .join("mocks")
            .join(format!("{snake}_repository.rs")),
    ];
    confirm_and_remove(&mut paths)?;
    remove_empty_dir(&domain_dir);
    remove_pub_mod_if_present(
        &root.join("src").join("domain").join("mod.rs"),
        "domain-mods",
        &snake,
    )?;
    remove_pub_mod_if_present(
        &root
            .join("src")
            .join("infrastructure")
            .join("persistence")
            .join("mod.rs"),
        "persistence-mods",
        &format!("{snake}_repository"),
    )?;
    remove_pub_mod_if_present(
        &root
            .join("src")
            .join("infrastructure")
            .join("mocks")
            .join("mod.rs"),
        "mock-mods",
        &format!("{snake}_repository"),
    )?;
    remove_wiring_bind_if_present(
        &root.join("src").join("wiring.rs"),
        &format!("domain:{snake}"),
    )?;
    Ok(())
}

fn destroy_dto(root: &Path, name: &str) -> Result<()> {
    validate_identifier(name, "dto")?;
    let snake = name.to_snake_case();
    let mut paths = vec![root.join("src").join("dto").join(format!("{snake}.rs"))];
    confirm_and_remove(&mut paths)?;
    remove_pub_mod_if_present(
        &root.join("src").join("dto").join("mod.rs"),
        "dto-mods",
        &snake,
    )?;
    Ok(())
}

fn destroy_migration(root: &Path, name: &str) -> Result<()> {
    validate_identifier(name, "migration")?;
    let migrations_dir = root.join("migrations");
    if !migrations_dir.exists() {
        bail!(
            "migrations directory not found: {}",
            migrations_dir.display()
        );
    }
    let snake = name.to_snake_case();
    let mut matches: Vec<PathBuf> = Vec::new();
    let entries =
        std::fs::read_dir(&migrations_dir).context("failed to read migrations directory")?;
    for entry in entries {
        let entry = entry.context("failed to read migration entry")?;
        let file_name = entry.file_name();
        let Some(name_str) = file_name.to_str() else {
            continue;
        };
        if matches_migration_filename(name_str, &snake) {
            matches.push(entry.path());
        }
    }
    if matches.is_empty() {
        bail!("no migration matched name: {name}");
    }
    confirm_and_remove(&mut matches)?;
    Ok(())
}

/// Returns true when `file_name` is `<14 digits>_<snake>.{up,down}.sql` exactly.
/// Prevents `destroy migration users` from also removing
/// `add_users.up.sql` etc.
fn matches_migration_filename(file_name: &str, snake: &str) -> bool {
    if file_name.len() < 15 {
        return false;
    }
    let (ts, rest) = file_name.split_at(14);
    if !ts.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let Some(rest) = rest.strip_prefix('_') else {
        return false;
    };
    rest == format!("{snake}.up.sql") || rest == format!("{snake}.down.sql")
}

fn route_to_app_dir(root: &Path, route: &str) -> PathBuf {
    let mut dir = root.join("app");
    for segment in route.split('/').filter(|s| !s.is_empty()) {
        dir.push(segment);
    }
    dir
}

fn confirm_and_remove(paths: &mut Vec<PathBuf>) -> Result<()> {
    paths.retain(|p| p.exists());
    if paths.is_empty() {
        println!("nothing to remove");
        return Ok(());
    }
    for path in paths {
        if !confirm(&format!("remove {}?", path.display()))? {
            println!("skipped {}", path.display());
            continue;
        }
        std::fs::remove_file(&*path)
            .with_context(|| format!("failed to remove {}", path.display()))?;
        println!("removed {}", path.display());
    }
    Ok(())
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{prompt} [y/N] ");
    std::io::stdout()
        .flush()
        .context("failed to flush stdout")?;
    let stdin = std::io::stdin();
    let mut line = String::new();
    let mut handle = stdin.lock();
    if handle
        .read_line(&mut line)
        .context("failed to read stdin")?
        == 0
    {
        return Ok(false);
    }
    let trimmed = line.trim().to_ascii_lowercase();
    Ok(trimmed == "y" || trimmed == "yes")
}

/// Remove the `// <gluon:bind:{key}> ... // </gluon:bind:{key}>` block from
/// `wiring.rs`. Idempotent -- no-ops when either file or block is absent.
fn remove_wiring_bind_if_present(wiring_path: &Path, key: &str) -> Result<()> {
    if !wiring_path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(wiring_path)
        .with_context(|| format!("failed to read {}", wiring_path.display()))?;
    let open = format!("// <gluon:bind:{key}>");
    let close = format!("// </gluon:bind:{key}>");
    let Some(open_pos) = content.find(&open) else {
        return Ok(());
    };
    let line_start = content[..open_pos].rfind('\n').map_or(0, |i| i + 1);
    let Some(close_rel) = content[open_pos..].find(&close) else {
        return Ok(());
    };
    let mut end = open_pos + close_rel + close.len();
    if content[end..].starts_with('\n') {
        end += 1;
    }
    let mut new_content = String::with_capacity(content.len());
    new_content.push_str(&content[..line_start]);
    new_content.push_str(&content[end..]);
    std::fs::write(wiring_path, new_content)
        .with_context(|| format!("failed to write {}", wiring_path.display()))?;
    println!("update {} (bind {key} removed)", wiring_path.display());
    Ok(())
}

/// Remove `pub mod <name>;` from inside the `<gluon:<marker>>` /
/// `</gluon:<marker>>` block of `mod_rs_path`. Idempotent -- no-ops when the
/// file or marker is missing, or when the entry was never present.
fn remove_pub_mod_if_present(mod_rs_path: &Path, marker: &str, name: &str) -> Result<()> {
    if !mod_rs_path.exists() {
        return Ok(());
    }
    let original = std::fs::read_to_string(mod_rs_path)
        .with_context(|| format!("failed to read {}", mod_rs_path.display()))?;
    let open_marker = format!("// <gluon:{marker}>");
    let close_marker = format!("// </gluon:{marker}>");

    let Some(open_at) = original.find(&open_marker) else {
        return Ok(());
    };
    let block_start = open_at + open_marker.len();
    let Some(close_at_rel) = original[block_start..].find(&close_marker) else {
        return Ok(());
    };
    let close_at = block_start + close_at_rel;

    let target = format!("pub mod {name};");
    let inner = &original[block_start..close_at];
    let original_count = inner.lines().filter(|l| !l.trim().is_empty()).count();
    let mut lines: Vec<String> = inner
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty() && l.trim() != target)
        .map(String::from)
        .collect();
    if lines.len() == original_count {
        return Ok(());
    }
    lines.sort();
    let mut rebuilt = String::from("\n");
    for line in &lines {
        rebuilt.push_str(line);
        rebuilt.push('\n');
    }

    let mut output = String::with_capacity(original.len());
    output.push_str(&original[..block_start]);
    output.push_str(&rebuilt);
    output.push_str(&original[close_at..]);
    std::fs::write(mod_rs_path, output)
        .with_context(|| format!("failed to write {}", mod_rs_path.display()))?;
    println!("update {} ({marker}:{name} removed)", mod_rs_path.display());
    Ok(())
}

fn remove_empty_dir(dir: &Path) {
    if !dir.is_dir() {
        return;
    }
    if let Ok(mut entries) = std::fs::read_dir(dir)
        && entries.next().is_none()
    {
        let _ = std::fs::remove_dir(dir);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        matches_migration_filename, remove_pub_mod_if_present, remove_wiring_bind_if_present,
    };

    #[test]
    fn matches_full_filename() {
        assert!(matches_migration_filename(
            "20260620120000_create_users.up.sql",
            "create_users"
        ));
        assert!(matches_migration_filename(
            "20260620120000_create_users.down.sql",
            "create_users"
        ));
    }

    #[test]
    fn rejects_suffix_only_match() {
        // `users` should NOT match `add_users.up.sql` or `create_users.up.sql`.
        assert!(!matches_migration_filename(
            "20260620120000_create_users.up.sql",
            "users"
        ));
        assert!(!matches_migration_filename(
            "20260620120000_add_users.up.sql",
            "users"
        ));
    }

    #[test]
    fn rejects_missing_timestamp() {
        assert!(!matches_migration_filename(
            "create_users.up.sql",
            "create_users"
        ));
    }

    #[test]
    fn rejects_short_timestamp() {
        assert!(!matches_migration_filename(
            "2026_create_users.up.sql",
            "create_users"
        ));
    }

    // matches_migration_filename: additional edge cases.

    #[test]
    fn rejects_15_chars_without_underscore() {
        // 15 chars total: prefix 14 digits, then a non-`_` char.
        assert!(!matches_migration_filename(
            "20260620120000Xcreate_users.up.sql",
            "create_users"
        ));
        // Exactly 15 chars (no `_` at position 14).
        assert!(!matches_migration_filename("202606201200000", "0"));
    }

    #[test]
    fn accepts_all_digit_name() {
        // The `<snake>` portion may itself be digits -- that is allowed.
        assert!(matches_migration_filename(
            "20260620120000_123.up.sql",
            "123"
        ));
    }

    #[test]
    fn rejects_uppercase_extensions() {
        assert!(!matches_migration_filename(
            "20260620120000_create_users.UP.SQL",
            "create_users"
        ));
    }

    #[test]
    fn rejects_sql_without_up_or_down() {
        assert!(!matches_migration_filename(
            "20260620120000_create_users.sql",
            "create_users"
        ));
    }

    #[test]
    fn rejects_empty_filename() {
        assert!(!matches_migration_filename("", "create_users"));
    }

    // remove_wiring_bind_if_present: file/marker behavior.

    #[test]
    fn remove_wiring_bind_no_op_when_file_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wiring.rs");
        // File does not exist.
        remove_wiring_bind_if_present(&path, "usecase:foo").unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn remove_wiring_bind_no_op_when_key_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wiring.rs");
        let original = "// <gluon:binds>\n    // <gluon:bind:usecase:other>\n    builder.bind();\n    // </gluon:bind:usecase:other>\n// </gluon:binds>\n";
        std::fs::write(&path, original).unwrap();
        remove_wiring_bind_if_present(&path, "usecase:missing").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn remove_wiring_bind_single_key_removes_block_and_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wiring.rs");
        let original = "before\n// <gluon:bind:foo>\nline\n// </gluon:bind:foo>\nafter\n";
        std::fs::write(&path, original).unwrap();
        remove_wiring_bind_if_present(&path, "foo").unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after, "before\nafter\n");
    }

    #[test]
    fn remove_wiring_bind_one_of_many() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wiring.rs");
        let original = concat!(
            "// <gluon:bind:a>\n",
            "line_a\n",
            "// </gluon:bind:a>\n",
            "// <gluon:bind:b>\n",
            "line_b\n",
            "// </gluon:bind:b>\n",
        );
        std::fs::write(&path, original).unwrap();
        remove_wiring_bind_if_present(&path, "a").unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(!after.contains("<gluon:bind:a>"));
        assert!(after.contains("<gluon:bind:b>"));
        assert!(after.contains("line_b"));
    }

    #[test]
    fn remove_wiring_bind_no_op_when_close_marker_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wiring.rs");
        let original = "// <gluon:bind:foo>\nbody\n";
        std::fs::write(&path, original).unwrap();
        remove_wiring_bind_if_present(&path, "foo").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    // remove_pub_mod_if_present cases.

    #[test]
    fn remove_pub_mod_no_op_when_file_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mod.rs");
        remove_pub_mod_if_present(&path, "domain-mods", "user").unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn remove_pub_mod_no_op_when_marker_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mod.rs");
        let original = "pub mod user;\n";
        std::fs::write(&path, original).unwrap();
        remove_pub_mod_if_present(&path, "domain-mods", "user").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn remove_pub_mod_single_entry_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mod.rs");
        let original = "// <gluon:domain-mods>\npub mod user;\n// </gluon:domain-mods>\n";
        std::fs::write(&path, original).unwrap();
        remove_pub_mod_if_present(&path, "domain-mods", "user").unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(!after.contains("pub mod user;"));
        assert!(after.contains("// <gluon:domain-mods>"));
        assert!(after.contains("// </gluon:domain-mods>"));
    }

    #[test]
    fn remove_pub_mod_no_op_when_target_absent_inside_block() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mod.rs");
        let original = "// <gluon:domain-mods>\npub mod other;\n// </gluon:domain-mods>\n";
        std::fs::write(&path, original).unwrap();
        remove_pub_mod_if_present(&path, "domain-mods", "user").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn remove_pub_mod_keeps_remaining_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mod.rs");
        let original = concat!(
            "// <gluon:domain-mods>\n",
            "pub mod alpha;\n",
            "pub mod beta;\n",
            "pub mod gamma;\n",
            "// </gluon:domain-mods>\n",
        );
        std::fs::write(&path, original).unwrap();
        remove_pub_mod_if_present(&path, "domain-mods", "beta").unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.contains("pub mod alpha;"));
        assert!(!after.contains("pub mod beta;"));
        assert!(after.contains("pub mod gamma;"));
        let alpha_pos = after.find("pub mod alpha;").unwrap();
        let gamma_pos = after.find("pub mod gamma;").unwrap();
        assert!(alpha_pos < gamma_pos, "expected sorted order: {after}");
    }
}
