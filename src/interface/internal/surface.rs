use anyhow::Result;
use std::path::Path;

use crate::adapters::launch;
use crate::interface::internal::bootstrap::bundle_deployment_status;
use crate::interface::internal::bundle::{
    canonical_interface_name, canonicalize_required_interface, interface_bundle,
    interface_bundle_catalog,
};
use crate::interface::{self, BootstrapResult, ProjectionMode};
use crate::project;

#[derive(Debug, Clone)]
pub struct AiProjectConfigResult {
    pub default_interface: String,
    pub updated: bool,
}

#[derive(Debug, Clone)]
pub struct PreparedInterface {
    pub name: String,
    pub display_name: String,
    pub bootstrap: BootstrapResult,
    pub current: bool,
}

#[derive(Debug, Clone)]
pub struct AiSurfaceResult {
    pub prepared: Vec<PreparedInterface>,
    pub default_interface: String,
    pub config_updated: bool,
}

#[derive(Debug, Clone)]
pub struct AiSurfaceRequest<'a> {
    pub project_root: &'a Path,
    pub force: bool,
    pub default_interface: Option<&'a str>,
}

pub fn supported_ai_interfaces() -> &'static [&'static str] {
    static SUPPORTED: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    SUPPORTED
        .get_or_init(|| {
            interface_bundle_catalog()
                .iter()
                .map(|bundle| bundle.name)
                .collect()
        })
        .as_slice()
}

pub fn resolve_preferred_ai_interface(project_root: &Path) -> Result<String> {
    let config = project::load_with_migration(project_root)?;
    choose_default_interface(&config, None)
}

pub fn set_project_default_interface(project_root: &Path, name: &str) -> Result<bool> {
    Ok(ensure_ai_project_config(project_root, Some(name))?.updated)
}

pub fn ensure_ai_project_config(
    project_root: &Path,
    default_interface: Option<&str>,
) -> Result<AiProjectConfigResult> {
    let mut config = project::load_with_migration(project_root)?;
    let desired_default = choose_default_interface(&config, default_interface)?;
    let mut updated = false;

    for supported in supported_ai_interfaces() {
        if !config
            .adapters
            .allowed
            .iter()
            .any(|allowed| allowed == supported)
        {
            config.adapters.allowed.push((*supported).to_string());
            updated = true;
        }
    }

    if config.adapters.default != desired_default {
        config.adapters.default = desired_default.clone();
        updated = true;
    }

    if updated {
        project::save(project_root, &config)?;
    }

    Ok(AiProjectConfigResult {
        default_interface: desired_default,
        updated,
    })
}

pub fn ensure_ai_surface(request: AiSurfaceRequest<'_>) -> Result<AiSurfaceResult> {
    let config = ensure_ai_project_config(request.project_root, request.default_interface)?;
    let mode = if request.force {
        ProjectionMode::ForceRewrite
    } else {
        ProjectionMode::RefreshManaged
    };
    let mut prepared = Vec::new();

    for name in supported_ai_interfaces() {
        let status = bundle_deployment_status(name, request.project_root)?;
        let bootstrap = interface::ensure_bundle_projection(name, request.project_root, mode)?;
        prepared.push(PreparedInterface {
            name: (*name).to_string(),
            display_name: interface_bundle(name)?.display_name.to_string(),
            bootstrap,
            current: status.current,
        });
    }

    Ok(AiSurfaceResult {
        prepared,
        default_interface: config.default_interface,
        config_updated: config.updated,
    })
}

pub fn prepare_ai_bundle(
    project_root: &Path,
    interface_name: &str,
    force: bool,
) -> Result<PreparedInterface> {
    let interface_name = canonicalize_required_interface(interface_name)?;
    let status = bundle_deployment_status(interface_name, project_root)?;
    let mode = if force {
        ProjectionMode::ForceRewrite
    } else {
        ProjectionMode::RefreshManaged
    };
    let bootstrap = if force {
        interface::ensure_bundle_projection(interface_name, project_root, mode)?
    } else {
        interface::ensure_bundle_bootstrap(interface_name, project_root)?
    };
    let bundle = interface_bundle(interface_name)?;

    Ok(PreparedInterface {
        name: bundle.name.to_string(),
        display_name: bundle.display_name.to_string(),
        bootstrap,
        current: status.current && !force,
    })
}

fn choose_default_interface(
    config: &project::ProjectConfig,
    explicit: Option<&str>,
) -> Result<String> {
    if let Some(name) = explicit {
        return canonicalize_required_interface(name).map(str::to_string);
    }

    if let Some(name) = canonical_interface_name(&config.adapters.default) {
        return Ok(name.to_string());
    }

    if let Ok(workspace_default) = launch::default_name() {
        if let Some(name) = canonical_interface_name(&workspace_default) {
            return Ok(name.to_string());
        }
    }

    for candidate in supported_ai_interfaces() {
        if launch::get(candidate)
            .map(|info| info.detected)
            .unwrap_or(false)
        {
            return Ok((*candidate).to_string());
        }
    }

    Ok(supported_ai_interfaces()[0].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{self, ProjectConfig};
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn setup_project() -> TempDir {
        let temp = TempDir::new().unwrap();
        let mut config = ProjectConfig::with_name("patina");
        config.adapters.allowed = vec!["claude".to_string()];
        config.adapters.default = String::new();
        project::save(temp.path(), &config).unwrap();
        temp
    }

    fn with_temp_env<T>(temp: &TempDir, f: impl FnOnce() -> T) -> T {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let patina_home = temp.path().join("patina-home");
        fs::create_dir_all(&patina_home).unwrap();

        let old_home = std::env::var_os("HOME");
        let old_patina_home = std::env::var_os("PATINA_HOME");
        unsafe {
            std::env::set_var("HOME", temp.path());
            std::env::set_var("PATINA_HOME", &patina_home);
        }
        let result = f();
        match old_home {
            Some(value) => unsafe {
                std::env::set_var("HOME", value);
            },
            None => unsafe {
                std::env::remove_var("HOME");
            },
        }
        match old_patina_home {
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
    fn ensure_ai_project_config_populates_supported_interfaces_and_default() {
        let temp = setup_project();

        let result = with_temp_env(&temp, || {
            ensure_ai_project_config(temp.path(), None).unwrap()
        });

        let config = project::load_with_migration(temp.path()).unwrap();
        assert!(result.updated);
        assert_eq!(config.adapters.default, result.default_interface);
        assert!(config.adapters.allowed.iter().any(|name| name == "claude"));
        assert!(config
            .adapters
            .allowed
            .iter()
            .any(|name| name == "opencode"));
        assert!(config.adapters.allowed.iter().any(|name| name == "gemini"));
    }

    #[test]
    fn ensure_ai_surface_prepares_all_peer_interfaces() {
        let temp = setup_project();

        let result = with_temp_env(&temp, || {
            ensure_ai_surface(AiSurfaceRequest {
                project_root: temp.path(),
                force: false,
                default_interface: Some("claude"),
            })
            .unwrap()
        });

        assert_eq!(result.default_interface, "claude");
        assert_eq!(result.prepared.len(), 3);
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
        assert!(temp.path().join(".opencode/commands/spec.md").exists());
        assert!(temp
            .path()
            .join(".opencode/commands/epistemic-beliefs.md")
            .exists());
        assert!(temp
            .path()
            .join(".gemini/commands/session-start.toml")
            .exists());
        assert!(temp.path().join(".gemini/commands/spec.toml").exists());
        assert!(temp
            .path()
            .join(".gemini/commands/epistemic-beliefs.toml")
            .exists());
    }

    #[test]
    fn prepare_ai_bundle_refreshes_only_requested_interface() {
        let temp = setup_project();

        with_temp_env(&temp, || {
            ensure_ai_surface(AiSurfaceRequest {
                project_root: temp.path(),
                force: false,
                default_interface: Some("claude"),
            })
            .unwrap()
        });

        let bootstrap = with_temp_env(&temp, || {
            prepare_ai_bundle(temp.path(), "gemini", false).unwrap()
        });

        assert_eq!(bootstrap.name, "gemini");
        assert_eq!(bootstrap.display_name, "Gemini CLI");
        assert!(bootstrap.bootstrap.bootstrap_path.ends_with("GEMINI.md"));
    }

    #[test]
    fn set_project_default_interface_updates_project_default() {
        let temp = setup_project();

        with_temp_env(&temp, || {
            set_project_default_interface(temp.path(), "gemini").unwrap()
        });

        let config = project::load_with_migration(temp.path()).unwrap();
        assert_eq!(config.adapters.default, "gemini");
    }
}
