//! Launcher functionality for adapters
//!
//! Provides CLI detection, MCP configuration, and bootstrap generation
//! for launching AI adapters. This complements the init-time functionality
//! in the adapter modules.
//!
//! # Example
//!
//! ```no_run
//! use patina::adapters::launch;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // List available adapters
//!     let adapters = launch::list()?;
//!     for f in &adapters {
//!         println!("{}: {} (detected: {})", f.name, f.display, f.detected);
//!     }
//!
//!     // Get specific adapter
//!     let claude = launch::get("claude")?;
//!     Ok(())
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::workspace;

/// Available adapter names
pub const ADAPTERS: &[&str] = &["claude", "gemini", "opencode"];

// =============================================================================
// Types
// =============================================================================

/// Adapter identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adapter {
    Claude,
    Gemini,
    OpenCode,
}

impl Adapter {
    pub fn name(&self) -> &'static str {
        match self {
            Adapter::Claude => "claude",
            Adapter::Gemini => "gemini",
            Adapter::OpenCode => "opencode",
        }
    }

    pub fn display(&self) -> &'static str {
        match self {
            Adapter::Claude => "Claude Code",
            Adapter::Gemini => "Gemini CLI",
            Adapter::OpenCode => "OpenCode",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "claude" => Some(Adapter::Claude),
            "gemini" => Some(Adapter::Gemini),
            "opencode" => Some(Adapter::OpenCode),
            _ => None,
        }
    }

    pub fn bootstrap_file(&self) -> &'static str {
        match self {
            Adapter::Claude => "CLAUDE.md",
            Adapter::Gemini => "GEMINI.md",
            Adapter::OpenCode => "OPENCODE.md",
        }
    }

    pub fn detect_commands(&self) -> &'static [&'static str] {
        match self {
            Adapter::Claude => &["claude --version"],
            Adapter::Gemini => &["gemini --version"],
            Adapter::OpenCode => &["opencode --version"],
        }
    }
}

/// Runtime adapter info with detection status
#[derive(Debug, Clone)]
pub struct AdapterInfo {
    pub name: String,
    pub display: String,
    pub detected: bool,
    pub version: Option<String>,
    pub mcp: Option<McpConfig>,
}

/// MCP configuration for an adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub config_path: String,
    pub config_format: String,
    #[serde(default)]
    pub config_template: Option<String>,
}

// =============================================================================
// Public API
// =============================================================================

/// List all available adapters with detection status
pub fn list() -> Result<Vec<AdapterInfo>> {
    let mut adapters = Vec::new();

    for name in ADAPTERS {
        if let Ok(info) = get(name) {
            adapters.push(info);
        }
    }

    Ok(adapters)
}

/// Get info for a specific adapter
pub fn get(name: &str) -> Result<AdapterInfo> {
    let adapter =
        Adapter::from_name(name).ok_or_else(|| anyhow::anyhow!("Unknown adapter: {}", name))?;

    let (detected, version) = detect_cli(&adapter);

    Ok(AdapterInfo {
        name: adapter.name().to_string(),
        display: adapter.display().to_string(),
        detected,
        version,
        mcp: get_mcp_config(&adapter),
    })
}

/// Check if an adapter CLI is available
pub fn is_available(name: &str) -> bool {
    get(name).map(|f| f.detected).unwrap_or(false)
}

/// Get the default adapter name from global config
pub fn default_name() -> Result<String> {
    let config = workspace::config()?;
    Ok(config.adapter.default)
}

/// Set the default adapter
pub fn set_default(name: &str) -> Result<()> {
    // Verify adapter exists
    let _ = Adapter::from_name(name).ok_or_else(|| anyhow::anyhow!("Unknown adapter: {}", name))?;

    let mut config = workspace::config()?;
    config.adapter.default = name.to_string();
    workspace::save_config(&config)?;

    Ok(())
}

/// Generate bootstrap file for a project
pub fn generate_bootstrap(name: &str, project_path: &Path) -> Result<()> {
    let adapter =
        Adapter::from_name(name).ok_or_else(|| anyhow::anyhow!("Unknown adapter: {}", name))?;

    let bootstrap_path = project_path.join(adapter.bootstrap_file());
    let content = bootstrap_content(&adapter);

    fs::write(&bootstrap_path, content)
        .with_context(|| format!("Failed to write {}", bootstrap_path.display()))?;

    Ok(())
}

/// Configure MCP for an adapter (update its config file)
pub fn configure_mcp(name: &str) -> Result<()> {
    let info = get(name)?;

    let mcp = info
        .mcp
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Adapter {} has no MCP configuration", name))?;

    let config_path = PathBuf::from(shellexpand::tilde(&mcp.config_path).as_ref());

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // For now, just write template if no config exists
    if !config_path.exists() {
        if let Some(template) = &mcp.config_template {
            fs::write(&config_path, template)?;
        }
    } else {
        eprintln!(
            "MCP config exists at {}. Please manually add patina server.",
            config_path.display()
        );
    }

    Ok(())
}

/// Detect CLI version for an adapter
pub fn detect_version(name: &str) -> Option<String> {
    let adapter = Adapter::from_name(name)?;
    let (_, version) = detect_cli(&adapter);
    version
}

/// Select an adapter from available options.
///
/// Returns the chosen adapter name.
///
/// Behavior:
/// - 0 available: Error with installation instructions
/// - 1 available: Returns it (no prompt)
/// - 2+ available: Prompts user to choose
///
/// If `preference` matches an available adapter, it becomes the default selection.
/// This is used to honor the global config default without forcing it.
pub fn select_adapter(available: &[AdapterInfo], preference: Option<&str>) -> Result<String> {
    use std::io::{self, Write};

    match available.len() {
        0 => {
            anyhow::bail!(
                "No AI adapters detected on this system.\n\
                 Install one of: {}",
                ADAPTERS.join(", ")
            );
        }
        1 => {
            // Single adapter - use it without prompting
            Ok(available[0].name.clone())
        }
        _ => {
            // Multiple adapters - prompt user to choose
            println!("\n📱 Available adapters:");

            // Find which index should be default (1-based for display)
            let default_idx = preference
                .and_then(|pref| available.iter().position(|a| a.name == pref))
                .map(|i| i + 1)
                .unwrap_or(1);

            for (i, adapter) in available.iter().enumerate() {
                let num = i + 1;
                let default_marker = if num == default_idx { " (default)" } else { "" };
                println!("  [{}] {}{}", num, adapter.display, default_marker);
            }

            print!("\nSelect adapter [{}]: ", default_idx);
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let choice = input.trim();
            let idx = if choice.is_empty() {
                default_idx
            } else {
                choice.parse::<usize>().unwrap_or(default_idx)
            };

            // Validate and return
            if idx >= 1 && idx <= available.len() {
                Ok(available[idx - 1].name.clone())
            } else {
                // Invalid input, use default
                Ok(available[default_idx - 1].name.clone())
            }
        }
    }
}

// =============================================================================
// Internal
// =============================================================================

/// Detect if CLI is installed and get version
fn detect_cli(adapter: &Adapter) -> (bool, Option<String>) {
    for cmd in adapter.detect_commands() {
        if let Some(version) = try_command(cmd) {
            return (true, Some(version));
        }
    }
    (false, None)
}

/// Try running a command and capture version output
fn try_command(cmd: &str) -> Option<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let output = Command::new(parts[0]).args(&parts[1..]).output().ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Return first non-empty line as version
        stdout
            .lines()
            .chain(stderr.lines())
            .find(|l| !l.trim().is_empty())
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

/// Get MCP config for an adapter
fn get_mcp_config(adapter: &Adapter) -> Option<McpConfig> {
    match adapter {
        Adapter::Claude => Some(McpConfig {
            config_path: "~/.claude/settings.json".to_string(),
            config_format: "json".to_string(),
            config_template: Some(MCP_TEMPLATE.to_string()),
        }),
        Adapter::Gemini => None,   // TBD
        Adapter::OpenCode => None, // TBD
    }
}

/// Generate bootstrap file content
fn bootstrap_content(adapter: &Adapter) -> String {
    format!(
        r#"# {}

This project uses Patina for knowledge management.

## MCP Tools (Use These!)

**`scry`** - Search codebase knowledge
- USE FIRST for any question about the code
- Searches indexed symbols, functions, git history, session learnings
- Example: "how does authentication work?"

**`context`** - Get project patterns
- USE to understand design rules before making changes
- Returns core patterns (eternal principles) and surface patterns (active architecture)

💡 These tools search pre-indexed knowledge - faster than manual file exploration.

---
*Generated by Patina | Adapter: {}*
"#,
        adapter.bootstrap_file(),
        adapter.display(),
    )
}

const MCP_TEMPLATE: &str = r#"{
  "mcpServers": {
    "patina": {
      "command": "patina",
      "args": ["serve", "--mcp-stdio"]
    }
  }
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_names() {
        assert_eq!(Adapter::Claude.name(), "claude");
        assert_eq!(Adapter::Gemini.name(), "gemini");
        assert_eq!(Adapter::OpenCode.name(), "opencode");
    }

    #[test]
    fn test_adapter_from_name() {
        assert_eq!(Adapter::from_name("claude"), Some(Adapter::Claude));
        assert_eq!(Adapter::from_name("CLAUDE"), Some(Adapter::Claude));
        assert_eq!(Adapter::from_name("opencode"), Some(Adapter::OpenCode));
        assert_eq!(Adapter::from_name("OpenCode"), Some(Adapter::OpenCode));
        assert_eq!(Adapter::from_name("unknown"), None);
    }

    #[test]
    fn test_bootstrap_files() {
        assert_eq!(Adapter::Claude.bootstrap_file(), "CLAUDE.md");
        assert_eq!(Adapter::Gemini.bootstrap_file(), "GEMINI.md");
        assert_eq!(Adapter::OpenCode.bootstrap_file(), "OPENCODE.md");
    }

    #[test]
    fn test_adapters_list() {
        assert!(ADAPTERS.contains(&"claude"));
        assert!(ADAPTERS.contains(&"gemini"));
        assert!(ADAPTERS.contains(&"opencode"));
        assert_eq!(ADAPTERS.len(), 3);
    }

    #[test]
    fn test_select_adapter_zero_available() {
        let available: Vec<AdapterInfo> = vec![];
        let result = select_adapter(&available, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No AI adapters detected"));
    }

    #[test]
    fn test_select_adapter_single_available() {
        let available = vec![AdapterInfo {
            name: "claude".to_string(),
            display: "Claude Code".to_string(),
            detected: true,
            version: Some("1.0".to_string()),
            mcp: None,
        }];
        let result = select_adapter(&available, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "claude");
    }

    #[test]
    fn test_select_adapter_single_ignores_preference() {
        let available = vec![AdapterInfo {
            name: "gemini".to_string(),
            display: "Gemini CLI".to_string(),
            detected: true,
            version: None,
            mcp: None,
        }];
        // Even with claude preference, returns the only available adapter
        let result = select_adapter(&available, Some("claude"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "gemini");
    }
}
