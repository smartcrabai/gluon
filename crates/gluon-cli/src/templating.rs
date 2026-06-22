use anyhow::{Context, Result, anyhow};
use minijinja::Environment;
use serde::Serialize;

use crate::embed::Templates;

/// Build a minijinja `Environment` and register a single template loaded from
/// the embedded `Templates` asset bundle.
///
/// The template is registered under its original `template_path` name so that
/// error messages produced by minijinja refer to the template by its on-disk
/// path inside the `templates/` directory.
fn load_environment(template_path: &str) -> Result<Environment<'static>> {
    let asset = Templates::get(template_path)
        .ok_or_else(|| anyhow!("embedded template not found: {template_path}"))?;
    let source = std::str::from_utf8(asset.data.as_ref())
        .with_context(|| format!("embedded template is not valid UTF-8: {template_path}"))?
        .to_owned();

    let mut env = Environment::new();
    env.add_template_owned(template_path.to_owned(), source)
        .with_context(|| format!("failed to parse template: {template_path}"))?;
    Ok(env)
}

/// Render an embedded template by its path (relative to `templates/`) with the
/// given serializable context.
///
/// # Errors
///
/// Returns an error if the template does not exist in the embedded bundle, is
/// not valid UTF-8, fails to parse, or fails to render.
pub fn render<S: Serialize>(template_path: &str, ctx: S) -> Result<String> {
    let env = load_environment(template_path)?;
    let tmpl = env
        .get_template(template_path)
        .with_context(|| format!("failed to get template: {template_path}"))?;
    tmpl.render(ctx)
        .with_context(|| format!("failed to render template: {template_path}"))
}

/// Read an embedded asset as raw bytes.
///
/// Useful for files that should be copied verbatim (binary assets, files that
/// happen to contain template-like syntax that must not be expanded, etc.).
///
/// # Errors
///
/// Returns an error if the asset does not exist in the embedded bundle.
pub fn read_bytes(template_path: &str) -> Result<Vec<u8>> {
    let asset = Templates::get(template_path)
        .ok_or_else(|| anyhow!("embedded template not found: {template_path}"))?;
    Ok(asset.data.into_owned())
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn render_existing_template_succeeds() {
        // `new/app/page.tsx` is shipped verbatim and contains the literal
        // `export default function` near the top.
        let out = render("new/app/page.tsx", serde_json::json!({})).unwrap();
        assert!(
            out.contains("export default function"),
            "unexpected output: {out}"
        );
    }

    #[test]
    fn render_unknown_template_errors() {
        let err = render("nonexistent.j2", serde_json::json!({})).unwrap_err();
        assert!(
            err.to_string().contains("embedded template not found"),
            "unexpected error: {err}"
        );
    }
}
