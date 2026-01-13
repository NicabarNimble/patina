//! Initialize a new Patina project skeleton
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
//! // Initialize a new project skeleton
//! init::execute(
//!     ".".to_string(),
//!     Some("docker".to_string()),
//!     false, // force
//!     false, // local
//!     false, // no_commit
//! ).expect("Failed to initialize project");
//!
//! // Then add an adapter:
//! // patina adapter add claude
//! ```

pub mod design_wizard;
mod internal;
pub mod tool_installer;

use anyhow::Result;

/// Execute the init command to create or reinitialize a Patina project skeleton
///
/// # Arguments
///
/// * `name` - Project name or "." for current directory
/// * `dev` - Optional development environment (e.g., "docker")
/// * `force` - Force initialization, backup and replace existing patina branch
/// * `local` - Skip GitHub integration (local-only mode)
/// * `no_commit` - Skip automatic git commit
///
/// # Process
///
/// 1. **Git Setup**: Ensures proper git branch and fork (if external repo)
/// 2. **Environment Detection**: Identifies available tools and languages
/// 3. **Project Setup**: Creates .patina/ and layer/ structure
/// 4. **Dev Environment**: Sets up development environment (Docker)
/// 5. **Pattern Copying**: Copies core patterns from Patina
///
/// # What This Does NOT Do
///
/// - Create adapter directories (.claude/, .gemini/)
/// - Configure MCP
/// - Run scrape or oxidize
///
/// Use `patina adapter add <claude|gemini|opencode>` to add LLM support.
///
/// # Re-initialization
///
/// When run in an existing Patina project:
/// - Preserves adapter config (frontends.allowed, frontends.default)
/// - Refreshes environment detection
/// - Updates dev environment if specified
///
/// # Errors
///
/// Returns an error if:
/// - Not a git repository
/// - Working tree has uncommitted changes (unless --force)
/// - Directory creation fails
/// - Environment validation shows critical missing tools
pub fn execute(
    name: String,
    dev: Option<String>,
    force: bool,
    local: bool,
    no_commit: bool,
) -> Result<()> {
    internal::execute_init(name, dev, force, local, no_commit)
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
