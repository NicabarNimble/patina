use anyhow::Result;
use std::path::Path;

use patina::interface::{self, interface as load_interface};

use crate::commands::launch::internal as launch_internal;

pub fn setup(interface_name: &str, path: Option<String>, force: bool) -> Result<()> {
    let project_path = launch_internal::resolve_project_path(path.as_deref())?;
    if !interface::is_supported_ai_interface(interface_name) {
        anyhow::bail!(
            "Interface '{}' is not part of the Patina AI surface. Choose one of: {}.",
            interface_name,
            interface::supported_ai_interfaces().join(", ")
        );
    }

    crate::commands::ai::surface::setup(crate::commands::ai::surface::AiSetupRequest {
        interface: None,
        path: Some(project_path.display().to_string()),
        force,
    })
}

pub fn ensure_interface_ready(
    interface_name: &str,
    project_path: &Path,
    force: bool,
) -> Result<(
    Box<dyn patina::interface::AiInterface>,
    interface::BootstrapResult,
)> {
    interface::ensure_ai_project_config(project_path, None)?;

    let adapter = load_interface(interface_name).map_err(|_| {
        anyhow::anyhow!(
            "Unsupported Patina AI interface '{}'. Choose one of: {}.",
            interface_name,
            interface::supported_ai_interfaces().join(", ")
        )
    })?;
    let bootstrap = interface::prepare_ai_bundle(project_path, interface_name, force)?.bootstrap;
    Ok((adapter, bootstrap))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;

    fn setup_project(adapter: &str) -> TempDir {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join(".patina")).unwrap();
        fs::write(temp.path().join(".patina/uid"), "test-project\n").unwrap();
        fs::write(
            temp.path().join(".patina/config.toml"),
            format!(
                "[interfaces]\nallowed = [\"{}\"]\ndefault = \"{}\"\n",
                adapter, adapter
            ),
        )
        .unwrap();
        temp
    }

    fn with_temp_patina_home<T>(temp: &TempDir, f: impl FnOnce() -> T) -> T {
        let _guard = patina::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let patina_home = temp.path().join("patina-home");
        fs::create_dir_all(&patina_home).unwrap();

        let old_dir = std::env::current_dir().ok();
        let old = std::env::var_os("PATINA_HOME");
        std::env::set_current_dir(temp.path()).unwrap();
        unsafe {
            std::env::set_var("PATINA_HOME", &patina_home);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        if let Some(path) = old_dir {
            let _ = std::env::set_current_dir(path);
        }
        match old {
            Some(value) => unsafe {
                std::env::set_var("PATINA_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("PATINA_HOME");
            },
        }
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    fn setup_creates_projection_for_opencode() {
        let temp = setup_project("opencode");

        with_temp_patina_home(&temp, || {
            crate::commands::interface::execute(
                crate::commands::interface::InterfaceCommands::Setup {
                    name: "opencode".to_string(),
                    path: Some(temp.path().display().to_string()),
                    force: false,
                },
            )
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
    }
}
