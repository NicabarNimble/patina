//! Internal implementation for OpenCode adapter

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters::templates;
use crate::environment::Environment;

/// Path constants for OpenCode adapter
const ADAPTER_DIR: &str = ".opencode";
const CONTEXT_FILE: &str = "AGENTS.md";
const MANAGED_CONTEXT_FILE: &str = "PATINA.md";
const MARKER_START: &str = "<!-- PATINA:START -->";
const MARKER_END: &str = "<!-- PATINA:END -->";

/// Initialize OpenCode project structure
pub fn init_project(
    project_path: &Path,
    project_name: &str,
    environment: &Environment,
) -> Result<()> {
    // Copy templates from central location (~/.patina/adapters/opencode/templates/)
    templates::copy_to_project("opencode", project_path)?;

    // Generate context file in .opencode/
    ensure_context_file(project_path, project_name, environment)?;

    Ok(())
}

/// Get context file path
pub fn get_context_file_path(project_path: &Path) -> PathBuf {
    project_path.join(ADAPTER_DIR).join(CONTEXT_FILE)
}

pub fn ensure_context_file(
    project_path: &Path,
    project_name: &str,
    environment: &Environment,
) -> Result<PathBuf> {
    let opencode_path = project_path.join(ADAPTER_DIR);
    fs::create_dir_all(&opencode_path)?;
    let managed_context_path = opencode_path.join(MANAGED_CONTEXT_FILE);
    fs::write(
        &managed_context_path,
        generate_managed_context(project_name, environment),
    )?;

    let context_path = opencode_path.join(CONTEXT_FILE);
    if !context_path.exists() {
        fs::write(
            &context_path,
            generate_context_shell(project_name, &managed_context_path),
        )?;
        return Ok(context_path);
    }

    let existing = fs::read_to_string(&context_path)?;
    if has_managed_markers(&existing) {
        fs::write(
            &context_path,
            replace_managed_section(
                &existing,
                &managed_shell_section(project_name, &managed_context_path),
            ),
        )?;
        return Ok(context_path);
    }

    Ok(managed_context_path)
}

/// Generate Patina-managed context for OpenCode.
fn generate_managed_context(project_name: &str, environment: &Environment) -> String {
    let mut content = String::new();

    // Header
    content.push_str(&format!("# {project_name} - Patina OpenCode Context\n\n"));
    content.push_str("Patina-managed context fragment for OpenCode.\n");
    content
        .push_str("User-owned instructions belong in `.opencode/AGENTS.md` or project docs.\n\n");

    // Environment
    content.push_str("## Environment\n\n");
    content.push_str(&format!(
        "- **Platform**: {} ({})\n",
        environment.os, environment.arch
    ));
    content.push_str(&format!("- **Directory**: {}\n", environment.current_dir));

    // Available tools
    let tools = ["cargo", "git", "docker", "python"];
    let available: Vec<_> = tools
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
        content.push_str(&format!("- **Tools**: {}\n", available.join(", ")));
    }
    content.push('\n');

    // Patterns
    content.push_str("## Patterns\n\n");
    content.push_str("See files in `layer/` directory for patterns and documentation.\n\n");

    // MCP Tools
    content.push_str("## MCP Tools (Use These First)\n\n");
    content.push_str(
        "Patina's authoritative interface surface is MCP, not shell-output scraping.\n\n",
    );
    content.push_str("**Core discovery**\n");
    content.push_str(
        "- `context` - architecture, beliefs, and project patterns before non-trivial changes\n",
    );
    content.push_str("- `scry` - codebase knowledge search when you need implementation context\n");
    content.push_str("- `assay` - exact structural/code inventory questions\n\n");
    content.push_str("**Session workflow**\n");
    content.push_str(
        "- `session.start` - start a new Patina session and get the durable artifact path\n",
    );
    content
        .push_str("- `session.update` - append a git-aware update to the current live session\n");
    content.push_str(
        "- `session.end` - archive the live session and update the last-session pointer\n",
    );
    content.push_str(
        "- `session.list` - inspect active/stale/recent sessions if selection is ambiguous\n\n",
    );
    content.push_str("**Spec workflow**\n");
    content.push_str("- `spec.next` - decide what should be worked next\n");
    content.push_str("- `spec.list` / `spec.ready` / `spec.blocked` - navigate the queue\n");
    content.push_str("- `spec.show` - load spec context before coding\n");
    content.push_str("- `spec.check` - verify exit criteria truthfully\n");
    content.push_str("- `spec.create` / `spec.set` - mutate spec state only when the workflow actually needs it\n\n");
    content.push_str(
        "Prefer MCP for session/spec actions. Use CLI `--json` only when MCP is unavailable.\n\n",
    );

    // Footer
    content.push_str(&format!(
        "---\n*Generated by Patina v{} | Interface: OpenCode*\n",
        env!("CARGO_PKG_VERSION")
    ));

    content
}

fn generate_context_shell(project_name: &str, managed_context_path: &Path) -> String {
    format!(
        "# {project_name} - OpenCode Context\n\n\
Project-specific instructions for OpenCode belong here.\n\n\
{}\n",
        managed_shell_section(project_name, managed_context_path)
    )
}

fn managed_shell_section(project_name: &str, managed_context_path: &Path) -> String {
    let managed_path = managed_context_path
        .file_name()
        .map(|name| format!(".opencode/{}", name.to_string_lossy()))
        .unwrap_or_else(|| ".opencode/PATINA.md".to_string());

    format!(
        "{MARKER_START}\n## Patina Managed Context\n\n\
Read `{managed_path}` before non-trivial changes.\n\
Use root `OPENCODE.md` for Patina session/bootstrap workflow.\n\
This section can be regenerated safely; keep custom instructions outside it.\n\
\n*Managed for {project_name} by Patina*\n{MARKER_END}"
    )
}

fn has_managed_markers(content: &str) -> bool {
    content.contains(MARKER_START) && content.contains(MARKER_END)
}

fn replace_managed_section(content: &str, managed_section: &str) -> String {
    if let (Some(start), Some(end)) = (content.find(MARKER_START), content.find(MARKER_END)) {
        if start < end {
            let before = &content[..start];
            let after = &content[end + MARKER_END.len()..];
            return format!("{before}{managed_section}{after}");
        }
    }

    content.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn generated_context_surfaces_session_and_spec_mcp_tools() {
        let environment = Environment {
            os: "macos".to_string(),
            arch: "arm64".to_string(),
            home_dir: "/tmp".to_string(),
            current_dir: "/tmp/project".to_string(),
            tools: Default::default(),
            languages: Default::default(),
            env_vars: Default::default(),
        };

        let content = generate_managed_context("patina", &environment);
        assert!(content.contains("session.start"));
        assert!(content.contains("session.update"));
        assert!(content.contains("session.end"));
        assert!(content.contains("spec.next"));
        assert!(content.contains("spec.check"));
    }

    #[test]
    fn preserves_user_owned_shell_without_markers() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path();
        let adapter_dir = project_root.join(".opencode");
        fs::create_dir_all(&adapter_dir).unwrap();
        let shell_path = adapter_dir.join("AGENTS.md");
        fs::write(&shell_path, "# User shell\n\nKeep this.\n").unwrap();

        let environment = Environment {
            os: "macos".to_string(),
            arch: "arm64".to_string(),
            home_dir: "/tmp".to_string(),
            current_dir: "/tmp/project".to_string(),
            tools: Default::default(),
            languages: Default::default(),
            env_vars: Default::default(),
        };

        let context_path = ensure_context_file(project_root, "patina", &environment).unwrap();

        assert_eq!(context_path, adapter_dir.join("PATINA.md"));
        assert_eq!(
            fs::read_to_string(&shell_path).unwrap(),
            "# User shell\n\nKeep this.\n"
        );
        assert!(fs::read_to_string(adapter_dir.join("PATINA.md"))
            .unwrap()
            .contains("Patina-managed context fragment"));
    }

    #[test]
    fn refreshes_patina_owned_shell_section() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path();
        let adapter_dir = project_root.join(".opencode");
        fs::create_dir_all(&adapter_dir).unwrap();
        let shell_path = adapter_dir.join("AGENTS.md");
        fs::write(
            &shell_path,
            "# User shell\n\n<!-- PATINA:START -->\nstale\n<!-- PATINA:END -->\n",
        )
        .unwrap();

        let environment = Environment {
            os: "macos".to_string(),
            arch: "arm64".to_string(),
            home_dir: "/tmp".to_string(),
            current_dir: "/tmp/project".to_string(),
            tools: Default::default(),
            languages: Default::default(),
            env_vars: Default::default(),
        };

        let context_path = ensure_context_file(project_root, "patina", &environment).unwrap();
        let updated_shell = fs::read_to_string(&shell_path).unwrap();

        assert_eq!(context_path, shell_path);
        assert!(updated_shell.contains("## Patina Managed Context"));
        assert!(updated_shell.contains("OPENCODE.md"));
        assert!(updated_shell.contains("# User shell"));
    }
}
