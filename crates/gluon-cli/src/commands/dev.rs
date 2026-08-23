use anyhow::Context;
use notify::{Event, RecursiveMode, Watcher};
use std::path::Path;
use std::process::{Child, Command};
use std::sync::mpsc;
use std::time::Duration;

pub fn run() -> anyhow::Result<()> {
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx).context("create watcher")?;

    for dir in ["app", "src", "migrations"] {
        let path = Path::new(dir);
        if path.exists() {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .context("watch dir")?;
        }
    }

    let mut child: Option<Child> = Some(spawn_app()?);

    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Ok(event)) => {
                if should_restart(&event) {
                    if let Some(mut c) = child.take() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                    match spawn_app() {
                        Ok(c) => child = Some(c),
                        Err(e) => eprintln!("restart failed: {e}"),
                    }
                }
            }
            Ok(Err(e)) => eprintln!("watch error: {e}"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if let Some(mut c) = child.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                break;
            }
        }
    }
    Ok(())
}

fn spawn_app() -> anyhow::Result<Child> {
    eprintln!("[gluon dev] cargo run");
    Command::new("cargo")
        .arg("run")
        .env(
            "GLUON_INSECURE_COOKIE",
            std::env::var_os("GLUON_INSECURE_COOKIE").unwrap_or_else(|| "1".into()),
        )
        .spawn()
        .context("spawn cargo run")
}

pub(crate) fn should_restart(event: &Event) -> bool {
    event.paths.iter().any(|path| {
        let normalized = path.to_string_lossy().replace('\\', "/");
        let file_name = normalized.rsplit('/').next().unwrap_or_default();
        let ignored_extension = [".ts", ".tsx", ".j2", ".swp", ".tmp"]
            .iter()
            .any(|extension| file_name.ends_with(extension));
        let editor_backup = file_name.ends_with('~') || file_name.starts_with(".#");
        let target_artifact = normalized.split('/').any(|segment| segment == "target");
        !ignored_extension && !editor_backup && !target_artifact
    })
}

#[cfg(test)]
mod tests {
    use super::should_restart;
    use notify::Event;
    use notify::event::{EventKind, ModifyKind};
    use std::path::PathBuf;

    fn make_event(paths: &[&str]) -> Event {
        Event {
            kind: EventKind::Modify(ModifyKind::Any),
            paths: paths.iter().map(PathBuf::from).collect(),
            attrs: notify::event::EventAttributes::default(),
        }
    }

    #[test]
    fn restart_for_rs() {
        assert!(should_restart(&make_event(&["src/main.rs"])));
    }

    #[test]
    fn no_restart_for_ts() {
        assert!(!should_restart(&make_event(&["app/page.ts"])));
    }

    #[test]
    fn no_restart_for_tsx() {
        assert!(!should_restart(&make_event(&["app/page.tsx"])));
    }

    #[test]
    fn no_restart_in_target() {
        assert!(!should_restart(&make_event(&["/repo/target/debug/foo"])));
        assert!(!should_restart(&make_event(&["target/debug/foo"])));
        assert!(!should_restart(&make_event(&[
            r"C:\repo\target\debug\foo.exe",
        ])));
    }

    #[test]
    fn no_restart_for_editor_temporary_files() {
        for path in [
            "src/main.rs.swp",
            "src/main.rs.tmp",
            "src/main.rs~",
            "src/.#main.rs",
        ] {
            assert!(!should_restart(&make_event(&[path])), "path: {path}");
        }
    }

    #[test]
    fn restart_when_any_path_matches() {
        // Mix of a ts (skip) and an rs (restart) -- `any` should win.
        assert!(should_restart(&make_event(&["app/page.ts", "src/main.rs"])));
    }
}
