//! Template extraction and management
//!
//! Handles extracting embedded templates to ~/.patina/interfaces/
//! and copying templates to projects.
//!
//! Templates are embedded at compile time and extracted on first run.
//! This allows user customization of templates in ~/.patina/interfaces/.

use anyhow::Result;
use std::borrow::Cow;
use std::fs;
use std::path::Path;

use crate::interface::interface_bundle;
use crate::mother::skills::{self, SkillContent, SkillContentMode};
use crate::paths;

// =============================================================================
// Thin wrapper scripts — generated per interface
// =============================================================================
// These keep the native session backend behind local interface scripts so the
// command markdown/TOML can stay workflow-oriented.

fn wrapper_start(interface: &str) -> String {
    format!(
        "#!/bin/bash\nexec env PATINA_AI_INTERFACE={interface} patina ai session start --json --interface {interface} \"$@\"\n"
    )
}

fn wrapper_update(interface: &str) -> String {
    format!(
        "#!/bin/bash\nexec env PATINA_AI_INTERFACE={interface} patina ai session update --json \"$@\"\n"
    )
}

fn wrapper_note(interface: &str) -> String {
    format!("#!/bin/bash\nexec env PATINA_AI_INTERFACE={interface} patina ai session note \"$@\"\n")
}

fn wrapper_end(interface: &str) -> String {
    format!(
        "#!/bin/bash\nexec env PATINA_AI_INTERFACE={interface} patina ai session end --json --commit \"$@\"\n"
    )
}

// =============================================================================
// Public API
// =============================================================================

/// Extract all templates to ~/.patina/interfaces/
///
/// Called during first-run setup. Creates the full template structure
/// for all supported interfaces.
pub fn install_all(interfaces_dir: &Path) -> Result<()> {
    install_claude_templates(interfaces_dir)?;
    install_gemini_templates(interfaces_dir)?;
    install_opencode_templates(interfaces_dir)?;
    install_pi_templates(interfaces_dir)?;
    write_interface_registry(interfaces_dir)?;
    Ok(())
}

/// Copy interface templates to project
///
/// Copies the interface-specific directory (.claude/, .gemini/) from
/// central templates to the project.
pub fn copy_to_project(interface_name: &str, project_path: &Path) -> Result<()> {
    let bundle = interface_bundle(interface_name)?;
    let iface_dir = paths::project::managed_interface_dir(project_path, interface_name);
    fs::create_dir_all(&iface_dir)?;
    fs::create_dir_all(iface_dir.join("bin"))?;

    write_executable(
        &iface_dir.join("bin/session-start.sh"),
        &wrapper_start(interface_name),
    )?;
    write_executable(
        &iface_dir.join("bin/session-update.sh"),
        &wrapper_update(interface_name),
    )?;
    write_executable(
        &iface_dir.join("bin/session-note.sh"),
        &wrapper_note(interface_name),
    )?;
    write_executable(
        &iface_dir.join("bin/session-end.sh"),
        &wrapper_end(interface_name),
    )?;

    let declared_skills = bundle
        .skills
        .as_ref()
        .map(|skills| skills.include.as_slice())
        .unwrap_or(&[]);

    for skill in declared_skills {
        let content = skills::skill_content(interface_name, skill).ok_or_else(|| {
            anyhow::anyhow!(
                "Mother skill '{}' is not available for interface '{}'. Remove it from skills.include or add it to mother::skills.",
                skill,
                interface_name
            )
        })?;
        write_skill_content(&iface_dir, interface_name, &content)?;
    }

    Ok(())
}

/// Check if templates are installed for an interface
pub fn templates_installed(interface_name: &str) -> bool {
    let templates_dir = paths::interfaces_dir()
        .join(interface_name)
        .join("templates");
    templates_dir.exists()
}

// =============================================================================
// Claude Templates Installation
// =============================================================================

fn install_claude_templates(interfaces_dir: &Path) -> Result<()> {
    install_interface_templates(interfaces_dir, "claude", true)
}

// =============================================================================
// Gemini Templates Installation
// =============================================================================

fn install_gemini_templates(interfaces_dir: &Path) -> Result<()> {
    install_interface_templates(interfaces_dir, "gemini", true)
}

// =============================================================================
// OpenCode Templates Installation
// =============================================================================

fn install_opencode_templates(interfaces_dir: &Path) -> Result<()> {
    install_interface_templates(interfaces_dir, "opencode", true)
}

fn install_pi_templates(interfaces_dir: &Path) -> Result<()> {
    install_interface_templates(interfaces_dir, "pi", false)?;
    let context_dir = interfaces_dir.join("pi/templates/.pi/context");
    fs::create_dir_all(&context_dir)?;
    let review = skills::skill_content("opencode", "patina-review")
        .and_then(|content| content.files.first().map(|file| file.bytes))
        .unwrap_or_default()
        .replace(".opencode/", ".pi/");
    fs::write(context_dir.join("agent-instructions.md"), review)?;
    Ok(())
}

const INSTALL_SKILLS: &[&str] = &[
    "session-start",
    "session-update",
    "session-note",
    "session-end",
    "patina-review",
    "spec",
    "epistemic-beliefs",
];

fn install_interface_templates(
    interfaces_dir: &Path,
    interface_name: &str,
    fail_closed: bool,
) -> Result<()> {
    let interface_dir = interfaces_dir
        .join(interface_name)
        .join("templates")
        .join(format!(".{}", interface_name));
    fs::create_dir_all(&interface_dir)?;
    fs::create_dir_all(interface_dir.join("bin"))?;

    write_executable(
        &interface_dir.join("bin/session-start.sh"),
        &wrapper_start(interface_name),
    )?;
    write_executable(
        &interface_dir.join("bin/session-update.sh"),
        &wrapper_update(interface_name),
    )?;
    write_executable(
        &interface_dir.join("bin/session-note.sh"),
        &wrapper_note(interface_name),
    )?;
    write_executable(
        &interface_dir.join("bin/session-end.sh"),
        &wrapper_end(interface_name),
    )?;

    for skill in INSTALL_SKILLS {
        match skills::skill_content(interface_name, skill) {
            Some(content) => write_skill_content(&interface_dir, interface_name, &content)?,
            None if fail_closed => {
                anyhow::bail!(
                    "Mother skill '{}' is not available for interface '{}'.",
                    skill,
                    interface_name
                )
            }
            None => {}
        }
    }

    Ok(())
}

fn write_interface_registry(interfaces_dir: &Path) -> Result<()> {
    fs::create_dir_all(interfaces_dir)?;
    fs::write(
        interfaces_dir.join("registry.toml"),
        crate::interface::builtin_registry_toml(),
    )?;
    Ok(())
}

// =============================================================================
// Helpers
// =============================================================================

fn write_skill_content(
    base_dir: &Path,
    interface_name: &str,
    content: &SkillContent,
) -> Result<()> {
    for file in content.files {
        let path = base_dir.join(file.projection_file);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let body: Cow<'_, str> = if interface_name == "pi" {
            Cow::Owned(file.bytes.replace(".opencode/", ".pi/"))
        } else {
            Cow::Borrowed(file.bytes)
        };

        match file.mode {
            SkillContentMode::Executable => write_executable(&path, &body)?,
            SkillContentMode::Markdown | SkillContentMode::Toml => fs::write(&path, body.as_ref())?,
        }
    }
    Ok(())
}

/// Write a file and make it executable
fn write_executable(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_claude_templates_compile() {
        let start = skills::skill_content("claude", "session-start").unwrap();
        assert!(start.files[0].bytes.contains("session-start.sh"));
        let end = skills::skill_content("claude", "session-end").unwrap();
        assert!(end.files[0].bytes.contains("session-end.sh"));
        let spec = skills::skill_content("claude", "spec").unwrap();
        assert!(spec.files[0].bytes.contains("patina spec"));
        let epistemic = skills::skill_content("claude", "epistemic-beliefs").unwrap();
        assert!(epistemic.files.iter().any(|f| f.bytes.contains("belief")));
        assert!(epistemic
            .files
            .iter()
            .any(|f| f.bytes.contains("create-belief")));
    }

    #[test]
    fn test_gemini_templates_compile() {
        let start = skills::skill_content("gemini", "session-start").unwrap();
        assert!(start.files[0].bytes.contains("session-start.sh"));
        let spec = skills::skill_content("gemini", "spec").unwrap();
        assert!(spec.files[0].bytes.contains("patina spec"));
        let beliefs = skills::skill_content("gemini", "epistemic-beliefs").unwrap();
        assert!(beliefs
            .files
            .iter()
            .any(|f| f.bytes.contains("create-belief.sh")));
    }

    #[test]
    fn test_opencode_templates_compile() {
        let start = skills::skill_content("opencode", "session-start").unwrap();
        assert!(start.files[0]
            .bytes
            .contains(".opencode/bin/session-start.sh"));
        let update = skills::skill_content("opencode", "session-update").unwrap();
        assert!(update.files[0]
            .bytes
            .contains(".opencode/bin/session-update.sh"));
        let end = skills::skill_content("opencode", "session-end").unwrap();
        assert!(end.files[0].bytes.contains(".opencode/bin/session-end.sh"));
        let spec = skills::skill_content("opencode", "spec").unwrap();
        assert!(spec.files[0].bytes.contains("patina spec"));
        let beliefs = skills::skill_content("opencode", "epistemic-beliefs").unwrap();
        assert!(beliefs
            .files
            .iter()
            .any(|f| f.bytes.contains("create-belief.sh")));
    }

    #[test]
    fn test_wrapper_scripts_content() {
        let start = wrapper_start("claude");
        let update = wrapper_update("opencode");
        let note = wrapper_note("gemini");
        let end = wrapper_end("claude");

        assert!(start.contains("PATINA_AI_INTERFACE=claude"));
        assert!(start.contains("patina ai session start --json --interface claude"));
        assert!(update.contains("PATINA_AI_INTERFACE=opencode"));
        assert!(update.contains("patina ai session update --json"));
        assert!(note.contains("PATINA_AI_INTERFACE=gemini"));
        assert!(note.contains("patina ai session note"));
        assert!(end.contains("PATINA_AI_INTERFACE=claude"));
        assert!(end.contains("patina ai session end --json"));
        assert!(end.contains("--commit"));
    }

    #[test]
    fn test_install_claude_templates() {
        let temp = TempDir::new().unwrap();
        install_claude_templates(temp.path()).unwrap();

        // Templates install to .claude/ structure for copy_to_project()
        let templates_dir = temp.path().join("claude/templates");
        assert!(templates_dir.join(".claude/bin/session-start.sh").exists());
        assert!(templates_dir
            .join(".claude/commands/session-start.md")
            .exists());
        assert!(templates_dir
            .join(".claude/commands/patina-review.md")
            .exists());
        assert!(templates_dir.join(".claude/commands/spec.md").exists());
        // Deprecated commands should not exist
        assert!(!templates_dir.join(".claude/bin/launch.sh").exists());
        assert!(!templates_dir.join(".claude/bin/persona-start.sh").exists());

        // Wrapper scripts should forward to the native AI session backend
        let wrapper =
            fs::read_to_string(templates_dir.join(".claude/bin/session-start.sh")).unwrap();
        assert!(wrapper.contains("PATINA_AI_INTERFACE=claude"));
        assert!(wrapper.contains("patina ai session start --json --interface claude"));

        // Skills should be installed
        assert!(templates_dir
            .join(".claude/skills/epistemic-beliefs/SKILL.md")
            .exists());
        assert!(templates_dir
            .join(".claude/skills/epistemic-beliefs/scripts/create-belief.sh")
            .exists());
        assert!(templates_dir
            .join(".claude/skills/epistemic-beliefs/references/belief-example.md")
            .exists());
        assert!(templates_dir
            .join(".claude/skills/epistemic-beliefs/references/verification-schema.md")
            .exists());
    }

    #[test]
    fn test_install_gemini_templates() {
        let temp = TempDir::new().unwrap();
        install_gemini_templates(temp.path()).unwrap();

        // Templates install to .gemini/ structure for copy_to_project()
        let templates_dir = temp.path().join("gemini/templates");
        assert!(templates_dir.join(".gemini/bin/session-start.sh").exists());
        assert!(templates_dir
            .join(".gemini/commands/session-start.toml")
            .exists());
        assert!(templates_dir.join(".gemini/commands/spec.toml").exists());
        assert!(templates_dir
            .join(".gemini/commands/epistemic-beliefs.toml")
            .exists());
        assert!(templates_dir.join(".gemini/bin/create-belief.sh").exists());

        // Wrapper scripts should forward to the native AI session backend
        let wrapper =
            fs::read_to_string(templates_dir.join(".gemini/bin/session-start.sh")).unwrap();
        assert!(wrapper.contains("PATINA_AI_INTERFACE=gemini"));
        assert!(wrapper.contains("patina ai session start --json --interface gemini"));
    }

    #[test]
    fn test_install_opencode_templates() {
        let temp = TempDir::new().unwrap();
        install_opencode_templates(temp.path()).unwrap();

        let templates_dir = temp.path().join("opencode/templates");
        assert!(templates_dir
            .join(".opencode/bin/session-start.sh")
            .exists());
        assert!(templates_dir
            .join(".opencode/commands/session-start.md")
            .exists());
        assert!(templates_dir.join(".opencode/commands/spec.md").exists());
        assert!(templates_dir
            .join(".opencode/commands/epistemic-beliefs.md")
            .exists());
        assert!(templates_dir
            .join(".opencode/bin/create-belief.sh")
            .exists());

        let session_start =
            fs::read_to_string(templates_dir.join(".opencode/commands/session-start.md")).unwrap();
        assert!(session_start.contains(".opencode/bin/session-start.sh"));
        assert!(session_start.contains("spec.check"));

        let session_update =
            fs::read_to_string(templates_dir.join(".opencode/commands/session-update.md")).unwrap();
        assert!(session_update.contains(".opencode/bin/session-update.sh"));
        assert!(!session_update.contains("session.update"));

        let session_end =
            fs::read_to_string(templates_dir.join(".opencode/commands/session-end.md")).unwrap();
        assert!(session_end.contains(".opencode/bin/session-end.sh"));
        assert!(!session_end.contains("session.end"));

        let spec = fs::read_to_string(templates_dir.join(".opencode/commands/spec.md")).unwrap();
        assert!(spec.contains("patina spec check"));

        let beliefs =
            fs::read_to_string(templates_dir.join(".opencode/commands/epistemic-beliefs.md"))
                .unwrap();
        assert!(beliefs.contains(".opencode/bin/create-belief.sh"));
    }
}
