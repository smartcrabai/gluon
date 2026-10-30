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
//! Scans the consuming application's `app/` tree for route handlers, embeds
//! every TSX template and `public/` asset at compile time, and generates the
//! Rust router in `$OUT_DIR/__gluon_app.rs`. Relative TSX imports resolve from
//! the generated source map. Page routes inherit layouts and error templates;
//! route handlers retain their response bodies, and sibling `route.tsx` files
//! support View responses and SSE event fragments. The router embeds `gluon.toml`
//! when present.

use std::collections::BTreeSet;
use std::fmt;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use syn::{Item, ItemFn};
use walkdir::WalkDir;

/// Errors that can occur while running the build-time scanner / code generator.
#[derive(Debug)]
pub enum BuildError {
    /// I/O failure while reading the `app/` tree or `gluon.toml`, or while writing
    /// the generated file.
    Io(std::io::Error),
    /// `syn` failed to parse a discovered Rust source file.
    Syn {
        path: Option<PathBuf>,
        source: syn::Error,
    },
    /// Other build-time failures (environment variables, malformed paths, ...).
    Other(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "gluon-build io error: {e}"),
            Self::Syn {
                path: Some(path),
                source,
            } => write!(f, "gluon-build parse error in {}: {source}", path.display()),
            Self::Syn { path: None, source } => write!(f, "gluon-build parse error: {source}"),
            Self::Other(msg) => write!(f, "gluon-build error: {msg}"),
        }
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Syn { source, .. } => Some(source),
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
        Self::Syn {
            path: None,
            source: e,
        }
    }
}

/// HTTP methods that gluon recognises as page/route handlers, in declaration
/// order. Exposed so the CLI's `gluon routes` command can share the list.
pub const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete"];
const TEMPLATE_EXTENSIONS: &[&str] = &["tsx", "jsx", "ts", "js"];

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
    /// Returns the URL path this entry serves (`/users/{id}`, `/`, ...).
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

/// Recursively collects regular files under a source tree.
///
/// An empty extension list matches every file; otherwise only files with a listed
/// extension are returned. Symlinks inside the tree are followed, but targets
/// outside the tree are rejected.
fn scan_files(root: &Path, extensions: &[&str]) -> Result<Vec<PathBuf>, BuildError> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let canonical_root = fs::canonicalize(root)?;
    let mut files = Vec::new();
    for entry in WalkDir::new(root).follow_links(true) {
        let entry = entry.map_err(|error| {
            BuildError::Other(format!("walkdir failed under {}: {error}", root.display()))
        })?;
        let supported_extension = entry
            .path()
            .extension()
            .is_some_and(|extension| extensions.iter().any(|candidate| extension == *candidate));
        if entry.file_type().is_file() && (extensions.is_empty() || supported_extension) {
            let canonical_path = fs::canonicalize(entry.path())?;
            if !canonical_path.starts_with(&canonical_root) {
                return Err(BuildError::Other(format!(
                    "source file {} resolves outside {}",
                    entry.path().display(),
                    root.display()
                )));
            }
            files.push(entry.into_path());
        }
    }
    files.sort();
    Ok(files)
}

fn write_generated_if_changed(path: &Path, contents: &str) -> Result<bool, BuildError> {
    if let Ok(previous) = fs::read_to_string(path)
        && previous == contents
    {
        return Ok(false);
    }
    fs::write(path, contents)?;
    Ok(true)
}
fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hash = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        hash.push(char::from(HEX[usize::from(byte >> 4)]));
        hash.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    hash
}

#[derive(Debug)]
struct TailwindStylesheet {
    file_name: String,
    url: String,
    content_globs: Vec<String>,
}

fn build_tailwind_stylesheet(
    app_sources: &[PathBuf],
    manifest_dir: &Path,
    out_dir: &Path,
    embedded_config: Option<&str>,
) -> Result<(Option<TailwindStylesheet>, Vec<PathBuf>), BuildError> {
    let Some(config) = embedded_config else {
        return Ok((None, Vec::new()));
    };
    let config: toml::Value = toml::from_str(config)
        .map_err(|error| BuildError::Other(format!("failed to parse gluon.toml: {error}")))?;
    let Some(view) = config.get("view").and_then(toml::Value::as_table) else {
        return Ok((None, Vec::new()));
    };
    let tailwind_enabled = view
        .get("tailwind")
        .and_then(toml::Value::as_bool)
        .unwrap_or(false);
    if !tailwind_enabled {
        return Ok((None, Vec::new()));
    }
    let patterns = match view.get("tailwind_content") {
        Some(value) => value
            .as_array()
            .ok_or_else(|| {
                BuildError::Other(
                    "[view].tailwind_content must be an array of glob patterns".into(),
                )
            })?
            .iter()
            .map(|value| {
                value.as_str().map(str::to_owned).ok_or_else(|| {
                    BuildError::Other(
                        "[view].tailwind_content entries must be glob pattern strings".into(),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };

    let mut source_paths = BTreeSet::new();
    let mut rerun_paths = BTreeSet::new();
    for path in app_sources {
        source_paths.insert(path.clone());
        rerun_paths.insert(path.clone());
    }
    for pattern in &patterns {
        let glob_path = manifest_dir.join(pattern);
        rerun_paths.insert(glob_watch_root(manifest_dir, pattern)?);
        let glob_pattern = glob_path.to_string_lossy();
        let paths = glob::glob(&glob_pattern).map_err(|error| {
            BuildError::Other(format!(
                "invalid [view].tailwind_content glob '{pattern}': {error}"
            ))
        })?;
        for path in paths {
            let path = path.map_err(|error| {
                BuildError::Other(format!(
                    "failed to scan [view].tailwind_content glob '{pattern}': {error}"
                ))
            })?;
            if let Some(parent) = path.parent() {
                rerun_paths.insert(parent.to_path_buf());
            }
            if path.is_file() {
                source_paths.insert(path.clone());
                rerun_paths.insert(path);
            } else if path.is_dir() {
                rerun_paths.insert(path);
            }
        }
    }

    let sources = source_paths
        .iter()
        .map(fs::read_to_string)
        .collect::<Result<Vec<_>, _>>()?;
    let css = encre_css::generate(
        sources.iter().map(String::as_str),
        &encre_css::Config::default(),
    );
    let hash = sha256_hex(css.as_bytes());
    let file_name = format!("app.{hash}.css");
    let url = format!("/_gluon/{file_name}");
    write_generated_if_changed(&out_dir.join(&file_name), &css)?;

    Ok((
        Some(TailwindStylesheet {
            file_name,
            url,
            content_globs: patterns,
        }),
        rerun_paths.into_iter().collect(),
    ))
}

fn glob_watch_root(manifest_dir: &Path, pattern: &str) -> Result<PathBuf, BuildError> {
    let glob_path = manifest_dir.join(pattern);
    let mut root = PathBuf::new();
    let mut contains_glob = false;
    for component in glob_path.components() {
        let part = component.as_os_str().to_string_lossy();
        if part
            .chars()
            .any(|character| matches!(character, '*' | '?' | '[' | '{'))
        {
            contains_glob = true;
            break;
        }
        root.push(component.as_os_str());
    }
    if !contains_glob {
        root = glob_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
    }
    while !root.is_dir() && root != manifest_dir {
        let Some(parent) = root.parent() else {
            break;
        };
        root = parent.to_path_buf();
    }
    if root == manifest_dir || !root.starts_with(manifest_dir) {
        return Err(BuildError::Other(format!(
            "[view].tailwind_content glob '{pattern}' must have an existing directory below {}",
            manifest_dir.display()
        )));
    }
    Ok(root)
}

/// Entry point. Call this from `build.rs`.
///
/// # Errors
///
/// Returns [`BuildError`] when the `app/`, `public/`, or `gluon.toml` source
/// cannot be read, a discovered `.rs` file fails to parse, or the generated
/// file cannot be written.
pub fn run() -> Result<(), BuildError> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|e| BuildError::Other(format!("CARGO_MANIFEST_DIR not set: {e}")))?;
    let out_dir =
        std::env::var("OUT_DIR").map_err(|e| BuildError::Other(format!("OUT_DIR not set: {e}")))?;

    let manifest_dir = PathBuf::from(manifest_dir);
    let app_dir = manifest_dir.join("app");
    let public_dir = manifest_dir.join("public");
    let config_path = manifest_dir.join("gluon.toml");
    let out_dir = PathBuf::from(out_dir);
    let out_path = out_dir.join("__gluon_app.rs");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=app");
    println!("cargo:rerun-if-changed=gluon.toml");
    println!("cargo:rerun-if-changed=public");

    let embedded_config = match fs::read_to_string(&config_path) {
        Ok(config) => Some(config),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let entries = scan(&app_dir)?;
    check_url_collisions(&entries)?;
    let templates = scan_files(&app_dir, TEMPLATE_EXTENSIONS)?;
    let assets = scan_files(&public_dir, &[])?;
    let (tailwind_stylesheet, tailwind_inputs) = build_tailwind_stylesheet(
        &templates,
        &manifest_dir,
        &out_dir,
        embedded_config.as_deref(),
    )?;
    for path in tailwind_inputs {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let generated = generate_with_embedded(
        &entries,
        &manifest_dir,
        &templates,
        &assets,
        embedded_config.as_deref(),
        tailwind_stylesheet.as_ref(),
    );
    write_generated_if_changed(&out_path, &generated)?;

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
        let parsed = syn::parse_file(&source).map_err(|source| BuildError::Syn {
            path: Some(abs_path.clone()),
            source,
        })?;
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

/// Generate `$OUT_DIR/__gluon_app.rs`, including compile-time templates and assets.
///
/// `manifest_dir` supplies include paths and the runtime source root.
#[cfg(test)]
fn generate(entries: &[Entry], manifest_dir: &Path) -> String {
    generate_with_embedded(entries, manifest_dir, &[], &[], None, None)
}

fn generate_with_embedded(
    entries: &[Entry],
    manifest_dir: &Path,
    templates: &[PathBuf],
    assets: &[PathBuf],
    embedded_config: Option<&str>,
    tailwind_stylesheet: Option<&TailwindStylesheet>,
) -> String {
    let mut out = String::new();
    out.push_str("// @generated by gluon-build. Do not edit.\n");
    out.push_str(
        "#[allow(non_snake_case, dead_code, unused_imports, clippy::all, clippy::pedantic)]\n",
    );
    out.push_str("pub mod __gluon_app {\n");

    let tree = build_mod_tree(entries);
    emit_mod_tree(&mut out, &tree, 1);

    out.push_str("}\n\n");
    emit_embedded_sources(&mut out, templates, assets, manifest_dir);
    let stylesheet_url = tailwind_stylesheet.map_or_else(
        || "None".to_owned(),
        |stylesheet| format!("Some({})", rust_string_literal_str(&stylesheet.url)),
    );
    let _ = writeln!(
        out,
        "#[allow(dead_code)]\npub const __GLUON_TAILWIND_STYLESHEET: Option<&'static str> = {stylesheet_url};\n"
    );
    emit_router_fn(
        &mut out,
        entries,
        manifest_dir,
        embedded_config,
        manifest_dir.join("public").is_dir(),
        tailwind_stylesheet,
    );
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

/// Emit the `__gluon_router` function, config registration, app routes, and public service.
fn emit_router_fn(
    out: &mut String,
    entries: &[Entry],
    manifest_dir: &Path,
    embedded_config: Option<&str>,
    has_public_directory: bool,
    tailwind_stylesheet: Option<&TailwindStylesheet>,
) {
    out.push_str("#[allow(clippy::let_and_return)]\n");
    out.push_str("pub fn __gluon_router() -> axum::Router<std::sync::Arc<gluon::Container>> {\n");
    if let Some(config) = embedded_config {
        let _ = writeln!(
            out,
            "    gluon::__register_embedded_config({});",
            rust_string_literal_str(config),
        );
    }
    out.push_str("    let mut router = axum::Router::new();\n");
    let has_page_rendering = entries
        .iter()
        .any(|entry| entry.file_stem == "page" && !entry.methods.is_empty());
    let has_route_templates = entries.iter().any(|entry| {
        entry.file_stem == "route"
            && !entry.methods.is_empty()
            && entry
                .abs_path
                .parent()
                .is_some_and(|parent| parent.join("route.tsx").is_file())
    });
    let has_template_context = has_page_rendering || has_route_templates;
    if has_template_context {
        out.push_str("    let source_map = __gluon_source_map();\n");
    }
    if has_template_context || has_public_directory || tailwind_stylesheet.is_some() {
        out.push_str(
            "    let source_root = std::path::PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"));\n",
        );
    }
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

        if entry.file_stem == "page" {
            emit_page_route(out, entry, &url_path, &chain, manifest_dir);
        } else if let Some(template) = entry
            .abs_path
            .parent()
            .map(|parent| parent.join("route.tsx"))
            .filter(|path| path.is_file())
        {
            emit_route_template(out, &url_path, &chain, &template, manifest_dir);
        } else {
            let _ = writeln!(
                out,
                "    router = router.route({url}, {chain});",
                url = rust_string_literal_str(&url_path),
            );
        }
    }

    if let Some(stylesheet) = tailwind_stylesheet {
        let url = rust_string_literal_str(&stylesheet.url);
        let asset = rust_string_literal_str(&format!("/{}", stylesheet.file_name));
        let content_globs = stylesheet
            .content_globs
            .iter()
            .map(|pattern| rust_string_literal_str(pattern))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "    router = router.route({url}, axum::routing::get({{\n        let source_root = source_root.clone();\n        move || gluon::middleware::static_files::serve_tailwind_css(include_bytes!(concat!(env!(\"OUT_DIR\"), {asset})), source_root.clone(), &[{content_globs}])\n    }}));"
        );
    }

    if has_public_directory {
        out.push_str("    let public_service = if std::env::var_os(\"GLUON_DEV_TEMPLATES\").as_deref() == Some(std::ffi::OsStr::new(\"1\")) {\n");
        out.push_str(
            "        gluon::middleware::static_files::service(source_root.join(\"public\"))\n",
        );
        out.push_str("    } else {\n");
        out.push_str(
            "        gluon::middleware::static_files::embedded_service(__gluon_public_assets())\n",
        );
        out.push_str("    };\n");
        out.push_str("    router = router.nest_service(\"/public\", public_service);\n");
    }
    out.push_str("    router\n");
    out.push_str("}\n");
}

fn emit_page_route(
    out: &mut String,
    entry: &Entry,
    url_path: &str,
    chain: &str,
    manifest_dir: &Path,
) {
    let app_dir = manifest_dir.join("app");
    let template = entry
        .abs_path
        .parent()
        .map(|parent| parent.join("page.tsx"))
        .filter(|path| path.is_file());
    let template_expr = template.as_deref().map_or_else(
        || "None".to_owned(),
        |path| format!("Some({})", embedded_template_path_expr(path, manifest_dir)),
    );
    let layouts_expr = format!(
        "vec![{}]",
        page_layout_paths(&entry.abs_path, &app_dir)
            .iter()
            .map(|path| embedded_template_path_expr(path, manifest_dir))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let error_dir_expr = embedded_template_path_expr(&app_dir.join("_error"), manifest_dir);
    let root_layout_path = app_dir.join("layout.tsx");
    let root_layout_expr = if root_layout_path.is_file() {
        format!(
            "Some({})",
            embedded_template_path_expr(&root_layout_path, manifest_dir)
        )
    } else {
        "None".to_owned()
    };
    let _ = writeln!(
        out,
        "    router = router.route({url}, {{\n        let template = {template};\n        let layouts = {layouts};\n        let error_dir = {error_dir};\n        let root_layout = {root_layout};\n        let source_map = source_map.clone();\n        let source_root = source_root.clone();\n        let mw = axum::middleware::from_fn(move |req: axum::extract::Request, next: axum::middleware::Next| {{\n            let template = template.clone();\n            let layouts = layouts.clone();\n            let error_dir = error_dir.clone();\n            let root_layout = root_layout.clone();\n            let source_map = source_map.clone();\n            let source_root = source_root.clone();\n            async move {{ gluon::view::render_page_request(req, next, gluon::view::PageRenderContext {{ template, layouts, error_dir, root_layout, source_map, source_root, stylesheet_url: __GLUON_TAILWIND_STYLESHEET }}).await }}\n        }});\n        ({chain}).layer(mw)\n    }});",
        url = rust_string_literal_str(url_path),
        template = template_expr,
        layouts = layouts_expr,
        error_dir = error_dir_expr,
        root_layout = root_layout_expr,
    );
}

fn emit_route_template(
    out: &mut String,
    url_path: &str,
    chain: &str,
    template: &Path,
    manifest_dir: &Path,
) {
    let template_expr = embedded_template_path_expr(template, manifest_dir);
    let _ = writeln!(
        out,
        "    router = router.route({url}, {{\n        let template = {template};\n        let source_map = source_map.clone();\n        let source_root = source_root.clone();\n        let mw = axum::middleware::from_fn(move |req: axum::extract::Request, next: axum::middleware::Next| {{\n            let template = template.clone();\n            let source_map = source_map.clone();\n            let source_root = source_root.clone();\n            async move {{ gluon::view::render_route_request(req, next, template, source_map, source_root, __GLUON_TAILWIND_STYLESHEET).await }}\n        }});\n        ({chain}).layer(mw)\n    }});",
        url = rust_string_literal_str(url_path),
        template = template_expr,
    );
}

fn embedded_template_path_expr(path: &Path, manifest_dir: &Path) -> String {
    let rel = path
        .strip_prefix(manifest_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    format!(
        "std::path::PathBuf::from({})",
        rust_string_literal_str(&rel)
    )
}

fn emit_embedded_sources(
    out: &mut String,
    templates: &[PathBuf],
    assets: &[PathBuf],
    manifest_dir: &Path,
) {
    out.push_str("#[allow(dead_code)]\nfn __gluon_source_map() -> std::sync::Arc<gluon::__private::jsxrs::SourceMap> {\n");
    out.push_str("    static SOURCE_MAP: std::sync::LazyLock<std::sync::Arc<gluon::__private::jsxrs::SourceMap>> = std::sync::LazyLock::new(|| std::sync::Arc::new(gluon::__private::jsxrs::SourceMap::new(vec![\n");
    for path in templates {
        let rel = path
            .strip_prefix(manifest_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        let key = rust_string_literal_str(&rel);
        let include_path = rust_string_literal_str(&format!("/{rel}"));
        let _ = writeln!(
            out,
            "        ({key}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), {include_path}))),"
        );
    }
    out.push_str("    ])));\n    std::sync::Arc::clone(&SOURCE_MAP)\n}\n");

    out.push_str("#[allow(dead_code)]\nstatic __GLUON_PUBLIC_ASSETS: &[(&'static str, &'static [u8])] = &[\n");
    for path in assets {
        let rel = path
            .strip_prefix(manifest_dir.join("public"))
            .unwrap_or(path)
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        let key = rust_string_literal_str(&rel);
        let include_path = rust_string_literal_str(&format!("/public/{rel}"));
        let _ = writeln!(
            out,
            "    ({key}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), {include_path}))),"
        );
    }
    out.push_str("];\n");
    out.push_str("#[allow(dead_code)]\nfn __gluon_public_assets() -> &'static [(&'static str, &'static [u8])] { __GLUON_PUBLIC_ASSETS }\n\n");
}

/// Find existing `layout.tsx` files from the app root through the page directory.
fn page_layout_paths(page_path: &Path, app_dir: &Path) -> Vec<PathBuf> {
    let mut layouts = Vec::new();
    let mut current = page_path.parent();
    while let Some(directory) = current {
        if !directory.starts_with(app_dir) {
            break;
        }
        let layout = directory.join("layout.tsx");
        if layout.is_file() {
            layouts.push(layout);
        }
        if directory == app_dir {
            break;
        }
        current = directory.parent();
    }
    layouts.reverse();
    layouts
}

/// Compute the URL path for a directory segment list.
///
/// Rules:
/// - `[id]` -> `{id}`
/// - `[...slug]` -> `{*slug}`
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
            return format!("{{*{rest}}}");
        }
        return format!("{{{inner}}}");
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
            "/users/{id}"
        );
    }

    #[test]
    fn url_path_catch_all() {
        assert_eq!(
            url_path_for(&["files".to_owned(), "[...path]".to_owned()]),
            "/files/{*path}"
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
    fn scan_parse_error_includes_source_file_path() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "users/route.rs", "pub async fn get( {\n");
        let error = scan_app_dir(&app).unwrap_err().to_string();
        let normalized_error = error.replace('\\', "/");
        assert!(normalized_error.contains("/users/route.rs"), "{error}");
    }
    #[test]
    fn scan_finds_simple_page() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "page.rs", "pub async fn get() {}\n");
        let entries = scan(&app).unwrap();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.dir_segments, [] as [std::string::String; 0]);
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
    fn extracts_handler_with_sse_return_type() {
        let parsed: syn::File = syn::parse_str(
            "pub async fn get() -> gluon::Result<gluon::sse::Sse<impl futures_util::Stream<Item = gluon::Result<gluon::sse::Event>>>> { todo!() }",
        )
        .unwrap();
        assert_eq!(extract_http_methods(&parsed), vec!["get".to_owned()]);
    }

    #[test]
    fn ignores_unsupported_method_names() {
        let parsed: syn::File =
            syn::parse_str("pub async fn handle() {} pub async fn foo() {}").unwrap();
        assert_eq!(
            extract_http_methods(&parsed),
            [] as [std::string::String; 0]
        );
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
        assert_eq!(
            extract_http_methods(&parsed),
            [] as [std::string::String; 0]
        );
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
        let err = BuildError::Syn {
            path: Some(PathBuf::from("app/users/route.rs")),
            source: syn_err,
        };
        let s = err.to_string();
        assert!(s.contains("gluon-build parse error"), "s: {s}");
        assert!(s.contains("app/users/route.rs"), "s: {s}");
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
        assert!(matches!(syn_err, BuildError::Syn { path: None, .. }));
    }

    // ---------- generation ----------

    #[test]
    fn generate_without_app_or_public_tree_omits_public_route() {
        let tmp = tempfile::tempdir().unwrap();
        let s = generate(&[], tmp.path());
        assert!(s.contains("pub fn __gluon_router"), "s: {s}");
        assert!(s.contains("axum::Router::new()"), "s: {s}");
        assert!(s.contains("static __GLUON_PUBLIC_ASSETS"), "s: {s}");
        assert!(!s.contains("include_bytes!("), "unexpected assets: {s}");
        assert!(!s.contains("router.nest_service(\"/public\""), "{s}");
    }

    #[test]
    fn generate_mounts_empty_public_directory_for_dev_assets() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("public")).unwrap();
        let generated = generate(&[], tmp.path());
        assert!(
            generated
                .contains("gluon::middleware::static_files::service(source_root.join(\"public\"))"),
            "{generated}"
        );
    }

    #[test]
    fn generate_embeds_public_assets_without_an_app_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        let public = tmp.path().join("public");
        write_file(&public, "marker.txt", "available without app routes\n");
        let entries = scan(&app).unwrap();
        let assets = scan_files(&public, &[]).unwrap();
        let generated = generate_with_embedded(&entries, tmp.path(), &[], &assets, None, None);
        assert!(
            generated.contains(r#"("marker.txt", include_bytes!"#),
            "{generated}"
        );
        assert!(
            generated.contains(
                "gluon::middleware::static_files::embedded_service(__gluon_public_assets())"
            ),
            "{generated}"
        );
    }

    #[test]
    fn tailwind_includes_fragment_only_route_template() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        let out = tmp.path().join("out");
        std::fs::create_dir(&out).unwrap();
        write_file(&app, "api/route.rs", "pub async fn get() {}\n");
        write_file(
            &app,
            "api/route.tsx",
            r#"<div class="sr-only">fragment</div>"#,
        );
        let templates = scan_files(&app, TEMPLATE_EXTENSIONS).unwrap();
        let (stylesheet, _) = build_tailwind_stylesheet(
            &templates,
            tmp.path(),
            &out,
            Some("[view]\ntailwind = true\n"),
        )
        .unwrap();
        let stylesheet = stylesheet.unwrap();
        let css = std::fs::read_to_string(out.join(&stylesheet.file_name)).unwrap();
        assert!(css.contains(".sr-only"), "{css}");
        let entries = scan(&app).unwrap();
        let generated = generate_with_embedded(
            &entries,
            tmp.path(),
            &templates,
            &[],
            Some("[view]\ntailwind = true\n"),
            Some(&stylesheet),
        );
        assert!(
            generated.contains(
                "render_route_request(req, next, template, source_map, source_root, __GLUON_TAILWIND_STYLESHEET)"
            ),
            "{generated}"
        );
    }

    #[test]
    fn tailwind_includes_configured_glob_content_and_tracks_it() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("out");
        std::fs::create_dir(&out).unwrap();
        write_file(
            tmp.path(),
            "content/shared/page.html",
            r#"<div class="text-purple-700">shared</div>"#,
        );
        let config = "[view]\ntailwind = true\ntailwind_content = [\"content/**/*.html\"]\n";
        let (stylesheet, rerun_paths) =
            build_tailwind_stylesheet(&[], tmp.path(), &out, Some(config)).unwrap();
        let stylesheet = stylesheet.unwrap();
        let css = std::fs::read_to_string(out.join(&stylesheet.file_name)).unwrap();
        assert!(css.contains(".text-purple-700"), "{css}");
        assert!(!rerun_paths.contains(&tmp.path().join("content/**/*.html")));
        assert!(rerun_paths.contains(&tmp.path().join("content")));
        assert!(rerun_paths.contains(&tmp.path().join("content/shared/page.html")));
        let generated =
            generate_with_embedded(&[], tmp.path(), &[], &[], Some(config), Some(&stylesheet));
        assert!(
            generated.contains("source_root.clone(), &[\"content/**/*.html\"]"),
            "{generated}"
        );
    }
    #[test]
    fn tailwind_content_globs_do_not_watch_the_project_root() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("out");
        std::fs::create_dir(&out).unwrap();

        for pattern in ["**/*.html", "web/src/**/*.ts"] {
            let config = format!("[view]\ntailwind = true\ntailwind_content = [\"{pattern}\"]\n");
            let error =
                build_tailwind_stylesheet(&[], tmp.path(), &out, Some(&config)).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("must have an existing directory below"),
                "{error}"
            );
        }
    }

    #[test]
    fn tailwind_route_uses_cache_busted_css_asset() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        let out = tmp.path().join("out");
        std::fs::create_dir(&out).unwrap();
        write_file(&app, "page.tsx", r#"<div class="text-blue-700">page</div>"#);
        write_file(&app, "page.rs", "pub async fn get() {}\n");
        let templates = scan_files(&app, TEMPLATE_EXTENSIONS).unwrap();
        let entries = scan(&app).unwrap();
        let config = "[view]\ntailwind = true\n";
        let (stylesheet, _) =
            build_tailwind_stylesheet(&templates, tmp.path(), &out, Some(config)).unwrap();
        let stylesheet = stylesheet.unwrap();
        let css = std::fs::read(out.join(&stylesheet.file_name)).unwrap();
        let hash = sha256_hex(&css);
        assert_eq!(stylesheet.file_name, format!("app.{hash}.css"));
        assert_eq!(stylesheet.url, format!("/_gluon/app.{hash}.css"));

        let generated = generate_with_embedded(
            &entries,
            tmp.path(),
            &templates,
            &[],
            Some(config),
            Some(&stylesheet),
        );
        assert!(
            generated.contains(&format!(
                "pub const __GLUON_TAILWIND_STYLESHEET: Option<&'static str> = Some(\"{}\");",
                stylesheet.url
            )),
            "{generated}"
        );
        assert!(
            generated.contains(&format!(
                "router = router.route(\"{}\", axum::routing::get(",
                stylesheet.url
            )),
            "{generated}"
        );
        assert!(
            generated.contains(&format!(
                "include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{}\"))",
                stylesheet.file_name
            )),
            "{generated}"
        );
        assert!(
            generated.contains("gluon::middleware::static_files::serve_tailwind_css"),
            "{generated}"
        );
        assert!(
            generated.contains("source_root.clone(), &[]"),
            "{generated}"
        );
        assert!(
            generated.contains("stylesheet_url: __GLUON_TAILWIND_STYLESHEET"),
            "{generated}"
        );
    }

    #[test]
    fn tailwind_off_emits_no_asset_or_route() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        let out = tmp.path().join("out");
        std::fs::create_dir(&out).unwrap();
        write_file(&app, "page.tsx", r#"<div class="text-blue-700">page</div>"#);
        let templates = scan_files(&app, TEMPLATE_EXTENSIONS).unwrap();
        let config = "[view]\ntailwind = false\ntailwind_content = [\"content/**/*.html\"]\n";
        let (stylesheet, rerun_paths) =
            build_tailwind_stylesheet(&templates, tmp.path(), &out, Some(config)).unwrap();
        assert!(stylesheet.is_none());
        assert_eq!(rerun_paths, Vec::<PathBuf>::new());
        assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0);

        let generated =
            generate_with_embedded(&[], tmp.path(), &templates, &[], Some(config), None);
        assert!(
            generated
                .contains("pub const __GLUON_TAILWIND_STYLESHEET: Option<&'static str> = None;")
        );
        assert!(!generated.contains("serve_tailwind_css"), "{generated}");
    }

    #[test]
    fn generated_output_is_only_written_when_changed() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("__gluon_app.rs");
        assert!(write_generated_if_changed(&output, "generated source").unwrap());
        assert!(!write_generated_if_changed(&output, "generated source").unwrap());
        assert!(write_generated_if_changed(&output, "updated source").unwrap());
        assert_eq!(std::fs::read_to_string(output).unwrap(), "updated source");
    }

    #[test]
    fn generate_includes_router_call_for_entry() {
        let entry = make_entry(&[], "page", "/tmp/page.rs");
        let s = generate(std::slice::from_ref(&entry), Path::new("/tmp"));
        assert!(s.contains("axum::routing::get"), "s: {s}");
        assert!(s.contains("\"/\""), "s: {s}");
    }

    #[test]
    fn generate_emits_page_render_middleware() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "page.rs", "pub async fn get() {}\n");
        write_file(&app, "page.tsx", "export default function Page() {}\n");
        let entries = scan(&app).unwrap();
        let s = generate(&entries, tmp.path());
        assert!(s.contains("gluon::view::render_page_request"), "s: {s}");
        assert!(s.contains("let template = Some("), "s: {s}");
    }

    #[test]
    fn generate_scopes_route_handlers_with_sibling_templates() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "api/route.rs", "pub async fn get() {}\n");
        write_file(
            &app,
            "api/route.tsx",
            "export default function Route() {}\n",
        );
        let entries = scan(&app).unwrap();
        let templates = scan_files(&app, TEMPLATE_EXTENSIONS).unwrap();
        let generated = generate_with_embedded(&entries, tmp.path(), &templates, &[], None, None);

        assert!(
            generated.contains("gluon::view::render_route_request"),
            "{generated}"
        );
        assert!(
            generated.contains(r#"("app/api/route.tsx", include_str!"#),
            "{generated}"
        );
        assert!(
            !generated.contains("render_page_request"),
            "route templates must not use page error/layout middleware: {generated}"
        );
    }

    #[test]
    fn generate_emits_compile_time_sources_and_assets() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        let public = tmp.path().join("public");
        write_file(&app, "page.tsx", "export default function Page() {}\n");
        write_file(
            &app,
            "components/card.tsx",
            "export default function Card() {}\n",
        );
        write_file(
            &app,
            "_error/404.tsx",
            "export default function NotFound() {}\n",
        );
        write_file(
            &app,
            "api/route.tsx",
            "export default function Route() {}\n",
        );
        write_file(&public, "styles/site.css", "body { color: red; }\n");
        write_file(&public, "marker.txt", "embedded marker\n");
        write_file(
            &app,
            "components/card.jsx",
            "export default function Card() {}\n",
        );
        write_file(
            &app,
            "components/index.jsx",
            "export default function Index() {}\n",
        );
        write_file(&app, "components/legacy.ts", "export const value = 1;\n");
        write_file(&app, "components/legacy.js", "export const value = 1;\n");
        let templates = scan_files(&app, TEMPLATE_EXTENSIONS).unwrap();
        let assets = scan_files(&public, &[]).unwrap();
        let entries = scan(&app).unwrap();
        let generated =
            generate_with_embedded(&entries, tmp.path(), &templates, &assets, None, None);
        assert_eq!(generated.matches("include_str!(").count(), 8, "{generated}");
        assert_eq!(
            generated.matches("include_bytes!(").count(),
            2,
            "{generated}"
        );
        assert!(generated.contains(r#"("app/components/card.tsx", include_str!"#));
        assert!(generated.contains(r#"("app/api/route.tsx", include_str!"#));
        assert!(generated.contains(r#"("app/components/card.jsx", include_str!"#));
        assert!(generated.contains(r#"("app/components/index.jsx", include_str!"#));
        assert!(generated.contains(r#"("app/components/legacy.ts", include_str!"#));
        assert!(generated.contains(r#"("app/components/legacy.js", include_str!"#));
        assert!(generated.contains(r#"("styles/site.css", include_bytes!"#));
        assert!(generated.contains("LazyLock<std::sync::Arc<gluon::__private::jsxrs::SourceMap>>"));
    }

    #[cfg(unix)]
    #[test]
    fn scan_files_embeds_internal_symlinks() {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().unwrap();
        let public = tmp.path().join("public");
        write_file(&public, "original.txt", "embedded\n");
        symlink(public.join("original.txt"), public.join("alias.txt")).unwrap();

        let files = scan_files(&public, &[]).unwrap();
        assert!(files.contains(&public.join("alias.txt")));
    }

    #[cfg(unix)]
    #[test]
    fn scan_files_rejects_symlinks_outside_source_tree() {
        use std::os::unix::fs::symlink;

        let tmp = tempfile::tempdir().unwrap();
        let public = tmp.path().join("public");
        write_file(tmp.path(), "private.txt", "not public\n");
        std::fs::create_dir(&public).unwrap();
        symlink(tmp.path().join("private.txt"), public.join("private.txt")).unwrap();

        let error = scan_files(&public, &[]).unwrap_err();
        assert!(error.to_string().contains("resolves outside"));
    }

    #[test]
    fn generate_emits_concat_env_for_tsx() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "page.rs", "pub async fn get() {}\n");
        write_file(&app, "page.tsx", "export default function Page() {}\n");
        let entries = scan(&app).unwrap();
        let s = generate(&entries, tmp.path());
        // The router function must use env!("CARGO_MANIFEST_DIR") for the TSX
        // template path so the binary is not tied to the build machine's absolute
        // path. The mod tree #[path] attributes still use absolute paths (those
        // are compile-time only and are always evaluated on the build machine).
        let router_start = s.find("pub fn __gluon_router").unwrap_or(s.len());
        let router_section = &s[router_start..];
        assert!(
            router_section.contains(r#"env!("CARGO_MANIFEST_DIR")"#),
            "router should pass its source root to the renderer; s: {s}"
        );
        assert!(
            !router_section.contains(tmp.path().to_string_lossy().as_ref()),
            "router must not embed absolute build-machine paths; s: {s}"
        );
    }

    #[test]
    fn generate_collects_nested_and_route_group_layouts() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "layout.tsx", "export default function Root() {}\n");
        write_file(
            &app,
            "(marketing)/layout.tsx",
            "export default function Marketing() {}\n",
        );
        write_file(
            &app,
            "(marketing)/about/layout.tsx",
            "export default function About() {}\n",
        );
        write_file(&app, "(marketing)/about/page.rs", "pub async fn get() {}\n");
        let entries = scan(&app).unwrap();
        let s = generate(&entries, tmp.path());
        let root = s.find("app/layout.tsx").unwrap();
        let group = s.find("app/(marketing)/layout.tsx").unwrap();
        let nested = s.find("app/(marketing)/about/layout.tsx").unwrap();
        assert!(root < group && group < nested, "{s}");
        assert!(s.contains(r#"route("/about""#), "{s}");
        assert!(s.contains("app/_error"), "{s}");
        assert!(s.contains("root_layout = Some("), "{s}");
        assert!(s.contains("let template = None;"), "{s}");
    }

    #[test]
    fn generate_uses_route_template_middleware_without_page_middleware() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "api/route.rs", "pub async fn get() {}\n");
        write_file(&app, "api/route.tsx", "export default function R() {}\n");
        let entries = scan(&app).unwrap();
        let generated = generate(&entries, tmp.path());
        assert!(
            generated.contains("gluon::view::render_route_request"),
            "route with a sibling TSX template must get the renderer: {generated}",
        );
        assert!(
            !generated.contains("render_page_request"),
            "route TSX must not get page layouts or error rendering: {generated}",
        );
        assert!(
            generated.contains("let source_map = __gluon_source_map();"),
            "route template middleware needs the embedded source map: {generated}",
        );
    }

    #[test]
    fn generate_omits_template_layer_for_route_without_tsx() {
        let tmp = tempfile::tempdir().unwrap();
        let app = tmp.path().join("app");
        write_file(&app, "api/route.rs", "pub async fn get() {}\n");
        let entries = scan(&app).unwrap();
        let generated = generate(&entries, tmp.path());
        assert!(
            !generated.contains("render_route_request"),
            "route without a sibling TSX template must not get the renderer: {generated}"
        );
        assert!(
            !generated.contains("CURRENT_TEMPLATE.scope"),
            "route without a sibling TSX template must not get a template layer: {generated}"
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
            "/users/{id}"
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
        // "[]" -> inner "" -> "{}" (current behavior, pinned by this test)
        assert_eq!(transform_url_segment("[]"), "{}");
    }

    #[test]
    fn transform_url_empty_catch_all() {
        // "[...]" -> inner "..." -> strip_prefix("...") = Some("") -> "{*}"
        assert_eq!(transform_url_segment("[...]"), "{*}");
    }

    // ---------- emit_router_fn ----------

    /// The generated router must use axum 0.8 path syntax (`{id}`, `{*slug}`);
    /// axum rejects the old 0.7 syntax (`:id`, `*slug`) at router construction,
    /// so a wrong emission only surfaces as a boot-time panic in user apps.
    #[test]
    fn emit_router_fn_uses_axum08_path_syntax() {
        let entries = vec![
            make_entry(&["users", "[id]"], "page", "/tmp/users/[id]/page.rs"),
            make_entry(
                &["files", "[...path]"],
                "route",
                "/tmp/files/[...path]/route.rs",
            ),
        ];
        let mut out = String::new();
        emit_router_fn(
            &mut out,
            &entries,
            Path::new("/tmp/proj"),
            None,
            false,
            None,
        );
        assert!(out.contains(r#"route("/users/{id}","#), "out: {out}");
        assert!(
            out.contains(r#"route("/files/{*path}","#),
            "catch-all must emit {{*name}} - out: {out}"
        );
        assert!(!out.contains(r#"":id""#), "out: {out}");
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
        assert!(!generated.contains("gluon::__register_embedded_config"));
    }

    #[test]
    #[serial]
    fn run_embeds_gluon_config_without_app_dir() {
        let manifest = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        std::fs::write(
            manifest.path().join("gluon.toml"),
            "[app]\nname = \"embedded\"\n[view]\ntailwind = true\n",
        )
        .unwrap();
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
        assert!(generated.contains("gluon::__register_embedded_config"));
        assert!(generated.contains("name = \\\"embedded\\\""));
        assert!(generated.contains("tailwind = true"));
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
