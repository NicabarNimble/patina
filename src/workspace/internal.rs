//! Internal implementation for workspace module
//!
//! Handles ~/.patina/ directory structure, config persistence, and first-run setup.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use super::SetupResult;

// =============================================================================
// Config Types
// =============================================================================

/// Global configuration stored in ~/.patina/config.toml
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub workspace: WorkspaceConfig,
    pub frontend: FrontendConfig,
    pub serve: ServeConfig,
    #[serde(default)]
    pub frontends: FrontendsConfig,
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
pub struct FrontendConfig {
    /// Default frontend to use (claude, gemini, codex)
    pub default: String,
}

impl Default for FrontendConfig {
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
    /// Auto-start mothership when launching frontend
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FrontendsConfig {
    #[serde(default)]
    pub claude: Option<FrontendEntry>,
    #[serde(default)]
    pub gemini: Option<FrontendEntry>,
    #[serde(default)]
    pub codex: Option<FrontendEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendEntry {
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
// Path Functions
// =============================================================================

/// Get the mothership directory (~/.patina)
pub fn mothership_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".patina")
}

/// Get the adapters directory (~/.patina/adapters)
pub fn adapters_dir() -> PathBuf {
    mothership_dir().join("adapters")
}

/// Get the config file path (~/.patina/config.toml)
pub fn config_path() -> PathBuf {
    mothership_dir().join("config.toml")
}

/// Get the workspace projects directory from config
pub fn projects_dir() -> Result<PathBuf> {
    let config = load_config()?;
    let path = shellexpand::tilde(&config.workspace.path);
    Ok(PathBuf::from(path.as_ref()))
}

// =============================================================================
// First-Run Detection
// =============================================================================

/// Check if this is first run
pub fn is_first_run() -> bool {
    !mothership_dir().exists()
}

// =============================================================================
// Setup
// =============================================================================

/// Perform first-run setup
pub fn setup() -> Result<SetupResult> {
    let mothership = mothership_dir();
    let adapters = adapters_dir();

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
    println!("  ✓ Installed adapters: claude, gemini, codex");

    // Detect installed frontends
    let mut detected = Vec::new();
    let mut frontends = FrontendsConfig::default();

    if detect_cli("claude") {
        detected.push("claude".to_string());
        frontends.claude = Some(FrontendEntry {
            command: "claude".to_string(),
            detected: true,
            mcp_config: Some("~/.claude/settings.json".to_string()),
        });
    }

    if detect_cli("gemini") {
        detected.push("gemini".to_string());
        frontends.gemini = Some(FrontendEntry {
            command: "gemini".to_string(),
            detected: true,
            mcp_config: None,
        });
    }

    if detect_cli("codex") {
        detected.push("codex".to_string());
        frontends.codex = Some(FrontendEntry {
            command: "codex".to_string(),
            detected: true,
            mcp_config: None,
        });
    }

    println!("\nDetecting LLM frontends...");
    for name in &["claude", "gemini", "codex"] {
        if detected.contains(&name.to_string()) {
            println!("  ✓ {} (found)", name);
        } else {
            println!("  ✗ {} (not found)", name);
        }
    }

    // Determine default frontend
    let default_frontend = detected.first().cloned();

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
        frontend: FrontendConfig {
            default: default_frontend
                .clone()
                .unwrap_or_else(|| "claude".to_string()),
        },
        serve: ServeConfig::default(),
        frontends,
    };

    save_config(&config)?;

    if let Some(ref frontend) = default_frontend {
        println!("\nSetting default: {}", frontend);
    }

    Ok(SetupResult {
        mothership_path: mothership,
        workspace_path,
        adapters_installed: vec![
            "claude".to_string(),
            "gemini".to_string(),
            "codex".to_string(),
        ],
        frontends_detected: detected,
        default_frontend,
    })
}

/// Ensure workspace exists (idempotent)
pub fn ensure_workspace() -> Result<()> {
    let mothership = mothership_dir();

    if !mothership.exists() {
        setup()?;
        return Ok(());
    }

    // Ensure subdirectories exist
    let adapters = adapters_dir();
    if !adapters.exists() {
        fs::create_dir_all(&adapters)?;
        for adapter in &["claude", "gemini", "codex"] {
            fs::create_dir_all(adapters.join(adapter))?;
        }
    }

    // Ensure config exists
    if !config_path().exists() {
        save_config(&GlobalConfig::default())?;
    }

    Ok(())
}

// =============================================================================
// Config
// =============================================================================

/// Load config from ~/.patina/config.toml
pub fn load_config() -> Result<GlobalConfig> {
    let path = config_path();

    if !path.exists() {
        return Ok(GlobalConfig::default());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;

    toml::from_str(&contents).with_context(|| format!("Failed to parse config: {}", path.display()))
}

/// Save config to ~/.patina/config.toml
pub fn save_config(config: &GlobalConfig) -> Result<()> {
    let path = config_path();

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
    let mothership = mothership_dir();
    let config = load_config()?;
    let workspace_path = PathBuf::from(shellexpand::tilde(&config.workspace.path).as_ref());

    // Check installed adapters
    let adapters = adapters_dir();
    let mut installed = Vec::new();
    for name in &["claude", "gemini", "codex"] {
        if adapters.join(name).exists() {
            installed.push(name.to_string());
        }
    }

    Ok(WorkspaceInfo {
        mothership_path: mothership.clone(),
        workspace_path,
        config_exists: config_path().exists(),
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
        assert_eq!(config.frontend.default, "claude");
        assert_eq!(config.serve.port, 50051);
        assert!(config.serve.auto_start);
    }

    #[test]
    fn test_config_serialization() {
        let config = GlobalConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[workspace]"));
        assert!(toml_str.contains("[frontend]"));
        assert!(toml_str.contains("[serve]"));
    }
}
