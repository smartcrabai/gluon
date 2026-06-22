use std::process::Command;

pub fn build() -> anyhow::Result<()> {
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()?;
    if !status.success() {
        anyhow::bail!("cargo build failed");
    }
    Ok(())
}

pub fn run(release: bool) -> anyhow::Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("run");
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("cargo run failed");
    }
    Ok(())
}
