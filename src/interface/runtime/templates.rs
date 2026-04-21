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
use crate::mother::skills::{self, SkillContent, SkillContentMode, SkillOwnership};
use crate::paths;

// =============================================================================
// Thin wrapper scripts — generated per interface
// =============================================================================
// These keep the native session backend behind local interface scripts so the
// command markdown/TOML can stay workflow-oriented.

fn wrapper_start(interface: &str) -> String {
    format!(
        "#!/bin/bash\nexec env PATINA_AI_INTERFACE={interface} patina ai session new --json --interface {interface} \"$@\"\n"
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
        &iface_dir.join("bin/session-new.sh"),
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

        match skills::skill_ownership(skill) {
            SkillOwnership::GlobalInterface => {
                write_skill_content(&iface_dir, interface_name, &content)?;
            }
            SkillOwnership::ProjectPatina => {
                sync_project_skill_source(project_path, interface_name, skill, &content)?;
                project_skill_to_interface_projection(
                    project_path,
                    &iface_dir,
                    interface_name,
                    skill,
                    &content,
                )?;
            }
        }
    }

    apply_project_interface_overrides(project_path, interface_name, &iface_dir)?;

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
    "session-new",
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
        &interface_dir.join("bin/session-new.sh"),
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

const MARKER_HTML_START: &str = "<!-- PATINA:START -->";
const MARKER_HTML_END: &str = "<!-- PATINA:END -->";
const MARKER_TEXT_START: &str = "# PATINA:START";
const MARKER_TEXT_END: &str = "# PATINA:END";

fn render_skill_body<'a>(interface_name: &str, source: &'a str) -> Cow<'a, str> {
    if interface_name == "pi" {
        Cow::Owned(source.replace(".opencode/", ".pi/"))
    } else {
        Cow::Borrowed(source)
    }
}

fn sync_project_skill_source(
    project_root: &Path,
    interface_name: &str,
    skill_name: &str,
    content: &SkillContent,
) -> Result<()> {
    for file in content.files {
        let source_path =
            paths::project::interface_skill_dir(project_root, interface_name, skill_name)
                .join(file.projection_file);
        if let Some(parent) = source_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = render_skill_body(interface_name, file.bytes);
        write_skill_file(&source_path, file.mode, body.as_ref())?;
    }
    Ok(())
}

fn project_skill_to_interface_projection(
    project_root: &Path,
    interface_dir: &Path,
    interface_name: &str,
    skill_name: &str,
    content: &SkillContent,
) -> Result<()> {
    for file in content.files {
        let source_path =
            paths::project::interface_skill_dir(project_root, interface_name, skill_name)
                .join(file.projection_file);
        let body = if source_path.exists() {
            fs::read_to_string(&source_path)?
        } else {
            render_skill_body(interface_name, file.bytes).into_owned()
        };
        let projection_path = interface_dir.join(file.projection_file);
        if let Some(parent) = projection_path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_skill_file(&projection_path, file.mode, &body)?;
    }

    Ok(())
}

fn apply_project_interface_overrides(
    project_root: &Path,
    interface_name: &str,
    interface_dir: &Path,
) -> Result<()> {
    let overrides_dir = paths::project::interface_overrides_dir(project_root, interface_name);
    if !overrides_dir.exists() {
        return Ok(());
    }

    copy_override_tree(&overrides_dir, &overrides_dir, interface_dir)
}

fn copy_override_tree(root: &Path, current: &Path, destination_root: &Path) -> Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let source = entry.path();
        let relative = source.strip_prefix(root)?;
        let destination = destination_root.join(relative);

        if source.is_dir() {
            fs::create_dir_all(&destination)?;
            copy_override_tree(root, &source, destination_root)?;
            continue;
        }

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(&source, &destination)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let src_mode = fs::metadata(&source)?.permissions().mode();
            if src_mode & 0o111 != 0 {
                let mut perms = fs::metadata(&destination)?.permissions();
                perms.set_mode(src_mode);
                fs::set_permissions(&destination, perms)?;
            }
        }
    }

    Ok(())
}

fn managed_markers(mode: SkillContentMode) -> Option<(&'static str, &'static str)> {
    match mode {
        SkillContentMode::Markdown => Some((MARKER_HTML_START, MARKER_HTML_END)),
        SkillContentMode::Toml => Some((MARKER_TEXT_START, MARKER_TEXT_END)),
        SkillContentMode::Executable => None,
    }
}

fn update_or_append_managed(existing: &str, managed_block: &str, start: &str, end: &str) -> String {
    if let (Some(start_idx), Some(end_idx)) = (existing.find(start), existing.find(end)) {
        if start_idx < end_idx {
            let before = &existing[..start_idx];
            let after = &existing[end_idx + end.len()..];
            return format!("{}{}{}", before, managed_block, after);
        }
    }

    managed_block.to_string()
}

fn wrap_managed_content(mode: SkillContentMode, body: &str) -> String {
    let normalized = format!("{}\n", body.trim_end());
    match mode {
        SkillContentMode::Markdown => {
            format!("{}\n{}\n{}", MARKER_HTML_START, normalized, MARKER_HTML_END)
        }
        SkillContentMode::Toml => {
            format!("{}\n{}\n{}", MARKER_TEXT_START, normalized, MARKER_TEXT_END)
        }
        SkillContentMode::Executable => normalized,
    }
}

fn write_skill_file(path: &Path, mode: SkillContentMode, body: &str) -> Result<()> {
    if mode == SkillContentMode::Executable {
        write_executable(path, body)?;
        return Ok(());
    }

    let managed = wrap_managed_content(mode, body);
    let next = if path.exists() {
        let existing = fs::read_to_string(path)?;
        if let Some((start, end)) = managed_markers(mode) {
            update_or_append_managed(&existing, &managed, start, end)
        } else {
            managed
        }
    } else {
        managed
    };

    fs::write(path, next)?;
    Ok(())
}

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

        let body = render_skill_body(interface_name, file.bytes);
        write_skill_file(&path, file.mode, body.as_ref())?;
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
        let start = skills::skill_content("claude", "session-new").unwrap();
        assert!(start.files[0].bytes.contains("session-new.sh"));
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
        let start = skills::skill_content("gemini", "session-new").unwrap();
        assert!(start.files[0].bytes.contains("session-new.sh"));
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
        let start = skills::skill_content("opencode", "session-new").unwrap();
        assert!(start.files[0]
            .bytes
            .contains(".opencode/bin/session-new.sh"));
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
        assert!(start.contains("patina ai session new --json --interface claude"));
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
        assert!(templates_dir.join(".claude/bin/session-new.sh").exists());
        assert!(templates_dir
            .join(".claude/commands/session-new.md")
            .exists());
        assert!(templates_dir
            .join(".claude/commands/patina-review.md")
            .exists());
        assert!(templates_dir.join(".claude/commands/spec.md").exists());
        // Deprecated commands should not exist
        assert!(!templates_dir.join(".claude/bin/launch.sh").exists());
        assert!(!templates_dir.join(".claude/bin/persona-start.sh").exists());

        // Wrapper scripts should forward to the native AI session backend
        let wrapper = fs::read_to_string(templates_dir.join(".claude/bin/session-new.sh")).unwrap();
        assert!(wrapper.contains("PATINA_AI_INTERFACE=claude"));
        assert!(wrapper.contains("patina ai session new --json --interface claude"));

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
        assert!(templates_dir.join(".gemini/bin/session-new.sh").exists());
        assert!(templates_dir
            .join(".gemini/commands/session-new.toml")
            .exists());
        assert!(templates_dir.join(".gemini/commands/spec.toml").exists());
        assert!(templates_dir
            .join(".gemini/commands/epistemic-beliefs.toml")
            .exists());
        assert!(templates_dir.join(".gemini/bin/create-belief.sh").exists());

        // Wrapper scripts should forward to the native AI session backend
        let wrapper = fs::read_to_string(templates_dir.join(".gemini/bin/session-new.sh")).unwrap();
        assert!(wrapper.contains("PATINA_AI_INTERFACE=gemini"));
        assert!(wrapper.contains("patina ai session new --json --interface gemini"));
    }

    #[test]
    fn test_install_opencode_templates() {
        let temp = TempDir::new().unwrap();
        install_opencode_templates(temp.path()).unwrap();

        let templates_dir = temp.path().join("opencode/templates");
        assert!(templates_dir.join(".opencode/bin/session-new.sh").exists());
        assert!(templates_dir
            .join(".opencode/commands/session-new.md")
            .exists());
        assert!(templates_dir.join(".opencode/commands/spec.md").exists());
        assert!(templates_dir
            .join(".opencode/commands/epistemic-beliefs.md")
            .exists());
        assert!(templates_dir
            .join(".opencode/bin/create-belief.sh")
            .exists());

        let session_start =
            fs::read_to_string(templates_dir.join(".opencode/commands/session-new.md")).unwrap();
        assert!(session_start.contains(".opencode/bin/session-new.sh"));
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

    #[test]
    fn copy_to_project_seeds_project_owned_patina_skills_for_pi() {
        let temp = TempDir::new().unwrap();
        copy_to_project("pi", temp.path()).unwrap();

        let source = temp
            .path()
            .join(".patina/skills/pi/epistemic-beliefs/commands/epistemic-beliefs.md");
        let projected = temp.path().join(".pi/commands/epistemic-beliefs.md");

        assert!(source.exists());
        assert!(projected.exists());

        let source_text = fs::read_to_string(&source).unwrap();
        assert!(source_text.contains("PATINA:START"));
        assert!(source_text.contains("Capture Patina beliefs"));

        let projected_text = fs::read_to_string(projected).unwrap();
        assert!(projected_text.contains("PATINA:START"));
        assert!(projected_text.contains(".pi/bin/create-belief.sh"));
    }

    #[test]
    fn project_overrides_win_last_for_interface_projection() {
        let temp = TempDir::new().unwrap();
        let override_path = temp
            .path()
            .join(".patina/interfaces/pi/overrides/commands/epistemic-beliefs.md");
        fs::create_dir_all(override_path.parent().unwrap()).unwrap();
        fs::write(&override_path, "override: true\n").unwrap();

        copy_to_project("pi", temp.path()).unwrap();

        let projected = temp.path().join(".pi/commands/epistemic-beliefs.md");
        assert_eq!(fs::read_to_string(projected).unwrap(), "override: true\n");
    }
}
