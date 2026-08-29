use std::process::Command;

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn main() {
    let head_path = git_output(&["rev-parse", "--git-path", "HEAD"]);
    let ref_path = git_output(&["symbolic-ref", "-q", "HEAD"])
        .and_then(|head_ref| git_output(&["rev-parse", "--git-path", &head_ref]));
    for path in [head_path, ref_path].into_iter().flatten() {
        println!("cargo:rerun-if-changed={path}");
    }

    let Some(revision) = git_output(&["rev-parse", "HEAD"]) else {
        return;
    };
    if revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        println!("cargo:rustc-env=GLUON_GIT_REV={revision}");
    }
}
