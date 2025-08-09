//! Claude adapter implementation (Private)
//!
//! This module contains the actual implementation of the Claude adapter.
//! It's hidden behind the trait boundary, allowing us to refactor freely.

use crate::adapters::LLMAdapter;
use crate::environment::{Environment, LanguageInfo};
use crate::layer::{Pattern, PatternType};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

use super::templates::{self, paths, SessionScripts};
use super::versioning::{self, CLAUDE_ADAPTER_VERSION};

#[derive(Serialize, Deserialize)]
struct AdapterManifest {
    adapter: String,
    version: String,
    installed_at: String,
    files: HashMap<String, String>,
}

/// The actual Claude implementation (hidden from public API)
pub(super) struct ClaudeImpl;

impl ClaudeImpl {
    pub fn new() -> Self {
        ClaudeImpl
    }

    /// Get the base Claude directory path
    fn get_claude_path(&self, project_path: &Path) -> PathBuf {
        project_path.join(paths::ADAPTER_DIR)
    }

    /// Get various subdirectory paths
    fn get_mcp_path(&self, project_path: &Path) -> PathBuf {
        self.get_claude_path(project_path).join(paths::MCP_DIR)
    }

    fn get_commands_path(&self, project_path: &Path) -> PathBuf {
        self.get_claude_path(project_path).join(paths::COMMANDS_DIR)
    }

    fn get_bin_path(&self, project_path: &Path) -> PathBuf {
        self.get_claude_path(project_path).join(paths::BIN_DIR)
    }

    fn get_context_path(&self, project_path: &Path) -> PathBuf {
        self.get_claude_path(project_path).join(paths::CONTEXT_DIR)
    }

    fn get_sessions_path(&self, project_path: &Path) -> PathBuf {
        self.get_context_path(project_path)
            .join(paths::SESSIONS_DIR)
    }

    fn get_manifest_path(&self, project_path: &Path) -> PathBuf {
        self.get_claude_path(project_path)
            .join(paths::MANIFEST_FILE)
    }

    /// Create session management scripts
    fn create_session_scripts(&self, project_path: &Path) -> Result<()> {
        let bin_path = self.get_bin_path(project_path);
        fs::create_dir_all(&bin_path)?;

        // Create each script
        templates::create_executable_script(
            &bin_path.join("session-start.sh"),
            SessionScripts::session_start(),
        )?;

        templates::create_executable_script(
            &bin_path.join("session-update.sh"),
            SessionScripts::session_update(),
        )?;

        templates::create_executable_script(
            &bin_path.join("session-note.sh"),
            SessionScripts::session_note(),
        )?;

        templates::create_executable_script(
            &bin_path.join("session-end.sh"),
            SessionScripts::session_end(),
        )?;

        Ok(())
    }

    /// Create the adapter manifest for version tracking
    fn create_manifest(&self, project_path: &Path) -> Result<()> {
        let manifest = AdapterManifest {
            adapter: "claude".to_string(),
            version: CLAUDE_ADAPTER_VERSION.to_string(),
            installed_at: Utc::now().to_rfc3339(),
            files: HashMap::new(),
        };

        let manifest_path = self.get_manifest_path(project_path);
        let content = serde_json::to_string_pretty(&manifest)?;
        fs::write(manifest_path, content)?;
        Ok(())
    }

    /// Check current installed version
    fn get_installed_version(&self, project_path: &Path) -> Option<String> {
        let manifest_path = self.get_manifest_path(project_path);
        if !manifest_path.exists() {
            return None;
        }

        fs::read_to_string(manifest_path)
            .ok()
            .and_then(|content| serde_json::from_str::<AdapterManifest>(&content).ok())
            .map(|manifest| manifest.version)
    }

    fn format_development_section(&self, environment: &Environment) -> String {
        let mut section = String::from("### Development Tools\n\n");

        for (tool, info) in &environment.tools {
            let status = if info.available { "✓" } else { "✗" };
            let version_info = if let Some(ref version) = info.version {
                format!(" ({})", version)
            } else {
                String::new()
            };
            section.push_str(&format!("- **{}**: {}{}\n", tool, status, version_info));
        }

        section
    }

    fn format_languages_section(&self, environment: &Environment) -> String {
        let mut section = String::from("### Languages\n\n");

        let languages = vec![
            ("python", "Python"),
            ("node", "Node.js"),
            ("rust", "Rust"),
            ("go", "Go"),
            ("java", "Java"),
        ];

        for (key, name) in languages {
            if let Some(info) = environment.languages.get(key) {
                if let Some(ref version) = info.version {
                    section.push_str(&format!("- **{}**: {}\n", key, version));
                }
            }
        }

        section
    }

    fn format_pattern_section(&self, pattern_type: &str, patterns: &[Pattern]) -> String {
        let filtered: Vec<_> = patterns
            .iter()
            .filter(|p| match (&p.pattern_type, pattern_type) {
                (PatternType::Core, "core") => true,
                (PatternType::Topic(topic), "topic") => true,
                (PatternType::Project(_), "project") => true,
                _ => false,
            })
            .collect();

        if filtered.is_empty() {
            return String::new();
        }

        let mut section = match pattern_type {
            "core" => String::from("### Core Patterns\n\nUniversal principles that apply across the entire system:\n\n"),
            "topic" => String::from("### Topic Patterns\n\nDomain-specific knowledge and patterns:\n\n"),
            "project" => String::from("### Project-Specific Patterns\n\nPatterns specific to this project:\n\n"),
            _ => return String::new(),
        };

        // Group patterns by topic if applicable
        if pattern_type == "topic" {
            let mut by_topic: HashMap<String, Vec<&Pattern>> = HashMap::new();
            for pattern in filtered {
                if let PatternType::Topic(ref topic) = pattern.pattern_type {
                    by_topic.entry(topic.clone()).or_default().push(pattern);
                }
            }

            for (topic, patterns) in by_topic {
                section.push_str(&format!("#### Topic: {}\n\n", topic));
                for pattern in patterns {
                    section.push_str(&format!(
                        "##### {}\n\n{}\n---\n\n",
                        pattern.name, pattern.content
                    ));
                }
            }
        } else {
            for pattern in filtered {
                section.push_str(&format!(
                    "#### {}\n\n{}\n---\n\n",
                    pattern.name, pattern.content
                ));
            }
        }

        section
    }
}

// Implement the LLMAdapter trait
impl LLMAdapter for ClaudeImpl {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn init_project(
        &self,
        project_path: &Path,
        design: &Value,
        environment: &Environment,
    ) -> Result<()> {
        // Create Claude-specific directories
        let claude_path = self.get_claude_path(project_path);
        fs::create_dir_all(&claude_path)?;

        let mcp_path = self.get_mcp_path(project_path);
        fs::create_dir_all(&mcp_path)?;

        let context_path = self.get_context_path(project_path);
        fs::create_dir_all(&context_path)?;

        let sessions_path = self.get_sessions_path(project_path);
        fs::create_dir_all(&sessions_path)?;

        // Create session management scripts
        self.create_session_scripts(project_path)?;

        // Create README
        let readme_path = claude_path.join("README.md");
        fs::write(readme_path, templates::claude_readme_template())?;

        // Create .gitignore
        let gitignore_path = claude_path.join(".gitignore");
        fs::write(gitignore_path, templates::claude_gitignore_template())?;

        // Create manifest for version tracking
        self.create_manifest(project_path)?;

        // Generate initial CLAUDE.md
        self.generate_context(
            project_path,
            design
                .get("project")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("project"),
            &toml::to_string_pretty(design)?,
            &[],
            environment,
        )?;

        Ok(())
    }

    fn generate_context(
        &self,
        project_path: &Path,
        project_name: &str,
        design_content: &str,
        patterns: &[Pattern],
        environment: &Environment,
    ) -> Result<()> {
        let mut content = String::new();

        // Header
        content.push_str(&format!("# {} - Claude Context\n\n", project_name));
        content.push_str("This context is maintained by Patina and provides comprehensive project understanding.\n\n");

        // Table of Contents
        content.push_str("## Table of Contents\n\n");
        content.push_str("1. [Environment](#environment)\n");
        content.push_str("2. [Project Design](#project-design)\n");
        if !patterns.is_empty() {
            content.push_str("3. [Brain Patterns](#brain-patterns)\n");
            content.push_str("4. [Development Sessions](#development-sessions)\n");
            content.push_str("5. [Custom Commands](#custom-commands)\n");
            content.push_str("6. [Working Patterns](#working-patterns)\n");
        } else {
            content.push_str("3. [Development Sessions](#development-sessions)\n");
            content.push_str("4. [Custom Commands](#custom-commands)\n");
            content.push_str("5. [Working Patterns](#working-patterns)\n");
        }

        // Environment section
        content.push_str("\n## Environment\n\n");
        content.push_str(&format!(
            "- **OS**: {} ({})\n",
            environment.os, environment.arch
        ));
        content.push_str(&format!("- **Home**: {}\n", environment.home_dir));
        content.push_str(&format!(
            "- **Working Directory**: {}\n",
            project_path.display()
        ));
        content.push_str("\n");

        content.push_str(&self.format_development_section(environment));
        content.push_str("\n");
        content.push_str(&self.format_languages_section(environment));

        // Key environment variables
        content.push_str("\n### Key Environment Variables\n\n");
        for (key, value) in &environment.env_vars {
            if key == "PATH" || key.starts_with("RUST") || key.starts_with("CARGO") {
                continue; // Skip noisy vars
            }
            content.push_str(&format!("- **{}**: {}\n", key, value));
        }

        // Project Design section
        content.push_str("\n## Project Design\n\n");
        content.push_str(&design_content);
        content.push_str("\n");

        // Brain Patterns section (if patterns exist)
        if !patterns.is_empty() {
            content.push_str("\n## Brain Patterns\n\n");

            // Core patterns
            let core_section = self.format_pattern_section("core", patterns);
            if !core_section.is_empty() {
                content.push_str(&core_section);
            }

            // Topic patterns
            let topic_section = self.format_pattern_section("topic", patterns);
            if !topic_section.is_empty() {
                content.push_str(&topic_section);
            }

            // Project patterns
            let project_section = self.format_pattern_section("project", patterns);
            if !project_section.is_empty() {
                content.push_str(&project_section);
            }
        }

        // Development Sessions section
        content.push_str("\n## Development Sessions\n\n");
        let sessions_path = self.get_sessions_path(project_path);
        if sessions_path.exists() {
            content.push_str("Recent sessions:\n\n");
            let mut sessions = Vec::new();
            for entry in fs::read_dir(&sessions_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        sessions.push(name.to_string());
                    }
                }
            }
            sessions.sort();
            sessions.reverse(); // Most recent first

            for (i, session) in sessions.iter().take(5).enumerate() {
                content.push_str(&format!("{}. {}\n", i + 1, session));
            }

            // Add last session info if it exists
            let last_session_path = self.get_context_path(project_path).join("last-session.md");
            if last_session_path.exists() {
                content.push_str("\n### Most Recent Session\n\n");
                if let Ok(last_content) = fs::read_to_string(&last_session_path) {
                    content.push_str(&last_content);
                }
            }
        } else {
            content.push_str("No sessions recorded yet. Use `/session-start` to begin.\n");
        }

        // Custom Commands section
        content.push_str("\n## Custom Commands\n\n");
        content.push_str("Patina uses a two-phase session workflow: **Capture** (during work) → **Distill** (at session end)\n\n");
        content.push_str("### Available Commands\n\n");
        for (cmd, desc) in self.get_custom_commands() {
            content.push_str(&format!("- `{}` - {}\n", cmd, desc));
        }

        content.push_str("\n### Session Workflow\n\n");
        content.push_str("1. **Start**: `/session-start \"feature-name\"` - Creates timestamped session file (e.g., `20250122-1430-feature-name.md`)\n");
        content.push_str("2. **Work**: Make changes, explore code, have discussions\n");
        content.push_str(
            "3. **Update**: `/session-update` - Marks time spans for Claude to fill with context\n",
        );
        content.push_str("4. **Note**: `/session-note \"key insight\"` - Captures human insights (high-signal for distillation)\n");
        content.push_str(
            "5. **End**: `/session-end` - Triggers distillation into patterns and next steps\n",
        );

        content.push_str("\n### Key Concepts\n\n");
        content.push_str("- **Time-span tracking**: Updates show \"covering since 14:15\" to prevent context gaps\n");
        content.push_str("- **Git awareness**: Sessions capture branch, commits, and changes\n");
        content.push_str("- **Human priority**: Notes are treated as high-value insights\n");
        content.push_str("- **Pattern extraction**: Session-end prompts for reusable patterns\n");

        // Working Patterns section
        content.push_str("\n## Working Patterns\n\n");
        content.push_str("### Adding Knowledge\n");
        content.push_str("```bash\n");
        content.push_str("patina add <type> <name>  # Add pattern to session\n");
        content.push_str("patina commit -m \"message\"  # Commit patterns to brain\n");
        content.push_str("patina update  # Refresh this file\n");
        content.push_str("```\n\n");

        content.push_str("### Pattern Types\n");
        content.push_str("- `core` - Universal principles\n");
        content.push_str("- `topic` - Domain-specific knowledge\n");
        content.push_str("- `project` - Project-specific patterns\n");
        content.push_str("- `decision` - Architectural decisions\n");
        content.push_str("- `constraint` - Technical constraints\n");
        content.push_str("- `principle` - Guiding principles\n");

        // Footer
        content.push_str(&format!(
            "\n---\n\n*Generated by Patina on {}*\n",
            Utc::now().to_rfc3339()
        ));
        content.push_str("*Run `patina update` to refresh this context*\n");

        // Write the file
        let context_file = project_path.join(paths::CONTEXT_FILE);
        fs::write(context_file, content)?;

        Ok(())
    }

    fn update_context(
        &self,
        project_path: &Path,
        project_name: &str,
        design: &Value,
        patterns: &[Pattern],
        environment: &Environment,
    ) -> Result<()> {
        // For Claude, we regenerate the entire context
        let design_content = toml::to_string_pretty(design)?;
        self.generate_context(
            project_path,
            project_name,
            &design_content,
            patterns,
            environment,
        )
    }

    fn get_custom_commands(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("/session-start [name]", "Start a new development session"),
            ("/session-update", "Update session with rich context"),
            ("/session-note [insight]", "Add human insight to session"),
            (
                "/session-end",
                "End session with comprehensive distillation",
            ),
        ]
    }

    fn get_context_file_path(&self, project_path: &Path) -> PathBuf {
        project_path.join(paths::CONTEXT_FILE)
    }

    fn check_for_updates(&self, project_path: &Path) -> Result<Option<(String, String)>> {
        let installed = self.get_installed_version(project_path);

        match installed {
            Some(version) if version != CLAUDE_ADAPTER_VERSION => {
                Ok(Some((version, CLAUDE_ADAPTER_VERSION.to_string())))
            }
            _ => Ok(None),
        }
    }

    fn update_adapter_files(&self, project_path: &Path) -> Result<()> {
        // Update session scripts
        self.create_session_scripts(project_path)?;

        // Update manifest
        self.create_manifest(project_path)?;

        Ok(())
    }

    fn get_version_changes(&self, version: &str) -> Option<Vec<String>> {
        versioning::get_version_changes(version)
    }

    fn get_changelog_since(&self, from_version: &str) -> Vec<String> {
        versioning::get_changelog_since(from_version)
    }

    fn get_sessions_path(&self, project_path: &Path) -> Option<PathBuf> {
        Some(self.get_sessions_path(project_path))
    }

    fn version(&self) -> &'static str {
        CLAUDE_ADAPTER_VERSION
    }
}
