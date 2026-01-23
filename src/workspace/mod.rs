//! Workspace module - Global Patina configuration and first-run setup
//!
//! Manages `~/.patina/` directory structure, global config, adapter installation,
//! and workspace folder. This is the foundation for the launcher architecture.
//!
//! # Example
//!
//! ```no_run
//! use patina::workspace;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Check if first run
//!     if workspace::is_first_run() {
//!         workspace::setup()?;
//!     }
//!
//!     // Load global config
//!     let config = workspace::config()?;
//!     println!("Default adapter: {}", config.adapter.default);
//!     Ok(())
//! }
//! ```

mod internal;

use anyhow::Result;

pub use internal::{GlobalConfig, WorkspaceInfo};

/// Check if this is first run (no ~/.patina/ directory)
pub fn is_first_run() -> bool {
    internal::is_first_run()
}

/// Perform first-run setup
///
/// Creates:
/// - `~/.patina/` directory structure
/// - `~/Projects/Patina` workspace folder (configurable)
/// - Default adapters (claude, gemini, codex)
/// - Global config file
pub fn setup() -> Result<SetupResult> {
    internal::setup()
}

/// Load global config from `~/.patina/config.toml`
pub fn config() -> Result<GlobalConfig> {
    internal::load_config()
}

/// Save global config to `~/.patina/config.toml`
pub fn save_config(config: &GlobalConfig) -> Result<()> {
    internal::save_config(config)
}

/// Get workspace info (paths, status)
pub fn info() -> Result<WorkspaceInfo> {
    internal::workspace_info()
}

/// Ensure ~/.patina/ exists with required structure
///
/// Safe to call multiple times - only creates what's missing.
pub fn ensure() -> Result<()> {
    internal::ensure_workspace()
}

// Path functions moved to patina::paths module
// Use paths::patina_home(), paths::adapters_dir(), etc.

/// Result of first-run setup
#[derive(Debug)]
pub struct SetupResult {
    /// Path to ~/.patina/
    pub mother_path: std::path::PathBuf,
    /// Path to workspace folder
    pub workspace_path: std::path::PathBuf,
    /// Installed adapters
    pub adapters_installed: Vec<String>,
    /// Detected adapter CLIs
    pub adapters_detected: Vec<String>,
    /// Default adapter set
    pub default_adapter: Option<String>,
}

// Tests for path functions are in paths module
