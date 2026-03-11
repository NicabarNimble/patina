use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

use crate::adapters::launch::Adapter;
use crate::adapters::{gemini::GeminiAdapter, opencode::OpenCodeAdapter, templates};
use crate::{project, Environment};

#[derive(Debug, Clone)]
pub struct BootstrapResult {
    pub bootstrap_path: PathBuf,
    pub context_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionMode {
    CreateMissing,
    RefreshManaged,
}

pub fn ensure_adapter_bootstrap(
    adapter_name: &str,
    project_root: &Path,
) -> Result<BootstrapResult> {
    ensure_adapter_projection(adapter_name, project_root, ProjectionMode::RefreshManaged)
}

pub fn ensure_adapter_projection(
    adapter_name: &str,
    project_root: &Path,
    mode: ProjectionMode,
) -> Result<BootstrapResult> {
    let project_name = project::load_with_migration(project_root)?.project.name;
    let environment = Environment::detect()?;

    let context_path = match adapter_name {
        "opencode" => {
            sync_managed_templates("opencode", project_root, mode)?;
            OpenCodeAdapter::ensure_context(project_root, &project_name, &environment)?
        }
        "gemini" => {
            sync_managed_templates("gemini", project_root, mode)?;
            GeminiAdapter::ensure_context(project_root, &project_name, &environment)?
        }
        other => bail!("Unsupported ai adapter bootstrap: {}", other),
    };

    crate::adapters::launch::generate_bootstrap(adapter_name, project_root)?;
    let bootstrap_file = Adapter::from_name(adapter_name)
        .ok_or_else(|| anyhow::anyhow!("unknown adapter {}", adapter_name))?
        .bootstrap_file();
    let bootstrap_path = project_root.join(bootstrap_file);

    Ok(BootstrapResult {
        bootstrap_path,
        context_path,
    })
}

fn sync_managed_templates(
    adapter_name: &str,
    project_root: &Path,
    mode: ProjectionMode,
) -> Result<()> {
    let adapter_dir = project_root.join(format!(".{}", adapter_name));
    if mode == ProjectionMode::CreateMissing && adapter_dir.exists() {
        return Ok(());
    }

    templates::copy_to_project(adapter_name, project_root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{self, ProjectConfig};
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn setup_project(adapter_name: &str) -> TempDir {
        let temp = TempDir::new().unwrap();
        let mut config = ProjectConfig::with_name("patina");
        config.adapters.allowed = vec!["claude".to_string(), adapter_name.to_string()];
        config.adapters.default = adapter_name.to_string();
        project::save(temp.path(), &config).unwrap();
        temp
    }

    fn with_temp_patina_home<T>(temp: &TempDir, f: impl FnOnce() -> T) -> T {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let patina_home = temp.path().join("patina-home");
        std::fs::create_dir_all(&patina_home).unwrap();

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
    fn creates_native_projection_for_opencode() {
        let temp = setup_project("opencode");

        let result = with_temp_patina_home(&temp, || {
            ensure_adapter_projection("opencode", temp.path(), ProjectionMode::RefreshManaged)
                .unwrap()
        });

        assert_eq!(result.bootstrap_path, temp.path().join("OPENCODE.md"));
        assert!(temp
            .path()
            .join(".opencode/commands/session-start.md")
            .exists());
        assert!(temp.path().join(".opencode/PATINA.md").exists());
    }

    #[test]
    fn refreshes_managed_files_without_overwriting_user_shell() {
        let temp = setup_project("gemini");
        let adapter_dir = temp.path().join(".gemini");
        std::fs::create_dir_all(&adapter_dir).unwrap();
        let shell_path = adapter_dir.join("GEMINI.md");
        std::fs::write(&shell_path, "# User shell\n").unwrap();

        let result = with_temp_patina_home(&temp, || {
            ensure_adapter_projection("gemini", temp.path(), ProjectionMode::RefreshManaged)
                .unwrap()
        });

        assert_eq!(result.context_path, adapter_dir.join("PATINA.md"));
        assert_eq!(
            std::fs::read_to_string(&shell_path).unwrap(),
            "# User shell\n"
        );
        assert!(temp.path().join("GEMINI.md").exists());
    }
}
