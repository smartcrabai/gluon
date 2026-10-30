use anyhow::Context;
use notify::event::{EventKind, ModifyKind};
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
    let config_file = Path::new("gluon.toml");
    if config_file.is_file() {
        watcher
            .watch(config_file, RecursiveMode::NonRecursive)
            .context("watch gluon.toml")?;
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
        .env(
            "GLUON_DEV_TEMPLATES",
            dev_templates_value(std::env::var_os("GLUON_DEV_TEMPLATES")),
        )
        .spawn()
        .context("spawn cargo run")
}

fn dev_templates_value(override_value: Option<std::ffi::OsString>) -> std::ffi::OsString {
    override_value.unwrap_or_else(|| "1".into())
}

pub(crate) fn should_restart(event: &Event) -> bool {
    let generated_template_event = matches!(
        &event.kind,
        EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_))
    );
    event.paths.iter().any(|path| {
        let normalized = path.to_string_lossy().replace('\\', "/");
        let file_name = normalized.rsplit('/').next().unwrap_or_default();
        let ignored_extension = [".ts", ".tsx", ".j2", ".swp", ".tmp"]
            .iter()
            .any(|extension| file_name.ends_with(extension));
        let editor_backup = file_name.ends_with('~') || file_name.starts_with(".#");
        let target_artifact = normalized.split('/').any(|segment| segment == "target");
        let new_or_removed_template = Path::new(file_name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("tsx"))
            && generated_template_event;
        !editor_backup && !target_artifact && (!ignored_extension || new_or_removed_template)
    })
}

#[cfg(test)]
mod tests {
    use super::{dev_templates_value, should_restart};
    use notify::Event;
    use notify::event::{EventKind, ModifyKind};
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn make_event(paths: &[&str]) -> Event {
        make_event_with_kind(paths, EventKind::Modify(ModifyKind::Any))
    }

    fn make_event_with_kind(paths: &[&str], kind: EventKind) -> Event {
        Event {
            kind,
            paths: paths.iter().map(PathBuf::from).collect(),
            attrs: notify::event::EventAttributes::default(),
        }
    }

    #[test]
    fn dev_templates_are_enabled_by_default() {
        assert_eq!(dev_templates_value(None), OsString::from("1"));
    }

    #[test]
    fn dev_templates_preserve_an_explicit_override() {
        assert_eq!(
            dev_templates_value(Some(OsString::from("0"))),
            OsString::from("0")
        );
    }

    #[test]
    fn restart_for_rs() {
        assert!(should_restart(&make_event(&["src/main.rs"])));
    }

    #[test]
    fn restart_for_gluon_toml() {
        assert!(should_restart(&make_event(&["gluon.toml"])));
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
    fn restart_when_tsx_template_is_created_or_removed() {
        for kind in [
            EventKind::Create(notify::event::CreateKind::File),
            EventKind::Remove(notify::event::RemoveKind::File),
        ] {
            assert!(
                should_restart(&make_event_with_kind(&["app/page.tsx"], kind)),
                "structural TSX changes must rebuild generated routes"
            );
        }
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
