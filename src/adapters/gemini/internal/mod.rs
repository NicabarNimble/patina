//! Internal implementation for Gemini adapter

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters::{launch, templates};
use crate::environment::Environment;

/// Path constants for Gemini adapter
const ADAPTER_DIR: &str = ".gemini";
const CONTEXT_FILE: &str = "GEMINI.md";
const MANAGED_CONTEXT_FILE: &str = "PATINA.md";
const MARKER_START: &str = "<!-- PATINA:START -->";
const MARKER_END: &str = "<!-- PATINA:END -->";

/// Initialize Gemini project structure
pub fn init_project(
    project_path: &Path,
    project_name: &str,
    environment: &Environment,
) -> Result<()> {
    // Copy templates from central location (~/.patina/adapters/gemini/templates/)
    templates::copy_to_project("gemini", project_path)?;

    // Generate context file in .gemini/
    ensure_context_file(project_path, project_name, environment)?;

    Ok(())
}

// Removed pattern-based context generation
// TODO: Implement real pattern extraction

/// Get context file path
pub fn get_context_file_path(project_path: &Path) -> PathBuf {
    project_path.join(ADAPTER_DIR).join(CONTEXT_FILE)
}

pub fn ensure_context_file(
    project_path: &Path,
    project_name: &str,
    environment: &Environment,
) -> Result<PathBuf> {
    let gemini_path = project_path.join(ADAPTER_DIR);
    fs::create_dir_all(&gemini_path)?;
    let managed_context_path = gemini_path.join(MANAGED_CONTEXT_FILE);
    let mcp_available = launch::native_interface_mcp_available("gemini")?;
    fs::write(
        &managed_context_path,
        generate_managed_context(project_name, environment, mcp_available),
    )?;

    let context_path = gemini_path.join(CONTEXT_FILE);
    if !context_path.exists() {
        fs::write(
            &context_path,
            generate_context_shell(project_name, &managed_context_path),
        )?;
        return Ok(context_path);
    }

    let existing = fs::read_to_string(&context_path)?;
    let shell = if has_managed_markers(&existing) {
        replace_managed_section(
            &existing,
            &managed_shell_section(project_name, &managed_context_path),
        )
    } else {
        generate_context_shell(project_name, &managed_context_path)
    };
    fs::write(&context_path, shell)?;

    Ok(context_path)
}

/// Generate Patina-managed context for Gemini.
fn generate_managed_context(
    project_name: &str,
    environment: &Environment,
    mcp_available: bool,
) -> String {
    let mut content = String::new();

    // Header
    content.push_str(&format!("# {project_name} - Patina Gemini Context\n\n"));
    content.push_str("Patina-managed context fragment for Gemini CLI.\n");
    content.push_str("User-owned instructions belong in `.gemini/GEMINI.md` or project docs.\n\n");

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

    // TODO: Implement real pattern discovery
    content.push_str("## Patterns\n\n");
    content.push_str("See files in `layer/` directory for patterns and documentation.\n\n");

    if mcp_available {
        content.push_str("## MCP Tools (Available In This Runtime)\n\n");
        content.push_str(
            "Patina MCP is configured for this Gemini runtime. Prefer MCP over shell-output scraping.\n\n",
        );
        content.push_str("**Core discovery**\n");
        content.push_str(
            "- `context` - architecture, beliefs, and project patterns before non-trivial changes\n",
        );
        content.push_str(
            "- `scry` - codebase knowledge search when you need implementation context\n",
        );
        content.push_str("- `assay` - exact structural/code inventory questions\n\n");
        content.push_str("**Session workflow**\n");
        content.push_str(
            "- `session.start` - start a new Patina session and get the durable artifact path\n",
        );
        content.push_str(
            "- `session.update` - append a git-aware update to the current live session\n",
        );
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
        content.push_str(
            "- `spec.create` / `spec.set` - mutate spec state only when the workflow actually needs it\n\n",
        );
        content.push_str(
            "If a session command cannot use MCP at runtime, fall back to `patina ai session ... --json` rather than `patina session ... --json`.\n\n",
        );
    } else {
        content.push_str("## Session Surface (MCP Unavailable Here)\n\n");
        content.push_str(
            "Patina MCP is not configured for this Gemini runtime. Do not assume `session.*`, `spec.*`, `context`, `scry`, or `assay` MCP tools exist here.\n\n",
        );
        content.push_str("Use the native machine-readable session fallback instead:\n");
        content.push_str("- `patina ai session start --json --adapter gemini \"<title>\"`\n");
        content.push_str("- `patina ai session update --json`\n");
        content.push_str("- `patina ai session end --json`\n");
        content.push_str("- `patina ai session list --json` when selection is ambiguous\n\n");
    }

    // Footer
    content.push_str(&format!(
        "---\n*Generated by Patina v{} | Interface: Gemini CLI*\n",
        env!("CARGO_PKG_VERSION")
    ));

    content
}

fn generate_context_shell(project_name: &str, managed_context_path: &Path) -> String {
    format!(
        "# {project_name} - Gemini Context\n\n\
Project-specific instructions for Gemini belong here.\n\n\
{}\n",
        managed_shell_section(project_name, managed_context_path)
    )
}

fn managed_shell_section(project_name: &str, managed_context_path: &Path) -> String {
    let managed_path = managed_context_path
        .file_name()
        .map(|name| format!(".gemini/{}", name.to_string_lossy()))
        .unwrap_or_else(|| ".gemini/PATINA.md".to_string());

    format!(
        "{MARKER_START}\n## Patina Managed Context\n\n\
Read `{managed_path}` before non-trivial changes.\n\
Use root `GEMINI.md` for Patina session/bootstrap workflow.\n\
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

    fn fake_environment() -> Environment {
        Environment {
            os: "macos".to_string(),
            arch: "arm64".to_string(),
            home_dir: "/tmp".to_string(),
            current_dir: "/tmp/project".to_string(),
            tools: Default::default(),
            languages: Default::default(),
            env_vars: Default::default(),
        }
    }

    #[test]
    fn managed_context_includes_session_and_spec_tools() {
        let content = generate_managed_context("patina", &fake_environment(), true);
        assert!(content.contains("session.start"));
        assert!(content.contains("session.update"));
        assert!(content.contains("session.end"));
        assert!(content.contains("spec.next"));
        assert!(content.contains("spec.check"));
    }

    #[test]
    fn rewrites_unmanaged_shell_without_markers() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path();
        let adapter_dir = project_root.join(".gemini");
        fs::create_dir_all(&adapter_dir).unwrap();
        let shell_path = adapter_dir.join("GEMINI.md");
        fs::write(&shell_path, "# User shell\n\nKeep this.\n").unwrap();

        let context_path =
            ensure_context_file(project_root, "patina", &fake_environment()).unwrap();

        assert_eq!(context_path, shell_path);
        let rewritten = fs::read_to_string(&shell_path).unwrap();
        assert!(rewritten.contains("## Patina Managed Context"));
        assert!(rewritten.contains("Project-specific instructions for Gemini belong here."));
        assert!(fs::read_to_string(adapter_dir.join("PATINA.md"))
            .unwrap()
            .contains("Patina-managed context fragment"));
    }

    #[test]
    fn refreshes_patina_owned_shell_section() {
        let temp = TempDir::new().unwrap();
        let project_root = temp.path();
        let adapter_dir = project_root.join(".gemini");
        fs::create_dir_all(&adapter_dir).unwrap();
        let shell_path = adapter_dir.join("GEMINI.md");
        fs::write(
            &shell_path,
            "# User shell\n\n<!-- PATINA:START -->\nstale\n<!-- PATINA:END -->\n",
        )
        .unwrap();

        let context_path =
            ensure_context_file(project_root, "patina", &fake_environment()).unwrap();
        let updated_shell = fs::read_to_string(&shell_path).unwrap();

        assert_eq!(context_path, shell_path);
        assert!(updated_shell.contains("## Patina Managed Context"));
        assert!(updated_shell.contains("# User shell"));
    }
}
