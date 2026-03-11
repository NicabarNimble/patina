use anyhow::{bail, Result};
use std::path::Path;

use patina::interface::{self, adapter as load_adapter};
use patina::project;

use crate::commands::launch::internal as launch_internal;

pub fn setup(interface_name: &str, path: Option<String>, force: bool) -> Result<()> {
    let project_path = launch_internal::resolve_project_path(path.as_deref())?;
    let (adapter, bootstrap) = ensure_interface_ready(interface_name, &project_path, force)?;

    println!(
        "Patina interface setup refreshed {}",
        adapter.display_name()
    );
    println!("  Context: {}", bootstrap.context_path.display());
    println!("  Bootstrap: {}", bootstrap.bootstrap_path.display());
    if let Some(backup_snapshot) = &bootstrap.backup_snapshot {
        println!("  Backup: {}", backup_snapshot.display());
    }

    Ok(())
}

pub fn ensure_interface_ready(
    interface_name: &str,
    project_path: &Path,
    force: bool,
) -> Result<(
    Box<dyn patina::interface::AiAdapter>,
    interface::BootstrapResult,
)> {
    ensure_interface_allowed(project_path, interface_name)?;

    let adapter = load_adapter(interface_name).map_err(|_| {
        anyhow::anyhow!(
            "Interface '{}' does not use the native Patina interface setup path.\nUse the Claude compatibility flow for Claude, or choose one of: opencode, gemini.",
            interface_name
        )
    })?;
    let bootstrap = if force {
        interface::ensure_adapter_projection(
            interface_name,
            project_path,
            interface::ProjectionMode::ForceRewrite,
        )?
    } else {
        adapter.bootstrap(project_path)?
    };
    Ok((adapter, bootstrap))
}

fn ensure_interface_allowed(project_path: &Path, interface_name: &str) -> Result<()> {
    let config = project::load_with_migration(project_path)?;
    if config
        .adapters
        .allowed
        .iter()
        .any(|allowed| allowed == interface_name)
    {
        return Ok(());
    }

    bail!(
        "Interface '{}' is not allowed for this project.\nAllow it first with: patina adapter add {}",
        interface_name,
        interface_name
    );
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn setup_project(adapter: &str) -> TempDir {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join(".patina")).unwrap();
        fs::write(temp.path().join(".patina/uid"), "test-project\n").unwrap();
        fs::write(
            temp.path().join(".patina/config.toml"),
            format!(
                "[adapters]\nallowed = [\"{}\"]\ndefault = \"{}\"\n",
                adapter, adapter
            ),
        )
        .unwrap();
        temp
    }

    fn with_temp_patina_home<T>(temp: &TempDir, f: impl FnOnce() -> T) -> T {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let patina_home = temp.path().join("patina-home");
        fs::create_dir_all(&patina_home).unwrap();

        let old = std::env::var_os("PATINA_HOME");
        unsafe {
            std::env::set_var("PATINA_HOME", &patina_home);
        }
        let result = f();
        match old {
            Some(value) => unsafe {
                std::env::set_var("PATINA_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("PATINA_HOME");
            },
        }
        result
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
        assert!(temp
            .path()
            .join(".opencode/commands/session-start.md")
            .exists());
    }
}
