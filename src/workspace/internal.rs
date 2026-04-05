//! Internal implementation for workspace module
//!
//! Handles ~/.patina/ directory structure, config persistence, and first-run setup.
//! Path definitions are in the paths module - this module contains behavior.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use super::SetupResult;
use crate::paths;

// =============================================================================
// Config Types
// =============================================================================

/// Global configuration stored in ~/.patina/config.toml
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub workspace: WorkspaceConfig,
    #[serde(rename = "interface")]
    pub interface: InterfaceConfig,
    pub serve: ServeConfig,
    #[serde(default, rename = "interfaces")]
    pub interfaces: InterfacesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Path to workspace folder (default: ~/Projects/Patina)
    pub path: String,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            path: home
                .join("Projects")
                .join("Patina")
                .to_string_lossy()
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceConfig {
    /// Default interface to use (claude, gemini, opencode)
    pub default: String,
}

impl Default for InterfaceConfig {
    fn default() -> Self {
        Self {
            default: "claude".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServeConfig {
    /// Port for mother server
    pub port: u16,
    /// Auto-start mother when launching interface
    pub auto_start: bool,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            port: 50051,
            auto_start: true,
        }
    }
}

/// Detected interfaces configuration (flexible HashMap)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InterfacesConfig {
    #[serde(flatten)]
    pub entries: HashMap<String, InterfaceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceEntry {
    pub command: String,
    pub detected: bool,
    #[serde(default)]
    pub mcp_config: Option<String>,
}

/// Workspace status info
#[derive(Debug)]
pub struct WorkspaceInfo {
    pub mother_path: PathBuf,
    pub workspace_path: PathBuf,
    pub config_exists: bool,
    pub interfaces_installed: Vec<String>,
}

// =============================================================================
// First-Run Detection
// =============================================================================

/// Check if this is first run
pub fn is_first_run() -> bool {
    !paths::patina_home().exists()
}

// =============================================================================
// Setup
// =============================================================================

/// Perform first-run setup
pub fn setup() -> Result<SetupResult> {
    let mother = paths::patina_home();
    let interfaces = paths::interfaces_dir();

    // Create directory structure
    println!("First-time setup...");

    // ~/.patina/
    fs::create_dir_all(&mother)
        .with_context(|| format!("Failed to create {}", mother.display()))?;
    println!("  ✓ Created {}", mother.display());

    // ~/.patina/mother/persona/default/
    let persona_dir =
        paths::mother::persona::ensure_persona_dir("default").map_err(|e| anyhow::anyhow!(e))?;
    let beliefs_db =
        paths::mother::persona::beliefs_db("default").map_err(|e| anyhow::anyhow!(e))?;
    rusqlite::Connection::open(&beliefs_db)
        .with_context(|| format!("Failed to initialize {}", beliefs_db.display()))?;
    let identity_age =
        paths::mother::persona::identity_age("default").map_err(|e| anyhow::anyhow!(e))?;
    if !identity_age.exists() {
        fs::write(&identity_age, "")
            .with_context(|| format!("Failed to create {}", identity_age.display()))?;
    }
    println!("  ✓ Created {}", persona_dir.display());

    // ~/.patina/interfaces/
    fs::create_dir_all(&interfaces)?;

    // ~/.patina/interfaces/{claude,gemini,codex}/
    for iface in &["claude", "gemini", "codex"] {
        fs::create_dir_all(interfaces.join(iface))?;
    }

    println!("  ✓ Installing interface templates...");
    crate::interface::templates::install_all(&interfaces)?;
    println!("  ✓ Installed interfaces: claude, gemini, codex");

    // Detect installed interfaces
    let mut detected = Vec::new();
    let mut interfaces_config = InterfacesConfig::default();

    // Detect available interfaces
    for (name, mcp_config) in [
        ("claude", Some("~/.claude/settings.json")),
        ("gemini", None),
        ("codex", None),
        ("opencode", None),
    ] {
        if detect_cli(name) {
            detected.push(name.to_string());
            interfaces_config.entries.insert(
                name.to_string(),
                InterfaceEntry {
                    command: name.to_string(),
                    detected: true,
                    mcp_config: mcp_config.map(String::from),
                },
            );
        }
    }

    println!("\nDetecting LLM interfaces...");
    for name in &["claude", "gemini", "codex", "opencode"] {
        if detected.contains(&name.to_string()) {
            println!("  ✓ {} (found)", name);
        } else {
            println!("  ✗ {} (not found)", name);
        }
    }

    let default_interface = detected.first().cloned();

    // Create workspace folder
    let workspace_path = dirs::home_dir()
        .unwrap_or_default()
        .join("Projects")
        .join("Patina");

    if !workspace_path.exists() {
        fs::create_dir_all(&workspace_path)?;
        println!("  ✓ Created {} workspace", workspace_path.display());
    }

    // Create config
    let config = GlobalConfig {
        workspace: WorkspaceConfig {
            path: workspace_path.to_string_lossy().to_string(),
        },
        interface: InterfaceConfig {
            default: default_interface
                .clone()
                .unwrap_or_else(|| "claude".to_string()),
        },
        serve: ServeConfig::default(),
        interfaces: interfaces_config,
    };

    save_config(&config)?;

    if let Some(ref iface) = default_interface {
        println!("\nSetting default: {}", iface);
    }

    Ok(SetupResult {
        mother_path: mother,
        workspace_path,
        interfaces_installed: vec![
            "claude".to_string(),
            "gemini".to_string(),
            "codex".to_string(),
            "opencode".to_string(),
        ],
        interfaces_detected: detected,
        default_interface,
    })
}

/// Ensure workspace exists (idempotent)
pub fn ensure_workspace() -> Result<()> {
    let mother = paths::patina_home();

    if !mother.exists() {
        setup()?;
        return Ok(());
    }

    // Ensure subdirectories exist
    let interfaces = paths::interfaces_dir();
    if !interfaces.exists() {
        fs::create_dir_all(&interfaces)?;
        for iface in &["claude", "gemini", "codex"] {
            fs::create_dir_all(interfaces.join(iface))?;
        }
        crate::interface::templates::install_all(&interfaces)?;
    } else {
        let claude_templates = interfaces.join("claude").join("templates");
        if !claude_templates.exists() {
            crate::interface::templates::install_all(&interfaces)?;
        }
    }

    // Ensure config exists
    if !paths::config_path().exists() {
        save_config(&GlobalConfig::default())?;
    }

    Ok(())
}

// =============================================================================
// Config
// =============================================================================

/// Load config from ~/.patina/config.toml
pub fn load_config() -> Result<GlobalConfig> {
    let path = paths::config_path();

    if !path.exists() {
        return Ok(GlobalConfig::default());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;

    toml::from_str(&contents).with_context(|| format!("Failed to parse config: {}", path.display()))
}

/// Save config to ~/.patina/config.toml
pub fn save_config(config: &GlobalConfig) -> Result<()> {
    let path = paths::config_path();

    // Ensure parent exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let contents = toml::to_string_pretty(config)?;
    fs::write(&path, contents)?;
    Ok(())
}

// =============================================================================
// Info
// =============================================================================

/// Get workspace info
pub fn workspace_info() -> Result<WorkspaceInfo> {
    let mother = paths::patina_home();
    let config = load_config()?;
    let workspace_path = PathBuf::from(shellexpand::tilde(&config.workspace.path).as_ref());

    let interfaces = paths::interfaces_dir();
    let mut installed = Vec::new();
    for name in &["claude", "gemini", "codex"] {
        if interfaces.join(name).exists() {
            installed.push(name.to_string());
        }
    }

    Ok(WorkspaceInfo {
        mother_path: mother.clone(),
        workspace_path,
        config_exists: paths::config_path().exists(),
        interfaces_installed: installed,
    })
}

// =============================================================================
// CLI Detection
// =============================================================================

/// Detect if a CLI command is available
fn detect_cli(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GlobalConfig::default();
        assert_eq!(config.interface.default, "claude");
        assert_eq!(config.serve.port, 50051);
        assert!(config.serve.auto_start);
    }

    #[test]
    fn test_config_serialization() {
        let config = GlobalConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[workspace]"));
        assert!(toml_str.contains("[interface]"));
        assert!(toml_str.contains("[serve]"));
    }
}
