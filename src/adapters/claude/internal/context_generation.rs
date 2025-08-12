//! Context file generation for Claude adapter
//! 
//! Generates minimal .claude/CLAUDE.md files (~50-100 lines)
//! instead of the previous 1000+ line monsters.

use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::environment::Environment;
use crate::layer::{Pattern, PatternType};
use toml::Value;

use super::paths;

/// Generate initial context during project setup
pub fn generate_initial_context(
    project_path: &Path,
    design: &Value,
    environment: &Environment,
) -> Result<()> {
    let project_name = extract_project_name(design);
    
    // For initial setup, we don't have patterns yet
    let patterns = Vec::new();
    
    write_context_file(project_path, &project_name, &patterns, environment)
}

/// Write the context file with current patterns and environment
pub fn write_context_file(
    project_path: &Path,
    project_name: &str,
    patterns: &[Pattern],
    environment: &Environment,
) -> Result<()> {
    let content = generate_minimal_context(project_name, patterns, environment);
    
    let output_path = paths::get_context_file_path(project_path);
    fs::write(output_path, content)?;
    
    Ok(())
}

/// Generate minimal context content
fn generate_minimal_context(
    project_name: &str,
    patterns: &[Pattern],
    environment: &Environment,
) -> String {
    let mut content = String::new();

    // Header
    content.push_str(&format!("# {project_name} - Local Context\n\n"));
    content.push_str("Auto-generated context for this development environment.\n");
    content.push_str("See root `CLAUDE.md` for project instructions.\n\n");
    
    // Note about CLAUDE.local.md
    content.push_str("💡 **Tip**: Create `CLAUDE.local.md` in project root for your personal notes.\n\n");

    // Environment Summary (minimal)
    content.push_str("## Environment\n\n");
    content.push_str(&format!("- **Platform**: {} ({})\n", environment.os, environment.arch));
    content.push_str(&format!("- **Directory**: {}\n", environment.current_dir));
    
    // Only show critical tools that are available
    let critical_tools = ["cargo", "git", "docker", "go", "dagger"];
    let available: Vec<_> = critical_tools
        .iter()
        .filter_map(|&tool| {
            environment.tools.get(tool)
                .filter(|info| info.available)
                .map(|_| tool)
        })
        .collect();
    
    if !available.is_empty() {
        content.push_str(&format!("- **Available**: {}\n", available.join(", ")));
    }
    content.push('\n');

    // Core Patterns Reference (no content, just pointers)
    let core_patterns: Vec<_> = patterns
        .iter()
        .filter(|p| matches!(p.pattern_type, PatternType::Core))
        .collect();
    
    if !core_patterns.is_empty() {
        content.push_str("## Core Patterns\n\n");
        content.push_str("Reference files in `layer/core/`:\n\n");
        
        for pattern in core_patterns {
            let summary = extract_summary(&pattern.content);
            content.push_str(&format!(
                "- **{}** → `layer/core/{}.md`\n  {}\n",
                pattern.name,
                pattern.name,
                summary
            ));
        }
        content.push('\n');
    }

    // Session Commands (ultra-minimal)
    content.push_str("## Session Commands\n\n");
    content.push_str("- `/session-start [name]` - Begin session\n");
    content.push_str("- `/session-update` - Mark progress\n");
    content.push_str("- `/session-note` - Capture insight\n");
    content.push_str("- `/session-end` - End & distill\n\n");

    // Footer
    content.push_str(&format!(
        "---\n*Generated: {} | Patina v{}*\n",
        chrono::Utc::now().format("%Y-%m-%d"),
        env!("CARGO_PKG_VERSION")
    ));

    content
}

/// Extract a one-line summary from pattern content
fn extract_summary(content: &str) -> String {
    // Skip frontmatter
    let mut past_frontmatter = false;
    let mut frontmatter_count = 0;
    
    for line in content.lines() {
        if line == "---" {
            frontmatter_count += 1;
            if frontmatter_count == 2 {
                past_frontmatter = true;
            }
            continue;
        }
        
        if past_frontmatter || frontmatter_count == 0 {
            // Look for Purpose line
            if line.starts_with("**Purpose:") {
                return line
                    .trim_start_matches("**Purpose:")
                    .trim_start_matches("**")
                    .trim()
                    .to_string();
            }
            
            // Or use first non-empty, non-header line
            if !line.trim().is_empty() && !line.starts_with('#') {
                let summary = line.trim();
                if summary.len() > 80 {
                    return format!("{}...", &summary[..77]);
                }
                return summary.to_string();
            }
        }
    }
    
    "See file for details".to_string()
}

/// Extract project name from design TOML
fn extract_project_name(design: &Value) -> String {
    design
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("project")
        .to_string()
}