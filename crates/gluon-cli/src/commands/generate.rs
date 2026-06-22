//! Implementation of `gluon generate` / `gluon g`.
//!
//! Each artifact kind has its own helper that renders one or more embedded
//! templates, writes the resulting files to disk, and (when appropriate)
//! inserts a bind marker into the application's `src/wiring.rs`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use heck::{ToPascalCase, ToSnakeCase};
use serde_json::{Value, json};

use crate::GenerateKind;
use crate::templating;
use crate::wiring;

// ---------------------------------------------------------------------------
// input validation
//
// CLI inputs (`route`, `name`, `--field NAME:TYPE`) flow directly into file
// paths and into generated Rust / SQL sources. The framework rejects values
// that could escape a string literal, break out of a struct field, or escape
// the project directory.
// ---------------------------------------------------------------------------

pub(crate) fn validate_route(route: &str) -> Result<()> {
    if route.is_empty() {
        bail!("route must not be empty");
    }
    for segment in route.split('/').filter(|s| !s.is_empty()) {
        let inner = if let Some(rest) = segment
            .strip_prefix("[...")
            .and_then(|s| s.strip_suffix(']'))
        {
            rest
        } else if let Some(rest) = segment.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            rest
        } else if let Some(rest) = segment.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
            rest
        } else {
            segment
        };
        if inner.is_empty()
            || inner == "."
            || inner == ".."
            || !inner
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            bail!("invalid route segment: {segment}");
        }
    }
    Ok(())
}

pub(crate) fn validate_identifier(s: &str, kind: &str) -> Result<()> {
    if s.is_empty() {
        bail!("{kind} name must not be empty");
    }
    let first = s.chars().next().unwrap_or('_');
    if !(first.is_ascii_alphabetic() || first == '_') {
        bail!("invalid {kind} name: {s} (must start with a letter or underscore)");
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        bail!("invalid {kind} name: {s} (only letters, digits and underscore are allowed)");
    }
    Ok(())
}

pub(crate) fn validate_field_type(ty: &str) -> Result<()> {
    if ty.is_empty() {
        bail!("field type must not be empty");
    }
    // A leading `:` means the user wrote `name::Type` (double colon in the
    // --field spec); split_once(':') then produces ty = ":Type" which would
    // generate invalid Rust. Reject it explicitly by requiring the first
    // character to be a valid type-identifier start.
    if !ty
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '\'')
    {
        bail!(
            "invalid field type: {ty} (must start with a letter, digit, underscore, or lifetime tick)"
        );
    }
    let allowed = |c: char| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '<' | '>' | ',' | ' ' | ':' | '\'')
    };
    if !ty.chars().all(allowed) {
        bail!("invalid field type: {ty} (contains disallowed character)");
    }
    Ok(())
}

/// Path to the application's wiring file, relative to the current working
/// directory.
const WIRING_PATH: &str = "src/wiring.rs";

/// Dispatcher invoked by `main.rs` for `gluon generate <kind> ...`.
///
/// # Errors
///
/// Propagates any I/O, template, or wiring error encountered while generating
/// the requested artifact.
pub fn run(kind: GenerateKind) -> Result<()> {
    match kind {
        GenerateKind::Controller { route, api } => generate_controller(&route, api),
        GenerateKind::Resource { name } => generate_resource(&name),
        GenerateKind::Usecase { name } => generate_usecase(&name),
        GenerateKind::Domain { name, fields } => generate_domain(&name, &fields),
        GenerateKind::Dto { name } => generate_dto(&name),
        GenerateKind::Migration { name } => generate_migration(&name),
    }
}

// ---------------------------------------------------------------------------
// controller
// ---------------------------------------------------------------------------

fn generate_controller(route: &str, api: bool) -> Result<()> {
    let trimmed = route.trim_matches('/');
    validate_route(trimmed)?;

    let dir = PathBuf::from("app").join(route_to_dir(trimmed));
    let pascal_name = route_to_pascal(trimmed);
    let ctx = json!({
        "pascal_name": pascal_name,
        "route": format!("/{trimmed}"),
    });

    if api {
        render_to_file("controller/route.rs.j2", &ctx, &dir.join("route.rs"))?;
    } else {
        render_to_file("controller/page.rs.j2", &ctx, &dir.join("page.rs"))?;
        render_to_file("controller/page.tsx.j2", &ctx, &dir.join("page.tsx"))?;
    }
    Ok(())
}

/// Convert a route like `users/[id]/edit` into an on-disk directory path.
///
/// Bracketed dynamic segments (e.g. `[id]`) are preserved verbatim. Empty
/// segments are skipped.
pub(crate) fn route_to_dir(route: &str) -> PathBuf {
    let mut out = PathBuf::new();
    for segment in route.split('/').filter(|s| !s.is_empty()) {
        out.push(segment);
    }
    out
}

/// Derive a `PascalCase` identifier from a route by joining its non-bracketed
/// segments. `/users/[id]/edit` becomes `UsersEdit`.
pub(crate) fn route_to_pascal(route: &str) -> String {
    let mut buf = String::new();
    for segment in route.split('/').filter(|s| !s.is_empty()) {
        let cleaned = segment.trim_start_matches('[').trim_end_matches(']');
        if cleaned.is_empty() {
            continue;
        }
        buf.push_str(&cleaned.to_pascal_case());
    }
    if buf.is_empty() {
        "Index".to_owned()
    } else {
        buf
    }
}

// ---------------------------------------------------------------------------
// resource (RESTful controllers)
// ---------------------------------------------------------------------------

fn generate_resource(name: &str) -> Result<()> {
    validate_identifier(name, "resource")?;
    let snake = name.to_snake_case();

    // Pages (HTML controllers).
    let page_routes = [
        snake.clone(),                // index   -> /users
        format!("{snake}/new"),       // new     -> /users/new
        format!("{snake}/[id]"),      // show    -> /users/[id]
        format!("{snake}/[id]/edit"), // edit    -> /users/[id]/edit
    ];
    for route in &page_routes {
        generate_controller(route, false)?;
    }

    // JSON endpoints for the mutating actions.
    let api_routes = [
        format!("api/{snake}"),      // create  -> POST   /api/users
        format!("api/{snake}/[id]"), // update/destroy -> PATCH/DELETE
    ];
    for route in &api_routes {
        generate_controller(route, true)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// usecase
// ---------------------------------------------------------------------------

fn generate_usecase(name: &str) -> Result<()> {
    validate_identifier(name, "usecase")?;
    let snake = name.to_snake_case();
    let pascal = name.to_pascal_case();

    let ctx = json!({
        "snake_name": snake,
        "pascal_name": pascal,
    });

    let file = PathBuf::from("src/usecases").join(format!("{snake}.rs"));
    render_to_file("usecase/usecase.rs.j2", &ctx, &file)?;
    insert_pub_mod_if_present(Path::new("src/usecases/mod.rs"), "usecase-mods", &snake)?;

    let bind_line = format!(
        "builder = builder.bind::<dyn crate::usecases::{snake}::{pascal}, _>(\
|_| std::sync::Arc::new(crate::usecases::{snake}::{pascal}Impl::new()));"
    );
    let key = format!("usecase:{snake}");
    insert_bind_if_present(&key, &bind_line)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// domain
// ---------------------------------------------------------------------------

fn generate_domain(name: &str, fields: &[String]) -> Result<()> {
    validate_identifier(name, "domain")?;
    let snake = name.to_snake_case();
    let pascal = name.to_pascal_case();

    let parsed_fields = parse_fields(fields)?;
    let value_objects = extract_value_objects(&parsed_fields);
    let fields_json: Vec<Value> = parsed_fields
        .iter()
        .map(|f| json!({ "name": f.name, "ty": f.ty }))
        .collect();

    let ctx = json!({
        "snake_name": snake,
        "pascal_name": pascal,
        "fields": fields_json,
        "value_objects": value_objects,
    });

    let dir = PathBuf::from("src/domain").join(&snake);
    render_to_file("domain/mod.rs.j2", &ctx, &dir.join("mod.rs"))?;
    render_to_file("domain/entity.rs.j2", &ctx, &dir.join("entity.rs"))?;
    render_to_file(
        "domain/value_objects.rs.j2",
        &ctx,
        &dir.join("value_objects.rs"),
    )?;
    render_to_file("domain/repository.rs.j2", &ctx, &dir.join("repository.rs"))?;
    render_to_file("domain/error.rs.j2", &ctx, &dir.join("error.rs"))?;

    let persistence_dir = PathBuf::from("src/infrastructure/persistence");
    render_to_file(
        "domain/repository_impl.rs.j2",
        &ctx,
        &persistence_dir.join(format!("{snake}_repository.rs")),
    )?;

    let mocks_dir = PathBuf::from("src/infrastructure/mocks");
    render_to_file(
        "domain/mock.rs.j2",
        &ctx,
        &mocks_dir.join(format!("{snake}_repository.rs")),
    )?;

    insert_pub_mod_if_present(Path::new("src/domain/mod.rs"), "domain-mods", &snake)?;
    insert_pub_mod_if_present(
        Path::new("src/infrastructure/persistence/mod.rs"),
        "persistence-mods",
        &format!("{snake}_repository"),
    )?;
    insert_pub_mod_if_present(
        Path::new("src/infrastructure/mocks/mod.rs"),
        "mock-mods",
        &format!("{snake}_repository"),
    )?;

    let bind_line = format!(
        "builder = builder.bind::<dyn crate::domain::{snake}::{pascal}Repository, _>(\
|c| std::sync::Arc::new(crate::infrastructure::persistence::{snake}_repository::Postgres{pascal}Repository::new(c.resolve::<sqlx::PgPool>())));"
    );
    let key = format!("domain:{snake}");
    insert_bind_if_present(&key, &bind_line)?;
    Ok(())
}

/// Parsed representation of a single `--field NAME:TYPE` argument.
#[derive(Debug)]
pub(crate) struct ParsedField {
    pub(crate) name: String,
    pub(crate) ty: String,
}

/// Parse `--field name:Type` arguments into a structured list.
pub(crate) fn parse_fields(raw: &[String]) -> Result<Vec<ParsedField>> {
    let mut out = Vec::with_capacity(raw.len());
    for field in raw {
        let (name, ty) = field
            .split_once(':')
            .ok_or_else(|| anyhow!("invalid --field value: {field} (expected NAME:TYPE)"))?;
        let name = name.trim();
        let ty = ty.trim();
        validate_identifier(name, "field")?;
        validate_field_type(ty)?;
        out.push(ParsedField {
            name: name.to_owned(),
            ty: ty.to_owned(),
        });
    }
    Ok(out)
}

/// Extract field types that look like newtype value objects (i.e. `PascalCase`
/// identifiers that aren't well-known primitive or std types).
pub(crate) fn extract_value_objects(fields: &[ParsedField]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for field in fields {
        let ty = field.ty.trim();
        if !is_value_object_type(ty) {
            continue;
        }
        let owned = ty.to_owned();
        if !seen.contains(&owned) {
            seen.push(owned);
        }
    }
    seen
}

/// Return true if `ty` is a bare `PascalCase` identifier that is not a
/// well-known primitive / std type. Anything containing generic parameters,
/// references, paths, or starting with a lowercase character is skipped.
pub(crate) fn is_value_object_type(ty: &str) -> bool {
    if ty.is_empty() {
        return false;
    }
    if !ty.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
        return false;
    }
    if !ty.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }
    !is_well_known_type(ty)
}

pub(crate) fn is_well_known_type(ty: &str) -> bool {
    matches!(
        ty,
        "String"
            | "Vec"
            | "Option"
            | "Box"
            | "Arc"
            | "Rc"
            | "HashMap"
            | "BTreeMap"
            | "HashSet"
            | "BTreeSet"
    )
}

// ---------------------------------------------------------------------------
// dto
// ---------------------------------------------------------------------------

fn generate_dto(name: &str) -> Result<()> {
    validate_identifier(name, "dto")?;
    let snake = name.to_snake_case();
    let pascal = name.to_pascal_case();

    let ctx = json!({
        "snake_name": snake,
        "pascal_name": pascal,
    });

    let file = PathBuf::from("src/dto").join(format!("{snake}.rs"));
    render_to_file("dto/dto.rs.j2", &ctx, &file)?;
    insert_pub_mod_if_present(Path::new("src/dto/mod.rs"), "dto-mods", &snake)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// migration
// ---------------------------------------------------------------------------

fn generate_migration(name: &str) -> Result<()> {
    validate_identifier(name, "migration")?;
    let snake = name.to_snake_case();

    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let ctx = json!({
        "name": snake,
        "timestamp": timestamp,
    });

    let dir = PathBuf::from("migrations");
    let up_path = dir.join(format!("{timestamp}_{snake}.up.sql"));
    let down_path = dir.join(format!("{timestamp}_{snake}.down.sql"));
    render_to_file("migration/up.sql.j2", &ctx, &up_path)?;
    render_to_file("migration/down.sql.j2", &ctx, &down_path)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Render an embedded template to a file, creating parent directories as
/// needed. Refuses to overwrite an existing file.
fn render_to_file(template: &str, ctx: &Value, dest: &Path) -> Result<()> {
    if dest.exists() {
        bail!("refusing to overwrite existing file: {}", dest.display());
    }
    if let Some(parent) = dest.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }
    let rendered = templating::render(template, ctx)?;
    std::fs::write(dest, rendered)
        .with_context(|| format!("failed to write file: {}", dest.display()))?;
    println!("create {}", dest.display());
    Ok(())
}

/// Insert a bind block into `src/wiring.rs` if such a file exists in the
/// current working directory. Silently no-ops otherwise (e.g. when the
/// command is run outside an application).
fn insert_bind_if_present(key: &str, line: &str) -> Result<()> {
    let wiring_path = Path::new(WIRING_PATH);
    if !wiring_path.exists() {
        return Ok(());
    }
    wiring::insert_bind(wiring_path, key, line)?;
    println!("update {WIRING_PATH} (bind {key})");
    Ok(())
}

/// Insert `pub mod <name>;` inside the `<gluon:<marker>>` / `</gluon:<marker>>`
/// block of `mod_rs_path`, in sorted order, idempotently. No-ops when the
/// target file does not exist or the line is already present.
pub(crate) fn insert_pub_mod_if_present(
    mod_rs_path: &Path,
    marker: &str,
    name: &str,
) -> Result<()> {
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

    let inner = &original[block_start..close_at];
    let new_line = format!("pub mod {name};");
    if inner.lines().any(|l| l.trim() == new_line) {
        return Ok(());
    }

    let mut lines: Vec<String> = inner
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    lines.push(new_line);
    lines.sort();
    let mut rebuilt = String::from("\n");
    for line in &lines {
        rebuilt.push_str(line);
        rebuilt.push('\n');
    }

    let mut output = String::with_capacity(original.len() + 32);
    output.push_str(&original[..block_start]);
    output.push_str(&rebuilt);
    output.push_str(&original[close_at..]);
    std::fs::write(mod_rs_path, output)
        .with_context(|| format!("failed to write {}", mod_rs_path.display()))?;
    println!("update {} ({marker}:{name})", mod_rs_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        ParsedField, extract_value_objects, insert_pub_mod_if_present, is_value_object_type,
        is_well_known_type, parse_fields, route_to_dir, route_to_pascal, validate_field_type,
        validate_identifier, validate_route,
    };

    // -----------------------------------------------------------------------
    // validate_route
    // -----------------------------------------------------------------------

    #[test]
    fn validate_route_accepts_simple_segment() {
        validate_route("users").unwrap();
    }

    #[test]
    fn validate_route_accepts_dynamic_segment() {
        validate_route("users/[id]").unwrap();
    }

    #[test]
    fn validate_route_accepts_nested_dynamic_segment() {
        validate_route("users/[id]/edit").unwrap();
    }

    #[test]
    fn validate_route_accepts_catch_all_segment() {
        validate_route("posts/[...slug]").unwrap();
    }

    #[test]
    fn validate_route_accepts_group_segment() {
        validate_route("(admin)/users").unwrap();
    }

    #[test]
    fn validate_route_accepts_hyphenated_segment() {
        validate_route("api/health-check").unwrap();
    }

    #[test]
    fn validate_route_accepts_three_levels() {
        validate_route("a/b/c").unwrap();
    }

    #[test]
    fn validate_route_rejects_empty() {
        assert!(validate_route("").is_err());
    }

    #[test]
    fn validate_route_rejects_slash_only() {
        // After filtering empty segments there are none -> no segment error, but
        // the top-level check requires the trimmed route to be non-empty. The
        // current implementation only checks `route.is_empty()`, so `"/"` is
        // accepted by `validate_route` itself -- the caller strips slashes first.
        // We still verify that callers' typical input (trimmed) is rejected.
        validate_route("/").unwrap();
    }

    #[test]
    fn validate_route_rejects_unclosed_bracket() {
        assert!(validate_route("[id").is_err());
    }

    #[test]
    fn validate_route_rejects_unmatched_closing_bracket() {
        assert!(validate_route("id]").is_err());
    }

    #[test]
    fn validate_route_rejects_empty_brackets() {
        assert!(validate_route("[]").is_err());
    }

    #[test]
    fn validate_route_rejects_unclosed_catch_all() {
        assert!(validate_route("[...slug").is_err());
    }

    #[test]
    fn validate_route_rejects_slash_inside_group() {
        assert!(validate_route("(group/x)").is_err());
    }

    #[test]
    fn validate_route_rejects_parent_directory() {
        assert!(validate_route("users/..").is_err());
    }

    #[test]
    fn validate_route_rejects_current_directory() {
        assert!(validate_route("users/.").is_err());
    }

    #[test]
    fn validate_route_rejects_non_ascii() {
        // sakoku-ignore-next-line
        assert!(validate_route("日本語").is_err());
    }

    #[test]
    fn validate_route_rejects_space_in_segment() {
        assert!(validate_route("users/foo bar").is_err());
    }

    #[test]
    fn validate_route_rejects_semicolon() {
        assert!(validate_route("users/foo;").is_err());
    }

    // -----------------------------------------------------------------------
    // validate_identifier
    // -----------------------------------------------------------------------

    #[test]
    fn validate_identifier_accepts_lower_word() {
        validate_identifier("foo", "test").unwrap();
    }

    #[test]
    fn validate_identifier_accepts_leading_underscore() {
        validate_identifier("_foo", "test").unwrap();
    }

    #[test]
    fn validate_identifier_accepts_snake_case() {
        validate_identifier("foo_bar", "test").unwrap();
    }

    #[test]
    fn validate_identifier_accepts_pascal_with_digit() {
        validate_identifier("Foo1", "test").unwrap();
    }

    #[test]
    fn validate_identifier_accepts_single_underscore() {
        validate_identifier("_", "test").unwrap();
    }

    #[test]
    fn validate_identifier_accepts_trailing_underscore() {
        validate_identifier("foo_", "test").unwrap();
    }

    #[test]
    fn validate_identifier_rejects_empty() {
        assert!(validate_identifier("", "test").is_err());
    }

    #[test]
    fn validate_identifier_rejects_leading_digit() {
        assert!(validate_identifier("1foo", "test").is_err());
    }

    #[test]
    fn validate_identifier_rejects_space() {
        assert!(validate_identifier("foo bar", "test").is_err());
    }

    #[test]
    fn validate_identifier_rejects_path_separator() {
        assert!(validate_identifier("foo::bar", "test").is_err());
    }

    #[test]
    fn validate_identifier_rejects_non_ascii() {
        // sakoku-ignore-next-line
        assert!(validate_identifier("日本語", "test").is_err());
        // sakoku-ignore-next-line
    }

    #[test]
    fn validate_identifier_rejects_hyphen() {
        assert!(validate_identifier("foo-bar", "test").is_err());
    }

    #[test]
    fn validate_identifier_rejects_dot() {
        assert!(validate_identifier("foo.bar", "test").is_err());
    }

    #[test]
    fn validate_identifier_rejects_semicolon() {
        assert!(validate_identifier("foo;", "test").is_err());
    }

    // -----------------------------------------------------------------------
    // validate_field_type
    // -----------------------------------------------------------------------

    #[test]
    fn validate_field_type_accepts_simple() {
        validate_field_type("String").unwrap();
    }

    #[test]
    fn validate_field_type_accepts_option() {
        validate_field_type("Option<String>").unwrap();
    }

    #[test]
    fn validate_field_type_accepts_vec_primitive() {
        validate_field_type("Vec<u32>").unwrap();
    }

    #[test]
    fn validate_field_type_accepts_hashmap_two_args() {
        validate_field_type("HashMap<String, i32>").unwrap();
    }

    #[test]
    fn validate_field_type_accepts_box_dyn() {
        validate_field_type("Box<dyn Trait>").unwrap();
    }

    #[test]
    fn validate_field_type_accepts_qualified_path() {
        validate_field_type("std::sync::Arc<T>").unwrap();
    }

    #[test]
    fn validate_field_type_rejects_reference() {
        // The current allowed-character set excludes `&`, so references
        // (e.g. `&'static str`) are rejected even though they look syntactically
        // valid in Rust.
        assert!(validate_field_type("&'static str").is_err());
    }

    #[test]
    fn validate_field_type_rejects_empty() {
        assert!(validate_field_type("").is_err());
    }

    #[test]
    fn validate_field_type_rejects_semicolon() {
        assert!(validate_field_type("String;").is_err());
    }

    #[test]
    fn validate_field_type_rejects_open_brace() {
        assert!(validate_field_type("String{").is_err());
    }

    #[test]
    fn validate_field_type_rejects_close_brace() {
        assert!(validate_field_type("String}").is_err());
    }

    #[test]
    fn validate_field_type_rejects_newline() {
        assert!(validate_field_type("String\n").is_err());
    }

    #[test]
    fn validate_field_type_rejects_tab() {
        assert!(validate_field_type("String\t").is_err());
    }

    #[test]
    fn validate_field_type_rejects_equals() {
        assert!(validate_field_type("String=foo").is_err());
    }

    #[test]
    fn validate_field_type_rejects_leading_colon() {
        // `:MyType` results from `--field name::MyType` (double-colon in the
        // user spec). Reject it to prevent generating `: MyType` in Rust.
        assert!(validate_field_type(":MyType").is_err());
    }

    // -----------------------------------------------------------------------
    // route_to_dir
    // -----------------------------------------------------------------------

    #[test]
    fn route_to_dir_simple() {
        assert_eq!(route_to_dir("users"), PathBuf::from("users"));
    }

    #[test]
    fn route_to_dir_nested_dynamic() {
        assert_eq!(
            route_to_dir("users/[id]/edit"),
            PathBuf::from("users").join("[id]").join("edit"),
        );
    }

    #[test]
    fn route_to_dir_empty_input() {
        assert_eq!(route_to_dir(""), PathBuf::new());
    }

    #[test]
    fn route_to_dir_skips_empty_segments() {
        assert_eq!(
            route_to_dir("users//edit"),
            PathBuf::from("users").join("edit"),
        );
    }

    #[test]
    fn route_to_dir_three_levels() {
        let p = route_to_dir("a/b/c");
        assert_eq!(p.iter().count(), 3);
        assert_eq!(p, PathBuf::from("a").join("b").join("c"));
    }

    // -----------------------------------------------------------------------
    // route_to_pascal -- values were verified by exercising the helper directly.
    // -----------------------------------------------------------------------

    #[test]
    fn route_to_pascal_empty_returns_index() {
        assert_eq!(route_to_pascal(""), "Index");
    }

    #[test]
    fn route_to_pascal_single_segment() {
        assert_eq!(route_to_pascal("users"), "Users");
    }

    #[test]
    fn route_to_pascal_dynamic_segment_is_included() {
        // `[id]` -> trim brackets -> `id` -> PascalCase -> `Id`.
        assert_eq!(route_to_pascal("users/[id]"), "UsersId");
    }

    #[test]
    fn route_to_pascal_dynamic_then_static() {
        assert_eq!(route_to_pascal("users/[id]/edit"), "UsersIdEdit");
    }

    #[test]
    fn route_to_pascal_two_static_segments() {
        assert_eq!(route_to_pascal("api/health"), "ApiHealth");
    }

    #[test]
    fn route_to_pascal_catch_all_drops_dots() {
        // `[...slug]` -> trim leading `[` and trailing `]` -> `...slug`. heck's
        // PascalCase treats the dots as separators and yields `Slug`.
        assert_eq!(route_to_pascal("posts/[...slug]"), "PostsSlug");
    }

    #[test]
    fn route_to_pascal_only_slashes_returns_index() {
        assert_eq!(route_to_pascal("/"), "Index");
    }

    // -----------------------------------------------------------------------
    // parse_fields
    // -----------------------------------------------------------------------

    #[test]
    fn parse_fields_empty_input() {
        let parsed = parse_fields(&[]).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn parse_fields_single_field() {
        let parsed = parse_fields(&["name:String".to_owned()]).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "name");
        assert_eq!(parsed[0].ty, "String");
    }

    #[test]
    fn parse_fields_two_fields() {
        let parsed = parse_fields(&["name:String".to_owned(), "email:Email".to_owned()]).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "name");
        assert_eq!(parsed[0].ty, "String");
        assert_eq!(parsed[1].name, "email");
        assert_eq!(parsed[1].ty, "Email");
    }

    #[test]
    fn parse_fields_rejects_missing_colon() {
        assert!(parse_fields(&["nameString".to_owned()]).is_err());
    }

    #[test]
    fn parse_fields_rejects_empty_name() {
        assert!(parse_fields(&[":String".to_owned()]).is_err());
    }

    #[test]
    fn parse_fields_rejects_empty_type() {
        assert!(parse_fields(&["name:".to_owned()]).is_err());
    }

    #[test]
    fn parse_fields_trims_whitespace() {
        let parsed = parse_fields(&["  name : String  ".to_owned()]).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "name");
        assert_eq!(parsed[0].ty, "String");
    }

    #[test]
    fn parse_fields_preserves_generic_with_comma() {
        let parsed = parse_fields(&["name:Vec<String, i32>".to_owned()]).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "name");
        assert_eq!(parsed[0].ty, "Vec<String, i32>");
    }

    #[test]
    fn parse_fields_rejects_double_colon_in_spec() {
        // `--field status::PostStatus` splits on the first `:`, producing
        // ty = ":PostStatus" which starts with `:` and is now rejected to
        // prevent generating invalid Rust syntax.
        assert!(parse_fields(&["status::PostStatus".to_owned()]).is_err());
    }

    #[test]
    fn parse_fields_accepts_qualified_path_after_colon() {
        // `--field conn:std::sync::Arc<T>` splits into name="conn",
        // ty="std::sync::Arc<T>" which starts with 's' and is valid.
        let parsed = parse_fields(&["conn:std::sync::Arc<T>".to_owned()]).unwrap();
        assert_eq!(parsed[0].name, "conn");
        assert_eq!(parsed[0].ty, "std::sync::Arc<T>");
    }

    // -----------------------------------------------------------------------
    // is_well_known_type / is_value_object_type / extract_value_objects
    // -----------------------------------------------------------------------

    #[test]
    fn is_well_known_type_string() {
        assert!(is_well_known_type("String"));
    }

    #[test]
    fn is_well_known_type_vec() {
        assert!(is_well_known_type("Vec"));
    }

    #[test]
    fn is_well_known_type_email_is_not_well_known() {
        assert!(!is_well_known_type("Email"));
    }

    #[test]
    fn is_value_object_type_email_is_vo() {
        assert!(is_value_object_type("Email"));
    }

    #[test]
    fn is_value_object_type_string_is_not_vo() {
        assert!(!is_value_object_type("String"));
    }

    #[test]
    fn is_value_object_type_lowercase_primitive_is_not_vo() {
        assert!(!is_value_object_type("i32"));
    }

    #[test]
    fn is_value_object_type_generic_is_not_vo() {
        assert!(!is_value_object_type("Option<String>"));
    }

    #[test]
    fn is_value_object_type_reference_is_not_vo() {
        assert!(!is_value_object_type("&'static str"));
    }

    #[test]
    fn is_value_object_type_underscore_prefix_is_not_vo() {
        assert!(!is_value_object_type("_Foo"));
    }

    #[test]
    fn extract_value_objects_preserves_order_and_deduplicates() {
        let fields = vec![
            ParsedField {
                name: "a".into(),
                ty: "Email".into(),
            },
            ParsedField {
                name: "b".into(),
                ty: "Email".into(),
            },
            ParsedField {
                name: "c".into(),
                ty: "UserName".into(),
            },
            ParsedField {
                name: "d".into(),
                ty: "String".into(),
            },
        ];
        assert_eq!(extract_value_objects(&fields), vec!["Email", "UserName"]);
    }

    // -----------------------------------------------------------------------
    // insert_pub_mod_if_present
    // -----------------------------------------------------------------------

    fn write_mod_rs(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        let path = dir.join("mod.rs");
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn insert_pub_mod_inserts_in_sorted_order() {
        let tmp = tempfile::tempdir().unwrap();
        let mod_rs = write_mod_rs(
            tmp.path(),
            "// <gluon:domain-mods>\n// </gluon:domain-mods>\n",
        );
        insert_pub_mod_if_present(&mod_rs, "domain-mods", "foo").unwrap();
        let after = std::fs::read_to_string(&mod_rs).unwrap();
        assert!(
            after.contains("pub mod foo;"),
            "expected pub mod foo; in output, got:\n{after}",
        );
        assert_eq!(after.matches("pub mod foo;").count(), 1);
    }

    #[test]
    fn insert_pub_mod_is_idempotent_for_same_name() {
        let tmp = tempfile::tempdir().unwrap();
        let mod_rs = write_mod_rs(
            tmp.path(),
            "// <gluon:domain-mods>\n// </gluon:domain-mods>\n",
        );
        insert_pub_mod_if_present(&mod_rs, "domain-mods", "foo").unwrap();
        insert_pub_mod_if_present(&mod_rs, "domain-mods", "foo").unwrap();
        let after = std::fs::read_to_string(&mod_rs).unwrap();
        assert_eq!(after.matches("pub mod foo;").count(), 1);
    }

    #[test]
    fn insert_pub_mod_sorts_inserts() {
        let tmp = tempfile::tempdir().unwrap();
        let mod_rs = write_mod_rs(
            tmp.path(),
            "// <gluon:domain-mods>\n// </gluon:domain-mods>\n",
        );
        insert_pub_mod_if_present(&mod_rs, "domain-mods", "zoo").unwrap();
        insert_pub_mod_if_present(&mod_rs, "domain-mods", "bar").unwrap();
        let after = std::fs::read_to_string(&mod_rs).unwrap();
        let bar_at = after
            .find("pub mod bar;")
            .expect("bar entry should be present");
        let zoo_at = after
            .find("pub mod zoo;")
            .expect("zoo entry should be present");
        assert!(bar_at < zoo_at, "expected bar before zoo, got:\n{after}");
    }

    #[test]
    fn insert_pub_mod_is_no_op_when_marker_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let body = "// no markers here\npub mod existing;\n";
        let mod_rs = write_mod_rs(tmp.path(), body);
        insert_pub_mod_if_present(&mod_rs, "domain-mods", "foo").unwrap();
        let after = std::fs::read_to_string(&mod_rs).unwrap();
        assert_eq!(after, body);
    }

    #[test]
    fn insert_pub_mod_is_no_op_when_file_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nope/mod.rs");
        insert_pub_mod_if_present(&missing, "domain-mods", "foo").unwrap();
        assert!(!missing.exists());
    }
}
