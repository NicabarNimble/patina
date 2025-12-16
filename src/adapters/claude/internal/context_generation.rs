//! Context file generation for Claude adapter
//!
//! Generates minimal .claude/CLAUDE.md files (~50-100 lines)
//! instead of the previous 1000+ line monsters.

use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::environment::Environment;

use super::paths;

/// Generate initial context during project setup
pub fn generate_initial_context(
    project_path: &Path,
    project_name: &str,
    environment: &Environment,
) -> Result<()> {
    let content = generate_minimal_context(project_name, environment);

    let output_path = paths::get_context_file_path(project_path);
    fs::write(output_path, content)?;

    Ok(())
}

/// Generate minimal context content
fn generate_minimal_context(project_name: &str, environment: &Environment) -> String {
    let mut content = String::new();

    // Header
    content.push_str(&format!("# {project_name} - Local Context\n\n"));
    content.push_str("Auto-generated context for this development environment.\n");
    content.push_str("See root `CLAUDE.md` for project instructions.\n\n");

    // Note about CLAUDE.local.md
    content.push_str(
        "💡 **Tip**: Create `CLAUDE.local.md` in project root for your personal notes.\n\n",
    );

    // Environment Summary (minimal)
    content.push_str("## Environment\n\n");
    content.push_str(&format!(
        "- **Platform**: {} ({})\n",
        environment.os, environment.arch
    ));
    content.push_str(&format!("- **Directory**: {}\n", environment.current_dir));

    // Only show critical tools that are available
    let critical_tools = ["cargo", "git", "docker", "go"];
    let available: Vec<_> = critical_tools
        .iter()
        .filter_map(|&tool| {
            environment
                .tools
                .get(tool)
                .filter(|info| info.available)
                .map(|_| tool)
        })
        .collect();

    if !available.is_empty() {
        content.push_str(&format!("- **Available**: {}\n", available.join(", ")));
    }
    content.push('\n');

    // MCP Tools guidance
    content.push_str("## Patina MCP Tools\n\n");
    content.push_str("This project has pre-indexed knowledge. Use these tools:\n\n");
    content.push_str("- **`scry`** - Search codebase. USE FIRST for any code question.\n");
    content
        .push_str("- **`context`** - Get design patterns. USE before architectural changes.\n\n");
    content.push_str("💡 Faster than manual file exploration - searches indexed symbols, git history, sessions.\n\n");

    // Patterns reference
    content.push_str("## Patterns\n\n");
    content.push_str("See `layer/` directory for design patterns and documentation.\n\n");

    // Session Commands (Git-integrated)
    content.push_str("## Session Commands\n\n");
    content.push_str("- `/session-start [name]` - Start session with Git tracking\n");
    content.push_str("- `/session-update` - Update progress with Git context\n");
    content.push_str("- `/session-note [insight]` - Capture insight\n");
    content.push_str("- `/session-end` - End session & distill learnings\n");
    content.push_str("- `/launch [branch]` - Create experimental branch\n\n");

    // Footer
    content.push_str(&format!(
        "---\n*Generated: {} | Patina v{}*\n",
        chrono::Utc::now().format("%Y-%m-%d"),
        env!("CARGO_PKG_VERSION")
    ));

    content
}
