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
    pub adapter: AdapterConfig,
    pub serve: ServeConfig,
    #[serde(default)]
    pub adapters: AdaptersConfig,
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
pub struct AdapterConfig {
    /// Default adapter to use (claude, gemini, codex)
    pub default: String,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            default: "claude".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServeConfig {
    /// Port for mothership server
    pub port: u16,
    /// Auto-start mothership when launching adapter
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

/// Detected adapters configuration (flexible HashMap)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdaptersConfig {
    #[serde(flatten)]
    pub entries: HashMap<String, AdapterEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterEntry {
    pub command: String,
    pub detected: bool,
    #[serde(default)]
    pub mcp_config: Option<String>,
}

/// Workspace status info
#[derive(Debug)]
pub struct WorkspaceInfo {
    pub mothership_path: PathBuf,
    pub workspace_path: PathBuf,
    pub config_exists: bool,
    pub adapters_installed: Vec<String>,
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
    let mothership = paths::patina_home();
    let adapters = paths::adapters_dir();

    // Create directory structure
    println!("First-time setup...");

    // ~/.patina/
    fs::create_dir_all(&mothership)
        .with_context(|| format!("Failed to create {}", mothership.display()))?;
    println!("  ✓ Created {}", mothership.display());

    // ~/.patina/adapters/
    fs::create_dir_all(&adapters)?;

    // ~/.patina/adapters/{claude,gemini,codex}/
    for adapter in &["claude", "gemini", "codex"] {
        let adapter_dir = adapters.join(adapter);
        fs::create_dir_all(&adapter_dir)?;
    }

    // Extract embedded templates to ~/.patina/adapters/
    println!("  ✓ Installing adapter templates...");
    crate::adapters::templates::install_all(&adapters)?;
    println!("  ✓ Installed adapters: claude, gemini, codex");

    // Detect installed adapters
    let mut detected = Vec::new();
    let mut adapters_config = AdaptersConfig::default();

    // Detect available adapters
    for (name, mcp_config) in [
        ("claude", Some("~/.claude/settings.json")),
        ("gemini", None),
        ("codex", None),
        ("opencode", None),
    ] {
        if detect_cli(name) {
            detected.push(name.to_string());
            adapters_config.entries.insert(
                name.to_string(),
                AdapterEntry {
                    command: name.to_string(),
                    detected: true,
                    mcp_config: mcp_config.map(String::from),
                },
            );
        }
    }

    println!("\nDetecting LLM adapters...");
    for name in &["claude", "gemini", "codex", "opencode"] {
        if detected.contains(&name.to_string()) {
            println!("  ✓ {} (found)", name);
        } else {
            println!("  ✗ {} (not found)", name);
        }
    }

    // Determine default adapter
    let default_adapter = detected.first().cloned();

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
        adapter: AdapterConfig {
            default: default_adapter
                .clone()
                .unwrap_or_else(|| "claude".to_string()),
        },
        serve: ServeConfig::default(),
        adapters: adapters_config,
    };

    save_config(&config)?;

    if let Some(ref adapter) = default_adapter {
        println!("\nSetting default: {}", adapter);
    }

    Ok(SetupResult {
        mothership_path: mothership,
        workspace_path,
        adapters_installed: vec![
            "claude".to_string(),
            "gemini".to_string(),
            "codex".to_string(),
            "opencode".to_string(),
        ],
        adapters_detected: detected,
        default_adapter,
    })
}

/// Ensure workspace exists (idempotent)
pub fn ensure_workspace() -> Result<()> {
    let mothership = paths::patina_home();

    if !mothership.exists() {
        setup()?;
        return Ok(());
    }

    // Ensure subdirectories exist
    let adapters = paths::adapters_dir();
    if !adapters.exists() {
        fs::create_dir_all(&adapters)?;
        for adapter in &["claude", "gemini", "codex"] {
            fs::create_dir_all(adapters.join(adapter))?;
        }
        // Install templates if adapters directory was just created
        crate::adapters::templates::install_all(&adapters)?;
    } else {
        // Check if templates need to be installed
        let claude_templates = adapters.join("claude").join("templates");
        if !claude_templates.exists() {
            crate::adapters::templates::install_all(&adapters)?;
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
    let mothership = paths::patina_home();
    let config = load_config()?;
    let workspace_path = PathBuf::from(shellexpand::tilde(&config.workspace.path).as_ref());

    // Check installed adapters
    let adapters = paths::adapters_dir();
    let mut installed = Vec::new();
    for name in &["claude", "gemini", "codex"] {
        if adapters.join(name).exists() {
            installed.push(name.to_string());
        }
    }

    Ok(WorkspaceInfo {
        mothership_path: mothership.clone(),
        workspace_path,
        config_exists: paths::config_path().exists(),
        adapters_installed: installed,
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
        assert_eq!(config.adapter.default, "claude");
        assert_eq!(config.serve.port, 50051);
        assert!(config.serve.auto_start);
    }

    #[test]
    fn test_config_serialization() {
        let config = GlobalConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[workspace]"));
        assert!(toml_str.contains("[adapter]"));
        assert!(toml_str.contains("[serve]"));
    }
}
