//! `gluon new <name>` -- scaffold a new gluon application from embedded
//! templates.
//!
//! The implementation walks every embedded asset whose path starts with
//! `new/`, renders `*.j2` files through minijinja and copies the rest
//! verbatim into the freshly-created project directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use minijinja::context;

use crate::embed::Templates;
use crate::templating;

/// Prefix used to identify the application scaffold templates inside the
/// embedded bundle.
const SCAFFOLD_PREFIX: &str = "new/";

/// File extension that marks a template that should be processed by minijinja.
/// Files that do not end with this suffix are copied verbatim.
const TEMPLATE_SUFFIX: &str = ".j2";

/// Run the `gluon new` subcommand.
///
/// 1. Create `<name>/` in the current working directory. If it already exists
///    we abort with an error rather than overwriting user data.
/// 2. Expand every embedded asset under `new/` into the new directory,
///    rendering `*.j2` files with the project name as the template context.
/// 3. Optionally run `git init` (skipped if `no_git` is set).
/// 4. Optionally run `cargo fetch` (skipped if `no_install` is set, and
///    treated as best-effort: failures only produce a warning).
///
/// # Errors
///
/// Returns an error when the target directory cannot be created, when an
/// embedded asset cannot be rendered, or when writing a generated file
/// fails.
pub fn run(name: &str, no_git: bool, no_install: bool) -> Result<()> {
    validate_project_name(name)?;

    let project_dir = PathBuf::from(name);
    if project_dir.exists() {
        bail!(
            "destination `{}` already exists; refusing to overwrite",
            project_dir.display()
        );
    }
    fs::create_dir_all(&project_dir).with_context(|| {
        format!(
            "failed to create project directory: {}",
            project_dir.display()
        )
    })?;

    expand_scaffold(name, &project_dir)?;

    if !no_git {
        run_git_init(&project_dir);
    }

    if !no_install {
        run_cargo_fetch(&project_dir);
    }

    println!("created new gluon application in {}", project_dir.display());
    Ok(())
}

/// Reject obviously invalid project names early. The set of valid Cargo
/// package names is broader than this, but we only need to filter out the
/// names that would let a user escape the destination directory or produce
/// a confusing tree.
fn validate_project_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("project name must not be empty");
    }
    if name.contains('/') || name.contains('\\') {
        bail!("project name must not contain path separators: {name}");
    }
    if name == "." || name == ".." {
        bail!("project name must not be `.` or `..`");
    }
    Ok(())
}

/// Iterate over every embedded asset under `SCAFFOLD_PREFIX` and write the
/// rendered (or verbatim) output into `project_dir`.
fn expand_scaffold(name: &str, project_dir: &Path) -> Result<()> {
    let ctx = context! { name => name };

    let mut entries: Vec<String> = Templates::iter()
        .map(std::borrow::Cow::into_owned)
        .filter(|path| path.starts_with(SCAFFOLD_PREFIX))
        .collect();
    entries.sort();

    if entries.is_empty() {
        return Err(anyhow!(
            "no scaffold templates found under `{SCAFFOLD_PREFIX}` in the embedded bundle"
        ));
    }

    for template_path in entries {
        let relative = template_path
            .strip_prefix(SCAFFOLD_PREFIX)
            .ok_or_else(|| anyhow!("unexpected template path: {template_path}"))?;

        // `.gitkeep` is the convention for "preserve this empty directory in
        // git"; we keep its on-disk name as-is even when it lives inside the
        // scaffold tree.
        let (output_relative, is_template) =
            if let Some(stripped) = relative.strip_suffix(TEMPLATE_SUFFIX) {
                (stripped.to_owned(), true)
            } else {
                (relative.to_owned(), false)
            };

        let output_path = project_dir.join(&output_relative);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
        }

        if is_template {
            let rendered = templating::render(&template_path, &ctx)
                .with_context(|| format!("failed to render template: {template_path}"))?;
            fs::write(&output_path, rendered)
                .with_context(|| format!("failed to write file: {}", output_path.display()))?;
        } else {
            let bytes = templating::read_bytes(&template_path)
                .with_context(|| format!("failed to read embedded asset: {template_path}"))?;
            fs::write(&output_path, bytes)
                .with_context(|| format!("failed to write file: {}", output_path.display()))?;
        }
    }

    Ok(())
}

/// Best-effort `git init` inside the freshly-created project directory.
///
/// We deliberately do not fail the whole command if `git` is missing or
/// refuses to run -- the scaffold itself is already on disk and useful.
fn run_git_init(project_dir: &Path) {
    let status = Command::new("git")
        .arg("init")
        .arg("--quiet")
        .current_dir(project_dir)
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("warning: `git init` exited with status {s}"),
        Err(e) => eprintln!("warning: failed to run `git init`: {e}"),
    }
}

/// Best-effort `cargo fetch` inside the freshly-created project directory.
fn run_cargo_fetch(project_dir: &Path) {
    let status = Command::new("cargo")
        .arg("fetch")
        .current_dir(project_dir)
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("warning: `cargo fetch` exited with status {s}"),
        Err(e) => eprintln!("warning: failed to run `cargo fetch`: {e}"),
    }
}
