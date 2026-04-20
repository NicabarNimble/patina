use anyhow::Result;
use std::path::Path;

use patina::interface::{self, interface as load_interface};

pub fn ensure_interface_ready(
    interface_name: &str,
    project_path: &Path,
    force: bool,
) -> Result<(
    Box<dyn patina::interface::AiInterface>,
    interface::BootstrapResult,
)> {
    interface::ensure_ai_project_config(project_path, None)?;

    let iface = load_interface(interface_name).map_err(|_| {
        anyhow::anyhow!(
            "Unsupported Patina AI interface '{}'. Choose one of: {}.",
            interface_name,
            interface::supported_ai_interfaces().join(", ")
        )
    })?;
    let bootstrap = interface::prepare_ai_bundle(project_path, interface_name, force)?.bootstrap;
    Ok((iface, bootstrap))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    fn setup_project(interface_name: &str) -> TempDir {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join(".patina")).unwrap();
        fs::write(temp.path().join(".patina/uid"), "test-project\n").unwrap();
        fs::write(
            temp.path().join(".patina/config.toml"),
            format!(
                "[interfaces]\nallowed = [\"{}\"]\ndefault = \"{}\"\n",
                interface_name, interface_name
            ),
        )
        .unwrap();
        temp
    }

    fn with_test_env<T>(temp: &TempDir, f: impl FnOnce() -> T) -> T {
        let old_dir = std::env::current_dir().ok();
        std::env::set_current_dir(temp.path()).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::test_support::with_temp_patina_home(|_| f())
        }));
        if let Some(path) = old_dir {
            let _ = std::env::set_current_dir(path);
        }
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    fn setup_creates_projection_for_opencode() {
        let temp = setup_project("opencode");

        with_test_env(&temp, || {
            crate::commands::ai::surface::setup(crate::commands::ai::surface::AiSetupRequest {
                interface: None,
                path: Some(temp.path().display().to_string()),
                force: false,
                all: true,
            })
            .unwrap();
        });

        assert!(temp.path().join("AGENTS.md").exists());
        assert!(temp.path().join("CLAUDE.md").exists());
        assert!(temp.path().join("GEMINI.md").exists());
        assert!(temp
            .path()
            .join(".claude/commands/session-start.md")
            .exists());
        assert!(temp
            .path()
            .join(".opencode/commands/session-start.md")
            .exists());
        assert!(temp
            .path()
            .join(".gemini/commands/session-start.toml")
            .exists());
        assert!(temp.path().join(".pi/commands/session-start.md").exists());
    }
}
