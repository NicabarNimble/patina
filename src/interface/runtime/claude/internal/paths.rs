//! Path management for Claude adapter

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Path constants for Claude adapter
pub const ADAPTER_DIR: &str = ".claude";
pub const CONTEXT_FILE: &str = "CLAUDE.md";
pub const MCP_DIR: &str = "mcp";
pub const COMMANDS_DIR: &str = "commands";
pub const BIN_DIR: &str = "bin";
pub const CONTEXT_DIR: &str = "context";
pub const SESSIONS_DIR: &str = "sessions";
pub const MANIFEST_FILE: &str = "adapter-manifest.json";

/// Get the base Claude directory path
pub fn get_claude_path(project_path: &Path) -> PathBuf {
    project_path.join(ADAPTER_DIR)
}

/// Get the MCP directory path
pub fn get_mcp_path(project_path: &Path) -> PathBuf {
    get_claude_path(project_path).join(MCP_DIR)
}

/// Get the commands directory path
pub fn get_commands_path(project_path: &Path) -> PathBuf {
    get_claude_path(project_path).join(COMMANDS_DIR)
}

/// Get the bin directory path
pub fn get_bin_path(project_path: &Path) -> PathBuf {
    get_claude_path(project_path).join(BIN_DIR)
}

/// Get the context directory path
pub fn get_context_path(project_path: &Path) -> PathBuf {
    get_claude_path(project_path).join(CONTEXT_DIR)
}

/// Get the sessions directory path
pub fn get_sessions_path(project_path: &Path) -> PathBuf {
    get_context_path(project_path).join(SESSIONS_DIR)
}

/// Get the context file path (.claude/CLAUDE.md)
pub fn get_context_file_path(project_path: &Path) -> PathBuf {
    get_claude_path(project_path).join(CONTEXT_FILE)
}

/// Get the manifest file path
pub fn get_manifest_path(project_path: &Path) -> PathBuf {
    get_claude_path(project_path).join(MANIFEST_FILE)
}

/// Create all necessary directories for Claude adapter
pub fn create_directory_structure(project_path: &Path) -> Result<()> {
    // Create main .claude directory
    fs::create_dir_all(get_claude_path(project_path))?;

    // Create subdirectories
    fs::create_dir_all(get_mcp_path(project_path))?;
    fs::create_dir_all(get_commands_path(project_path))?;
    fs::create_dir_all(get_bin_path(project_path))?;
    fs::create_dir_all(get_sessions_path(project_path))?;

    Ok(())
}
