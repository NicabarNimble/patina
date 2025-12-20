//! Launcher functionality for adapters
//!
//! Provides CLI detection, MCP configuration, and bootstrap generation
//! for launching AI frontends. This complements the init-time functionality
//! in the adapter modules.
//!
//! # Example
//!
//! ```no_run
//! use patina::adapters::launch;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // List available frontends
//!     let frontends = launch::list()?;
//!     for f in &frontends {
//!         println!("{}: {} (detected: {})", f.name, f.display, f.detected);
//!     }
//!
//!     // Get specific frontend
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

/// Available frontend names
pub const FRONTENDS: &[&str] = &["claude", "gemini", "codex"];

// =============================================================================
// Types
// =============================================================================

/// Frontend identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frontend {
    Claude,
    Gemini,
    Codex,
}

impl Frontend {
    pub fn name(&self) -> &'static str {
        match self {
            Frontend::Claude => "claude",
            Frontend::Gemini => "gemini",
            Frontend::Codex => "codex",
        }
    }

    pub fn display(&self) -> &'static str {
        match self {
            Frontend::Claude => "Claude Code",
            Frontend::Gemini => "Gemini CLI",
            Frontend::Codex => "Codex",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "claude" => Some(Frontend::Claude),
            "gemini" => Some(Frontend::Gemini),
            "codex" => Some(Frontend::Codex),
            _ => None,
        }
    }

    pub fn bootstrap_file(&self) -> &'static str {
        match self {
            Frontend::Claude => "CLAUDE.md",
            Frontend::Gemini => "GEMINI.md",
            Frontend::Codex => "CODEX.md",
        }
    }

    pub fn detect_commands(&self) -> &'static [&'static str] {
        match self {
            Frontend::Claude => &["claude --version"],
            Frontend::Gemini => &["gemini --version"],
            Frontend::Codex => &["codex --version"],
        }
    }
}

/// Runtime frontend info with detection status
#[derive(Debug, Clone)]
pub struct FrontendInfo {
    pub name: String,
    pub display: String,
    pub detected: bool,
    pub version: Option<String>,
    pub mcp: Option<McpConfig>,
}

/// MCP configuration for a frontend
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

/// List all available frontends with detection status
pub fn list() -> Result<Vec<FrontendInfo>> {
    let mut frontends = Vec::new();

    for name in FRONTENDS {
        if let Ok(info) = get(name) {
            frontends.push(info);
        }
    }

    Ok(frontends)
}

/// Get info for a specific frontend
pub fn get(name: &str) -> Result<FrontendInfo> {
    let frontend =
        Frontend::from_name(name).ok_or_else(|| anyhow::anyhow!("Unknown frontend: {}", name))?;

    let (detected, version) = detect_cli(&frontend);

    Ok(FrontendInfo {
        name: frontend.name().to_string(),
        display: frontend.display().to_string(),
        detected,
        version,
        mcp: get_mcp_config(&frontend),
    })
}

/// Check if a frontend CLI is available
pub fn is_available(name: &str) -> bool {
    get(name).map(|f| f.detected).unwrap_or(false)
}

/// Get the default frontend name from global config
pub fn default_name() -> Result<String> {
    let config = workspace::config()?;
    Ok(config.frontend.default)
}

/// Set the default frontend
pub fn set_default(name: &str) -> Result<()> {
    // Verify frontend exists
    let _ =
        Frontend::from_name(name).ok_or_else(|| anyhow::anyhow!("Unknown frontend: {}", name))?;

    let mut config = workspace::config()?;
    config.frontend.default = name.to_string();
    workspace::save_config(&config)?;

    Ok(())
}

/// Generate bootstrap file for a project
pub fn generate_bootstrap(name: &str, project_path: &Path) -> Result<()> {
    let frontend =
        Frontend::from_name(name).ok_or_else(|| anyhow::anyhow!("Unknown frontend: {}", name))?;

    let bootstrap_path = project_path.join(frontend.bootstrap_file());
    let content = bootstrap_content(&frontend);

    fs::write(&bootstrap_path, content)
        .with_context(|| format!("Failed to write {}", bootstrap_path.display()))?;

    Ok(())
}

/// Configure MCP for a frontend (update its config file)
pub fn configure_mcp(name: &str) -> Result<()> {
    let info = get(name)?;

    let mcp = info
        .mcp
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Frontend {} has no MCP configuration", name))?;

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

/// Detect CLI version for a frontend
pub fn detect_version(name: &str) -> Option<String> {
    let frontend = Frontend::from_name(name)?;
    let (_, version) = detect_cli(&frontend);
    version
}

// =============================================================================
// Internal
// =============================================================================

/// Detect if CLI is installed and get version
fn detect_cli(frontend: &Frontend) -> (bool, Option<String>) {
    for cmd in frontend.detect_commands() {
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

/// Get MCP config for a frontend
fn get_mcp_config(frontend: &Frontend) -> Option<McpConfig> {
    match frontend {
        Frontend::Claude => Some(McpConfig {
            config_path: "~/.claude/settings.json".to_string(),
            config_format: "json".to_string(),
            config_template: Some(MCP_TEMPLATE.to_string()),
        }),
        Frontend::Gemini => None, // TBD
        Frontend::Codex => None,  // TBD
    }
}

/// Generate bootstrap file content
fn bootstrap_content(frontend: &Frontend) -> String {
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
*Generated by Patina | Frontend: {}*
"#,
        frontend.bootstrap_file(),
        frontend.display(),
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
    fn test_frontend_names() {
        assert_eq!(Frontend::Claude.name(), "claude");
        assert_eq!(Frontend::Gemini.name(), "gemini");
        assert_eq!(Frontend::Codex.name(), "codex");
    }

    #[test]
    fn test_frontend_from_name() {
        assert_eq!(Frontend::from_name("claude"), Some(Frontend::Claude));
        assert_eq!(Frontend::from_name("CLAUDE"), Some(Frontend::Claude));
        assert_eq!(Frontend::from_name("unknown"), None);
    }

    #[test]
    fn test_bootstrap_files() {
        assert_eq!(Frontend::Claude.bootstrap_file(), "CLAUDE.md");
        assert_eq!(Frontend::Gemini.bootstrap_file(), "GEMINI.md");
        assert_eq!(Frontend::Codex.bootstrap_file(), "CODEX.md");
    }

    #[test]
    fn test_frontends_list() {
        assert!(FRONTENDS.contains(&"claude"));
        assert!(FRONTENDS.contains(&"gemini"));
        assert!(FRONTENDS.contains(&"codex"));
    }
}
