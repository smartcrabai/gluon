//! Implementation of `gluon routes`.
//!
//! Reuses [`gluon_build::scan`] so the displayed routes always agree with the
//! ones the build script would compile in.
//!
//! Note: unit-testing `run` is intentionally skipped because it depends on the
//! process CWD and prints to stdout. End-to-end coverage owns this surface.

use std::path::Path;

use anyhow::{Context, bail};

pub fn run() -> anyhow::Result<()> {
    let app_dir = Path::new("app");
    if !app_dir.exists() {
        bail!("app/ directory not found");
    }
    let entries = gluon_build::scan(app_dir).context("failed to scan app/")?;

    let mut routes: Vec<(String, String, String)> = Vec::new();
    for entry in entries {
        let url = entry.url_path();
        let relative = entry
            .abs_path
            .strip_prefix(std::env::current_dir().unwrap_or_default())
            .unwrap_or(&entry.abs_path)
            .display()
            .to_string();
        for method in &entry.methods {
            let display = format!("{relative}::{method}");
            routes.push((method.to_uppercase(), url.clone(), display));
        }
    }
    routes.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    for (method, url, file) in routes {
        println!("{method:<7} {url:<30} {file}");
    }
    Ok(())
}
