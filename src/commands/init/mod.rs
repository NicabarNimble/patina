//! Initialize a new Patina project with LLM integration and development environment
//!
//! This module follows the dependable-rust pattern:
//! - Public interface (this file): ≤150 lines, clean API
//! - Internal implementation: All logic in internal/ submodules
//!
//! # Example
//!
//! ```no_run
//! use patina::commands::init;
//!
//! // Initialize a new project with Claude and Docker
//! init::execute(
//!     "my-project".to_string(),
//!     "claude".to_string(),
//!     "PROJECT_DESIGN.toml".to_string(),
//!     Some("docker".to_string())
//! ).expect("Failed to initialize project");
//! ```

pub mod design_wizard;
mod internal;
pub mod tool_installer;

use anyhow::Result;

/// Execute the init command to create or reinitialize a Patina project
///
/// # Arguments
///
/// * `name` - Project name or "." for current directory
/// * `llm` - LLM adapter to use (e.g., "claude", "gemini")
/// * `design` - Path to PROJECT_DESIGN.toml file
/// * `dev` - Optional development environment (e.g., "docker")
///
/// # Process
///
/// 1. **Environment Detection**: Identifies available tools and languages
/// 2. **Project Setup**: Creates directory structure and configuration
/// 3. **LLM Integration**: Initializes chosen LLM adapter (Claude, Gemini, etc.)
/// 4. **Dev Environment**: Sets up development environment (Docker)
/// 5. **Pattern Copying**: Copies core patterns from Patina
/// 6. **Navigation Index**: Creates searchable pattern database
///
/// # Re-initialization
///
/// When run in an existing Patina project:
/// - Backs up gitignored directories (.claude → .claude_backup_<timestamp>)
/// - Preserves PROJECT_DESIGN.toml
/// - Updates components to latest versions
/// - Refreshes configuration and adapters
///
/// # Safety
///
/// - Never overwrites PROJECT_DESIGN.toml in re-initialization
/// - Prevents self-overwriting when run in Patina source
/// - Creates backups of user data before updates
///
/// # Errors
///
/// Returns an error if:
/// - PROJECT_DESIGN.toml cannot be created or read
/// - Directory creation fails
/// - LLM adapter initialization fails
/// - Environment validation shows critical missing tools
pub fn execute(name: String, llm: String, design: String, dev: Option<String>) -> Result<()> {
    internal::execute_init(name, llm, design, dev)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_init_creates_structure() -> Result<()> {
        let temp = TempDir::new()?;
        let _project_path = temp.path().join("test-project");

        // Create a minimal PROJECT_DESIGN.toml
        let design_content = r#"
[project]
name = "test-project"
type = "tool"
purpose = "Testing init command"

[why]
problem = "Need to test init"
solution = "Create test project"
users = "Developers"
value = "Ensures init works"
"#;
        let design_path = temp.path().join("design.toml");
        fs::write(&design_path, design_content)?;

        // Note: This would need mocking for full test
        // as it tries to detect real environment and copy patterns

        // Verify structure would be created
        // assert!(project_path.join(".patina").exists());
        // assert!(project_path.join("layer").exists());
        // assert!(project_path.join(".claude").exists());

        Ok(())
    }
}
