#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::missing_panics_doc,
        clippy::allow_attributes
    )
)]
//! build.rs helpers for the gluon framework.
//!
//! Scans the `app/` directory of the consuming crate, discovers `page.rs` and
//! `route.rs` files, infers their URL paths from the directory layout, and
//! generates a Rust source file (`$OUT_DIR/__gluon_app.rs`) that exposes a
//! `__gluon_router()` function returning an `axum::Router`.

use std::fmt;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use syn::{Item, ItemFn};
use walkdir::WalkDir;

/// Errors that can occur while running the build-time scanner / code generator.
#[derive(Debug)]
pub enum BuildError {
    /// I/O failure while reading the `app/` tree or writing the generated file.
    Io(std::io::Error),
    /// `syn` failed to parse a discovered Rust source file.
    Syn(syn::Error),
    /// Other build-time failures (environment variables, malformed paths, ...).
    Other(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "gluon-build io error: {e}"),
            Self::Syn(e) => write!(f, "gluon-build parse error: {e}"),
            Self::Other(msg) => write!(f, "gluon-build error: {msg}"),
        }
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Syn(e) => Some(e),
            Self::Other(_) => None,
        }
    }
}

impl From<std::io::Error> for BuildError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<syn::Error> for BuildError {
    fn from(e: syn::Error) -> Self {
        Self::Syn(e)
    }
}

/// HTTP methods that gluon recognises as page/route handlers, in declaration
/// order. Exposed so the CLI's `gluon routes` command can share the list.
pub const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete"];

/// One discovered entry-point file (`page.rs` or `route.rs`).
#[derive(Debug, Clone)]
pub struct Entry {
    /// Absolute path to the `.rs` file.
    pub abs_path: PathBuf,
    /// Path segments from `app/` to the file's parent directory, in order.
    /// e.g. `app/users/[id]/page.rs` -> `["users", "[id]"]`.
    /// The file stem (`page` / `route`) is NOT included here.
    pub dir_segments: Vec<String>,
    /// File stem: either `page` or `route`.
    pub file_stem: String,
    /// HTTP methods (`get`, `post`, ...) the file defines as `pub async fn`.
    pub methods: Vec<String>,
}

impl Entry {
    /// Returns the URL path this entry serves (`/users/:id`, `/`, ...).
    #[must_use]
    pub fn url_path(&self) -> String {
        url_path_for(&self.dir_segments)
    }
}

/// Scans `app_dir`, parses each `page.rs` / `route.rs` file, and returns the
/// discovered route entries sorted by file path. Returns an empty vector when
/// `app_dir` does not exist.
///
/// # Errors
///
/// Returns [`BuildError`] when the directory cannot be walked or a discovered
/// file fails to parse.
pub fn scan(app_dir: &Path) -> Result<Vec<Entry>, BuildError> {
    if !app_dir.is_dir() {
        return Ok(Vec::new());
    }
    scan_app_dir(app_dir)
}

/// Entry point. Call this from `build.rs`.
///
/// # Errors
///
/// Returns [`BuildError`] when the `app/` tree cannot be read, a discovered
/// `.rs` file fails to parse, or the generated file cannot be written.
pub fn run() -> Result<(), BuildError> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|e| BuildError::Other(format!("CARGO_MANIFEST_DIR not set: {e}")))?;
    let out_dir =
        std::env::var("OUT_DIR").map_err(|e| BuildError::Other(format!("OUT_DIR not set: {e}")))?;

    let app_dir = PathBuf::from(&manifest_dir).join("app");
    let out_path = PathBuf::from(&out_dir).join("__gluon_app.rs");

    println!("cargo:rerun-if-changed=app");

    if !app_dir.is_dir() {
        // No app/ directory: emit an empty router and return Ok.
        let empty = empty_generated();
        fs::write(&out_path, empty)?;
        return Ok(());
    }

    let entries = scan_app_dir(&app_dir)?;
    check_url_collisions(&entries)?;
    let generated = generate(&entries, Path::new(&manifest_dir));
    fs::write(&out_path, generated)?;

    Ok(())
}

/// Fails the build when two discovered entries would register the same URL
/// path, which axum would otherwise reject at startup with a panic.
fn check_url_collisions(entries: &[Entry]) -> Result<(), BuildError> {
    use std::collections::HashMap;
    let mut seen: HashMap<String, &Path> = HashMap::new();
    for entry in entries {
        let url = url_path_for(&entry.dir_segments);
        if let Some(prev) = seen.insert(url.clone(), entry.abs_path.as_path()) {
            return Err(BuildError::Other(format!(
                "route '{url}' is registered twice: {} and {}",
                prev.display(),
                entry.abs_path.display(),
            )));
        }
    }
    Ok(())
}

/// Recursively walk `app_dir`, parsing each `page.rs` / `route.rs`.
fn scan_app_dir(app_dir: &Path) -> Result<Vec<Entry>, BuildError> {
    let mut entries: Vec<Entry> = Vec::new();

    for dir_entry in WalkDir::new(app_dir) {
        let dir_entry = dir_entry.map_err(|e| {
            BuildError::Other(format!("walkdir failed under {}: {e}", app_dir.display()))
        })?;

        if !dir_entry.file_type().is_file() {
            continue;
        }

        let file_name = dir_entry.file_name().to_string_lossy().into_owned();
        let file_stem = match file_name.as_str() {
            "page.rs" => "page",
            "route.rs" => "route",
            _ => continue,
        };

        let abs_path = dir_entry.path().to_path_buf();
        let rel = abs_path.strip_prefix(app_dir).map_err(|e| {
            BuildError::Other(format!(
                "failed to strip prefix {} from {}: {e}",
                app_dir.display(),
                abs_path.display()
            ))
        })?;

        let mut dir_segments: Vec<String> = Vec::new();
        let components: Vec<_> = rel.components().collect();
        // The last component is the file itself; everything before is directories.
        let dir_count = components.len().saturating_sub(1);
        for comp in components.iter().take(dir_count) {
            dir_segments.push(comp.as_os_str().to_string_lossy().into_owned());
        }

        let source = fs::read_to_string(&abs_path)?;
        let parsed = syn::parse_file(&source)?;
        let methods = extract_http_methods(&parsed);

        entries.push(Entry {
            abs_path,
            dir_segments,
            file_stem: file_stem.to_owned(),
            methods,
        });
    }

    // Sort for deterministic output across builds.
    entries.sort_by(|a, b| a.abs_path.cmp(&b.abs_path));
    Ok(entries)
}

/// Extract `pub async fn get/post/put/patch/delete` items from a parsed file.
fn extract_http_methods(file: &syn::File) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for item in &file.items {
        if let Item::Fn(func) = item {
            if !is_pub_async(func) {
                continue;
            }
            let name = func.sig.ident.to_string();
            if HTTP_METHODS.contains(&name.as_str()) && !found.contains(&name) {
                found.push(name);
            }
        }
    }
    // Keep the spec order (get, post, put, patch, delete) for stability.
    found.sort_by_key(|m| {
        HTTP_METHODS
            .iter()
            .position(|x| *x == m.as_str())
            .unwrap_or(usize::MAX)
    });
    found
}

fn is_pub_async(func: &ItemFn) -> bool {
    matches!(func.vis, syn::Visibility::Public(_)) && func.sig.asyncness.is_some()
}

/// Generate the full content of `$OUT_DIR/__gluon_app.rs`.
///
/// `manifest_dir` is used to compute relative template paths in the generated
/// router so that the binary is not tied to the build machine's absolute path.
fn generate(entries: &[Entry], manifest_dir: &Path) -> String {
    let mut out = String::new();
    out.push_str("// @generated by gluon-build. Do not edit.\n");
    out.push_str(
        "#[allow(non_snake_case, dead_code, unused_imports, clippy::all, clippy::pedantic)]\n",
    );
    out.push_str("pub mod __gluon_app {\n");

    let tree = build_mod_tree(entries);
    emit_mod_tree(&mut out, &tree, 1);

    out.push_str("}\n\n");

    emit_router_fn(&mut out, entries, manifest_dir);
    out
}

/// Output produced when no `app/` directory exists.
fn empty_generated() -> String {
    let mut out = String::new();
    out.push_str("// @generated by gluon-build. Do not edit.\n");
    out.push_str(
        "#[allow(non_snake_case, dead_code, unused_imports, clippy::all, clippy::pedantic)]\n",
    );
    out.push_str("pub mod __gluon_app {}\n\n");
    out.push_str("pub fn __gluon_router() -> axum::Router<std::sync::Arc<gluon::Container>> {\n");
    out.push_str("    axum::Router::new()\n");
    out.push_str("}\n");
    out
}

/// In-memory tree representation of the discovered `app/` layout.
#[derive(Debug, Default)]
struct ModNode {
    /// Child directories keyed by their original (un-mangled) segment name.
    children: std::collections::BTreeMap<String, ModNode>,
    /// `page.rs` absolute path at this node, if any.
    page_file: Option<PathBuf>,
    /// `route.rs` absolute path at this node, if any.
    route_file: Option<PathBuf>,
}

fn build_mod_tree(entries: &[Entry]) -> ModNode {
    let mut root = ModNode::default();
    for entry in entries {
        let mut node = &mut root;
        for seg in &entry.dir_segments {
            node = node.children.entry(seg.clone()).or_default();
        }
        match entry.file_stem.as_str() {
            "page" => node.page_file = Some(entry.abs_path.clone()),
            "route" => node.route_file = Some(entry.abs_path.clone()),
            _ => {}
        }
    }
    root
}

/// Recursively emit the `mod` tree.
fn emit_mod_tree(out: &mut String, node: &ModNode, depth: usize) {
    if let Some(page) = &node.page_file {
        push_indent(out, depth);
        let _ = writeln!(out, "#[path = {}]", rust_string_literal(page));
        push_indent(out, depth);
        out.push_str("pub mod page;\n");
    }
    if let Some(route) = &node.route_file {
        push_indent(out, depth);
        let _ = writeln!(out, "#[path = {}]", rust_string_literal(route));
        push_indent(out, depth);
        out.push_str("pub mod route;\n");
    }
    for (seg, child) in &node.children {
        let mod_name = mangle_segment_for_mod(seg);
        push_indent(out, depth);
        let _ = writeln!(out, "pub mod {mod_name} {{");
        emit_mod_tree(out, child, depth + 1);
        push_indent(out, depth);
        out.push_str("}\n");
    }
}

fn push_indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("    ");
    }
}

/// Emit the `__gluon_router` function.
///
/// Template paths are embedded as `concat!(env!("CARGO_MANIFEST_DIR"), "/rel/path.tsx")`
/// rather than absolute paths so that the generated binary is not tied to the
/// build machine's filesystem layout.
fn emit_router_fn(out: &mut String, entries: &[Entry], manifest_dir: &Path) {
    out.push_str("pub fn __gluon_router() -> axum::Router<std::sync::Arc<gluon::Container>> {\n");
    out.push_str("    let router = axum::Router::new();\n");

    for entry in entries {
        let Some((first, rest)) = entry.methods.split_first() else {
            continue;
        };
        let url_path = url_path_for(&entry.dir_segments);
        let mod_path = mod_path_for(&entry.dir_segments, &entry.file_stem);

        let mut chain = format!("axum::routing::{first}({mod_path}::{first})");
        for m in rest {
            let _ = write!(chain, ".{m}({mod_path}::{m})");
        }

        // For page.rs entries, attach a middleware that injects the sibling
        // .tsx path into CURRENT_TEMPLATE so View<P>::into_response can find
        // the template without requiring the handler to specify it explicitly.
        let template_path = if entry.file_stem == "page" {
            entry
                .abs_path
                .parent()
                .map(|p| p.join("page.tsx"))
                .filter(|p| p.is_file())
        } else {
            None
        };

        if let Some(template) = template_path {
            let template_expr = template_path_expr(&template, manifest_dir);
            let _ = writeln!(
                out,
                "    let router = router.route({url}, {{\n        let template = {template_expr};\n        let mw = axum::middleware::from_fn(move |req: axum::extract::Request, next: axum::middleware::Next| {{\n            let template = template.clone();\n            async move {{ gluon::view::CURRENT_TEMPLATE.scope(Some(template), next.run(req)).await }}\n        }});\n        ({chain}).layer(mw)\n    }});",
                url = rust_string_literal_str(&url_path),
            );
        } else {
            let _ = writeln!(
                out,
                "    let router = router.route({url}, {chain});",
                url = rust_string_literal_str(&url_path),
            );
        }
    }

    out.push_str("    router\n");
    out.push_str("}\n");
}

/// Build a Rust expression that evaluates to a `PathBuf` for the given
/// template path. Uses `concat!(env!("CARGO_MANIFEST_DIR"), "/rel/path")` when
/// the path is inside the project so the binary works on any machine where the
/// source tree is present at the same relative layout. Falls back to an
/// absolute string literal only when the path is outside the manifest dir.
fn template_path_expr(abs: &Path, manifest_dir: &Path) -> String {
    if let Ok(rel) = abs.strip_prefix(manifest_dir) {
        // Forward-slash normalisation for Windows hosts.
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        format!(r#"std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/{rel_str}"))"#)
    } else {
        format!("std::path::PathBuf::from({})", rust_string_literal(abs))
    }
}

/// Compute the URL path for a directory segment list.
///
/// Rules:
/// - `[id]` -> `:id`
/// - `[...slug]` -> `*slug`
/// - `(group)` -> removed from URL
/// - root (empty segments) -> `/`
fn url_path_for(dir_segments: &[String]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for seg in dir_segments {
        if is_group_segment(seg) {
            continue;
        }
        parts.push(transform_url_segment(seg));
    }
    if parts.is_empty() {
        "/".to_owned()
    } else {
        let mut s = String::from("/");
        s.push_str(&parts.join("/"));
        s
    }
}

fn transform_url_segment(seg: &str) -> String {
    if let Some(inner) = strip_brackets(seg) {
        if let Some(rest) = inner.strip_prefix("...") {
            return format!("*{rest}");
        }
        return format!(":{inner}");
    }
    seg.to_owned()
}

fn is_group_segment(seg: &str) -> bool {
    seg.starts_with('(') && seg.ends_with(')') && seg.len() >= 2
}

fn strip_brackets(seg: &str) -> Option<&str> {
    if seg.starts_with('[') && seg.ends_with(']') && seg.len() >= 2 {
        Some(&seg[1..seg.len() - 1])
    } else {
        None
    }
}

/// Mangle a directory segment to a valid Rust identifier for use as a mod name.
fn mangle_segment_for_mod(seg: &str) -> String {
    if let Some(inner) = strip_brackets(seg) {
        if let Some(rest) = inner.strip_prefix("...") {
            return format!("_catch_{}", sanitize_ident(rest));
        }
        return format!("_dyn_{}", sanitize_ident(inner));
    }
    if is_group_segment(seg) {
        let inner = &seg[1..seg.len() - 1];
        return format!("_group_{}", sanitize_ident(inner));
    }
    sanitize_ident(seg)
}

/// Replace any character that is not a valid Rust identifier continuation with `_`.
fn sanitize_ident(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, c) in s.chars().enumerate() {
        let ok = if i == 0 {
            c.is_ascii_alphabetic() || c == '_'
        } else {
            c.is_ascii_alphanumeric() || c == '_'
        };
        if ok {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

/// Build the Rust path expression to refer to a discovered file's module.
/// e.g. `["users", "[id]"]`, `"page"` -> `__gluon_app::users::_dyn_id::page`.
fn mod_path_for(dir_segments: &[String], file_stem: &str) -> String {
    let mut parts: Vec<String> = vec!["__gluon_app".to_owned()];
    for seg in dir_segments {
        parts.push(mangle_segment_for_mod(seg));
    }
    parts.push(file_stem.to_owned());
    parts.join("::")
}

/// Format a path as a valid Rust string literal (quoted, escaped).
fn rust_string_literal(p: &Path) -> String {
    rust_string_literal_str(&p.to_string_lossy())
}

fn rust_string_literal_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_path_root() {
        assert_eq!(url_path_for(&[]), "/");
    }

    #[test]
    fn url_path_static() {
        assert_eq!(
            url_path_for(&["users".to_owned(), "list".to_owned()]),
            "/users/list"
        );
    }

    #[test]
    fn url_path_dynamic() {
        assert_eq!(
            url_path_for(&["users".to_owned(), "[id]".to_owned()]),
            "/users/:id"
        );
    }

    #[test]
    fn url_path_catch_all() {
        assert_eq!(
            url_path_for(&["files".to_owned(), "[...path]".to_owned()]),
            "/files/*path"
        );
    }

    #[test]
    fn url_path_group_removed() {
        assert_eq!(
            url_path_for(&["(marketing)".to_owned(), "about".to_owned()]),
            "/about"
        );
    }

    #[test]
    fn mangle_dynamic() {
        assert_eq!(mangle_segment_for_mod("[id]"), "_dyn_id");
        assert_eq!(mangle_segment_for_mod("[...slug]"), "_catch_slug");
        assert_eq!(mangle_segment_for_mod("(group)"), "_group_group");
        assert_eq!(mangle_segment_for_mod("users"), "users");
    }

    #[test]
    fn rust_string_literal_escapes() {
        assert_eq!(rust_string_literal_str("a\\b\"c"), "\"a\\\\b\\\"c\"");
    }

    // ---------- scan() / scan_app_dir ----------

    use std::io::Write as _;

    /// Write `body` to `dir/<rel>` creating parent directories as needed.
    fn write_file(dir: &Path, rel: &str, body: &str) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
    }

    #[test]
    fn scan_returns_empty_when_app_dir_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("does-not-exist");
        let entries = scan(&missing).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn scan_returns_empty_when_app_dir_has_no_pages() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        write_file(&app, "other.rs", "// not a page or route\n");
        let entries = scan(&app).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn scan_finds_simple_page() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "page.rs", "pub async fn get() {}\n");
        let entries = scan(&app).unwrap();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert!(e.dir_segments.is_empty());
        assert_eq!(e.file_stem, "page");
        assert_eq!(e.methods, vec!["get".to_owned()]);
    }

    #[test]
    fn scan_finds_route_and_page_in_different_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "users/page.rs", "pub async fn get() {}\n");
        write_file(
            &app,
            "api/health/route.rs",
            "pub async fn get() {}\npub async fn post() {}\n",
        );
        let entries = scan(&app).unwrap();
        assert_eq!(entries.len(), 2);
        let urls: Vec<String> = entries.iter().map(Entry::url_path).collect();
        assert!(urls.contains(&"/users".to_owned()));
        assert!(urls.contains(&"/api/health".to_owned()));
    }

    #[test]
    fn scan_deep_nesting() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "a/b/c/page.rs", "pub async fn get() {}\n");
        let entries = scan(&app).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].dir_segments,
            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
        );
    }

    #[test]
    fn scan_handles_multiple_pages_sorted_by_path() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "zeta/page.rs", "pub async fn get() {}\n");
        write_file(&app, "alpha/page.rs", "pub async fn get() {}\n");
        write_file(&app, "beta/page.rs", "pub async fn get() {}\n");
        let entries = scan(&app).unwrap();
        assert_eq!(entries.len(), 3);
        for w in entries.windows(2) {
            assert!(w[0].abs_path <= w[1].abs_path, "entries not sorted");
        }
    }

    // ---------- check_url_collisions ----------

    fn make_entry(dir_segments: &[&str], file_stem: &str, abs: &str) -> Entry {
        Entry {
            abs_path: PathBuf::from(abs),
            dir_segments: dir_segments.iter().map(|s| (*s).to_owned()).collect(),
            file_stem: file_stem.to_owned(),
            methods: vec!["get".to_owned()],
        }
    }

    #[test]
    fn check_url_collisions_detects_duplicate() {
        let entries = vec![
            make_entry(&["x"], "page", "/tmp/a/page.rs"),
            make_entry(&["x"], "page", "/tmp/b/page.rs"),
        ];
        let err = check_url_collisions(&entries).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("/tmp/a/page.rs"), "msg: {msg}");
        assert!(msg.contains("/tmp/b/page.rs"), "msg: {msg}");
    }

    #[test]
    fn check_url_collisions_detects_group_normalization() {
        let entries = vec![
            make_entry(&["(g)", "about"], "page", "/tmp/g/about/page.rs"),
            make_entry(&["about"], "page", "/tmp/about/page.rs"),
        ];
        let err = check_url_collisions(&entries).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("/about"), "msg: {msg}");
    }

    #[test]
    fn check_url_collisions_ok_when_unique() {
        let entries = vec![
            make_entry(&["a"], "page", "/tmp/a/page.rs"),
            make_entry(&["b"], "page", "/tmp/b/page.rs"),
            make_entry(&["c"], "route", "/tmp/c/route.rs"),
        ];
        assert!(check_url_collisions(&entries).is_ok());
    }

    // ---------- extract_http_methods ----------

    #[test]
    fn extracts_only_pub_async_fn() {
        let parsed: syn::File =
            syn::parse_str("pub async fn get() {} async fn post() {} pub fn put() {}").unwrap();
        assert_eq!(extract_http_methods(&parsed), vec!["get".to_owned()]);
    }

    #[test]
    fn ignores_unsupported_method_names() {
        let parsed: syn::File =
            syn::parse_str("pub async fn handle() {} pub async fn foo() {}").unwrap();
        assert!(extract_http_methods(&parsed).is_empty());
    }

    #[test]
    fn dedupes_repeated_methods() {
        let parsed: syn::File =
            syn::parse_str("pub async fn get() {} pub async fn get() {}").unwrap();
        assert_eq!(extract_http_methods(&parsed), vec!["get".to_owned()]);
    }

    #[test]
    fn orders_by_http_methods_constant() {
        let parsed: syn::File =
            syn::parse_str("pub async fn delete() {} pub async fn get() {} pub async fn post() {}")
                .unwrap();
        assert_eq!(
            extract_http_methods(&parsed),
            vec!["get".to_owned(), "post".to_owned(), "delete".to_owned()]
        );
    }

    #[test]
    fn ignores_impl_block_methods() {
        let parsed: syn::File =
            syn::parse_str("struct Foo; impl Foo { pub async fn get(&self) {} }").unwrap();
        assert!(extract_http_methods(&parsed).is_empty());
    }

    // ---------- BuildError ----------

    #[test]
    fn build_error_display_io() {
        let err = BuildError::Io(std::io::Error::other("x"));
        let s = err.to_string();
        assert!(s.contains("gluon-build io error"), "s: {s}");
        assert!(s.contains('x'), "s: {s}");
    }

    #[test]
    fn build_error_display_syn() {
        let Err(syn_err) = syn::parse_str::<syn::File>("fn (") else {
            panic!("expected parse error");
        };
        let err = BuildError::Syn(syn_err);
        let s = err.to_string();
        assert!(s.contains("gluon-build parse error"), "s: {s}");
    }

    #[test]
    fn build_error_display_other() {
        let err = BuildError::Other("z".to_owned());
        assert_eq!(err.to_string(), "gluon-build error: z");
    }

    #[test]
    fn build_error_source_propagates() {
        use std::error::Error as _;
        let io = BuildError::Io(std::io::Error::other("io"));
        assert!(io.source().is_some());
        let other = BuildError::Other("nope".to_owned());
        assert!(other.source().is_none());
    }

    #[test]
    fn build_error_from_io_and_syn() {
        let io_err: BuildError = std::io::Error::other("x").into();
        assert!(matches!(io_err, BuildError::Io(_)));
        let Err(raw) = syn::parse_str::<syn::File>("fn (") else {
            panic!("expected parse error");
        };
        let syn_err: BuildError = raw.into();
        assert!(matches!(syn_err, BuildError::Syn(_)));
    }

    // ---------- generate / empty_generated ----------

    #[test]
    fn empty_generated_contains_router_fn() {
        let s = empty_generated();
        assert!(s.contains("pub fn __gluon_router"), "s: {s}");
        assert!(s.contains("axum::Router::new()"), "s: {s}");
    }

    #[test]
    fn generate_includes_router_call_for_entry() {
        let entry = make_entry(&[], "page", "/tmp/page.rs");
        let s = generate(std::slice::from_ref(&entry), Path::new("/tmp"));
        assert!(s.contains("axum::routing::get"), "s: {s}");
        assert!(s.contains("\"/\""), "s: {s}");
    }

    #[test]
    fn generate_emits_layer_for_page_with_tsx() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "page.rs", "pub async fn get() {}\n");
        write_file(&app, "page.tsx", "export default function Page() {}\n");
        let entries = scan(&app).unwrap();
        let s = generate(&entries, tmp.path());
        assert!(s.contains("gluon::view::CURRENT_TEMPLATE.scope"), "s: {s}");
    }

    #[test]
    fn generate_emits_concat_env_for_tsx() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "page.rs", "pub async fn get() {}\n");
        write_file(&app, "page.tsx", "export default function Page() {}\n");
        let entries = scan(&app).unwrap();
        let s = generate(&entries, tmp.path());
        // The router function must use env!("CARGO_MANIFEST_DIR") for the tsx
        // template path so the binary is not tied to the build machine's absolute
        // path. The mod tree #[path] attributes still use absolute paths (those
        // are compile-time only and are always evaluated on the build machine).
        let router_start = s.find("pub fn __gluon_router").unwrap_or(s.len());
        let router_section = &s[router_start..];
        assert!(
            router_section.contains(r#"env!("CARGO_MANIFEST_DIR")"#),
            "router should use env! macro for tsx path; s: {s}"
        );
        assert!(
            !router_section.contains(tmp.path().to_string_lossy().as_ref()),
            "router must not embed absolute build-machine path for tsx; s: {s}"
        );
    }

    #[test]
    fn generate_omits_layer_for_route_rs() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "api/route.rs", "pub async fn get() {}\n");
        // Even with a sibling .tsx, route.rs must not get the layer.
        write_file(&app, "api/route.tsx", "export default function R() {}\n");
        let entries = scan(&app).unwrap();
        let s = generate(&entries, tmp.path());
        assert!(
            !s.contains("CURRENT_TEMPLATE.scope"),
            "route.rs must not get template layer; s: {s}",
        );
    }

    // ---------- mangle_segment_for_mod / mod_path_for / sanitize_ident ----------

    #[test]
    fn mangle_catch_all() {
        assert_eq!(mangle_segment_for_mod("[...slug]"), "_catch_slug");
    }

    #[test]
    fn mangle_group() {
        assert_eq!(mangle_segment_for_mod("(admin)"), "_group_admin");
    }

    #[test]
    fn mangle_empty_brackets() {
        // "[]" -> strip_brackets succeeds (len >= 2), inner "" -> "_dyn_" + sanitize_ident("")
        // sanitize_ident("") returns "_", so the final value is "_dyn__".
        assert_eq!(mangle_segment_for_mod("[]"), "_dyn__");
    }

    #[test]
    fn sanitize_ident_leading_digit() {
        // First char '1' is not alpha/underscore -> '_'; remaining 'a','b','c' kept.
        assert_eq!(sanitize_ident("1abc"), "_abc");
    }

    #[test]
    fn sanitize_ident_punctuation() {
        assert_eq!(sanitize_ident("a-b"), "a_b");
    }

    #[test]
    fn sanitize_ident_empty() {
        assert_eq!(sanitize_ident(""), "_");
    }

    #[test]
    fn mod_path_for_dynamic() {
        assert_eq!(
            mod_path_for(&["users".to_owned(), "[id]".to_owned()], "page"),
            "__gluon_app::users::_dyn_id::page"
        );
    }

    #[test]
    fn mod_path_for_catch_all_and_group() {
        assert_eq!(
            mod_path_for(&["(g)".to_owned(), "[...rest]".to_owned()], "route"),
            "__gluon_app::_group_g::_catch_rest::route"
        );
    }

    // ---------- url_path_for extras ----------

    #[test]
    fn url_path_dynamic_in_group_combined() {
        assert_eq!(
            url_path_for(&["(g)".to_owned(), "users".to_owned(), "[id]".to_owned(),]),
            "/users/:id"
        );
    }

    // ---------- transform_url_segment / strip_brackets / is_group_segment ----------

    #[test]
    fn strip_brackets_returns_inner() {
        assert_eq!(strip_brackets("[abc]"), Some("abc"));
    }

    #[test]
    fn strip_brackets_none_for_unbalanced() {
        assert!(strip_brackets("[abc").is_none());
        assert!(strip_brackets("abc]").is_none());
    }

    #[test]
    fn is_group_segment_requires_balanced_parens() {
        assert!(is_group_segment("(abc)"));
        assert!(!is_group_segment("(abc"));
        assert!(!is_group_segment("abc)"));
    }

    #[test]
    fn transform_url_empty_dynamic() {
        // "[]" -> inner "" -> ":" (current behavior, pinned by this test)
        assert_eq!(transform_url_segment("[]"), ":");
    }

    #[test]
    fn transform_url_empty_catch_all() {
        // "[...]" -> inner "..." -> strip_prefix("...") = Some("") -> "*"
        assert_eq!(transform_url_segment("[...]"), "*");
    }

    // ---------- rust_string_literal_str extras ----------

    #[test]
    fn escapes_control_characters() {
        // U+0001 < 0x20 -> \u{1}
        assert_eq!(rust_string_literal_str("\x01"), "\"\\u{1}\"");
    }

    #[test]
    fn preserves_unicode() {
        // sakoku-ignore-next-line
        let s = rust_string_literal_str("日本語");
        // sakoku-ignore-next-line
        assert!(s.contains("日本語"), "s: {s}");
    }

    #[test]
    fn rust_string_literal_from_path() {
        assert_eq!(rust_string_literal(Path::new("a/b.rs")), "\"a/b.rs\"");
    }

    // ---------- HTTP_METHODS constant ----------

    #[test]
    fn http_methods_constant_is_stable() {
        assert_eq!(HTTP_METHODS, &["get", "post", "put", "patch", "delete"]);
    }

    // ---------- Entry::url_path ----------

    #[test]
    fn entry_url_path_delegates_to_url_path_for() {
        let entry = make_entry(&["users"], "page", "/tmp/users/page.rs");
        assert_eq!(entry.url_path(), "/users");
    }

    // ---------- build_mod_tree ----------

    #[test]
    fn build_mod_tree_merges_page_and_route_at_same_path() {
        let entries = vec![
            make_entry(&["api"], "page", "/tmp/api/page.rs"),
            make_entry(&["api"], "route", "/tmp/api/route.rs"),
        ];
        let tree = build_mod_tree(&entries);
        let api = tree.children.get("api").expect("api child missing");
        assert!(api.page_file.is_some(), "page_file should be set");
        assert!(api.route_file.is_some(), "route_file should be set");
    }

    // ---------- run() E2E via env vars ----------

    use serial_test::serial;

    #[test]
    #[serial]
    fn run_creates_empty_router_when_no_app_dir() {
        let manifest = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        // Intentionally do NOT create app/ inside manifest dir.
        // SAFETY: serial_test::serial ensures no parallel access to env.
        unsafe {
            std::env::set_var("CARGO_MANIFEST_DIR", manifest.path());
            std::env::set_var("OUT_DIR", out.path());
        }
        let result = run();
        // Clean up env regardless of result.
        unsafe {
            std::env::remove_var("CARGO_MANIFEST_DIR");
            std::env::remove_var("OUT_DIR");
        }
        result.unwrap();
        let generated = std::fs::read_to_string(out.path().join("__gluon_app.rs")).unwrap();
        assert!(
            generated.contains("axum::Router::new()"),
            "generated: {generated}",
        );
    }

    #[test]
    #[serial]
    fn run_returns_other_error_when_manifest_dir_missing() {
        // SAFETY: serial_test::serial ensures no parallel access to env.
        unsafe {
            std::env::remove_var("CARGO_MANIFEST_DIR");
            std::env::remove_var("OUT_DIR");
        }
        let result = run();
        assert!(matches!(result, Err(BuildError::Other(_))));
    }
}
