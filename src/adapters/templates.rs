//! Template extraction and management
//!
//! Handles extracting embedded templates to ~/.patina/adapters/
//! and copying templates to projects.
//!
//! Templates are embedded at compile time and extracted on first run.
//! This allows user customization of templates in ~/.patina/adapters/.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::paths;

// =============================================================================
// Embedded Templates - Claude
// =============================================================================

mod claude_templates {
    // Shell scripts (git-integrated)
    pub const SESSION_START_SH: &str = include_str!("../../resources/claude/session-start.sh");
    pub const SESSION_UPDATE_SH: &str = include_str!("../../resources/claude/session-update.sh");
    pub const SESSION_NOTE_SH: &str = include_str!("../../resources/claude/session-note.sh");
    pub const SESSION_END_SH: &str = include_str!("../../resources/claude/session-end.sh");

    // Commands (markdown)
    pub const SESSION_START_MD: &str = include_str!("../../resources/claude/session-start.md");
    pub const SESSION_UPDATE_MD: &str = include_str!("../../resources/claude/session-update.md");
    pub const SESSION_NOTE_MD: &str = include_str!("../../resources/claude/session-note.md");
    pub const SESSION_END_MD: &str = include_str!("../../resources/claude/session-end.md");
    pub const PATINA_REVIEW_MD: &str = include_str!("../../resources/claude/patina-review.md");

    // Skills - epistemic-beliefs
    pub const SKILL_EPISTEMIC_BELIEFS_MD: &str =
        include_str!("../../resources/claude/skills/epistemic-beliefs/SKILL.md");
    pub const SKILL_EPISTEMIC_BELIEFS_CREATE_SH: &str =
        include_str!("../../resources/claude/skills/epistemic-beliefs/scripts/create-belief.sh");
    pub const SKILL_EPISTEMIC_BELIEFS_EXAMPLE_MD: &str = include_str!(
        "../../resources/claude/skills/epistemic-beliefs/references/belief-example.md"
    );
}

// =============================================================================
// Embedded Templates - Gemini
// =============================================================================

mod gemini_templates {
    // Shell scripts (git-integrated)
    pub const SESSION_START_SH: &str = include_str!("../../resources/gemini/session-start.sh");
    pub const SESSION_UPDATE_SH: &str = include_str!("../../resources/gemini/session-update.sh");
    pub const SESSION_NOTE_SH: &str = include_str!("../../resources/gemini/session-note.sh");
    pub const SESSION_END_SH: &str = include_str!("../../resources/gemini/session-end.sh");

    // Commands (TOML format for Gemini)
    pub const SESSION_START_TOML: &str = include_str!("../../resources/gemini/session-start.toml");
    pub const SESSION_UPDATE_TOML: &str =
        include_str!("../../resources/gemini/session-update.toml");
    pub const SESSION_NOTE_TOML: &str = include_str!("../../resources/gemini/session-note.toml");
    pub const SESSION_END_TOML: &str = include_str!("../../resources/gemini/session-end.toml");
    pub const PATINA_REVIEW_TOML: &str = include_str!("../../resources/gemini/patina-review.toml");

    // Context template
    pub const GEMINI_MD: &str = include_str!("../../resources/gemini/GEMINI.md");
}

// =============================================================================
// Embedded Templates - OpenCode
// =============================================================================

mod opencode_templates {
    // Shell scripts (git-integrated)
    pub const SESSION_START_SH: &str = include_str!("../../resources/opencode/session-start.sh");
    pub const SESSION_UPDATE_SH: &str = include_str!("../../resources/opencode/session-update.sh");
    pub const SESSION_NOTE_SH: &str = include_str!("../../resources/opencode/session-note.sh");
    pub const SESSION_END_SH: &str = include_str!("../../resources/opencode/session-end.sh");

    // Commands (markdown format, same as Claude)
    pub const SESSION_START_MD: &str = include_str!("../../resources/opencode/session-start.md");
    pub const SESSION_UPDATE_MD: &str = include_str!("../../resources/opencode/session-update.md");
    pub const SESSION_NOTE_MD: &str = include_str!("../../resources/opencode/session-note.md");
    pub const SESSION_END_MD: &str = include_str!("../../resources/opencode/session-end.md");
    pub const PATINA_REVIEW_MD: &str = include_str!("../../resources/opencode/patina-review.md");
}

// =============================================================================
// Public API
// =============================================================================

/// Extract all templates to ~/.patina/adapters/
///
/// Called during first-run setup. Creates the full template structure
/// for all supported adapters.
pub fn install_all(adapters_dir: &Path) -> Result<()> {
    install_claude_templates(adapters_dir)?;
    install_gemini_templates(adapters_dir)?;
    install_opencode_templates(adapters_dir)?;
    Ok(())
}

/// Copy adapter templates to project
///
/// Copies the adapter-specific directory (.claude/, .gemini/) from
/// central templates to the project.
pub fn copy_to_project(adapter_name: &str, project_path: &Path) -> Result<()> {
    let templates_dir = paths::adapters_dir().join(adapter_name).join("templates");

    let adapter_dir_name = format!(".{}", adapter_name);
    let src = templates_dir.join(&adapter_dir_name);
    let dest = project_path.join(&adapter_dir_name);

    if !src.exists() {
        // Templates not installed yet, install from embedded
        let adapters = paths::adapters_dir();
        install_all(&adapters)?;
    }

    copy_dir_recursive(&src, &dest)?;
    Ok(())
}

/// Check if templates are installed for an adapter
pub fn templates_installed(adapter_name: &str) -> bool {
    let templates_dir = paths::adapters_dir().join(adapter_name).join("templates");
    templates_dir.exists()
}

// =============================================================================
// Claude Templates Installation
// =============================================================================

fn install_claude_templates(adapters_dir: &Path) -> Result<()> {
    let templates_dir = adapters_dir.join("claude").join("templates");
    // Create .claude/ structure inside templates/ so copy_to_project works correctly
    let claude_dir = templates_dir.join(".claude");
    let bin_dir = claude_dir.join("bin");
    let commands_dir = claude_dir.join("commands");
    let context_dir = claude_dir.join("context");
    let skills_dir = claude_dir.join("skills");

    // Create directories
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&commands_dir)?;
    fs::create_dir_all(&context_dir)?;
    fs::create_dir_all(&skills_dir)?;

    // Write shell scripts
    write_executable(
        &bin_dir.join("session-start.sh"),
        claude_templates::SESSION_START_SH,
    )?;
    write_executable(
        &bin_dir.join("session-update.sh"),
        claude_templates::SESSION_UPDATE_SH,
    )?;
    write_executable(
        &bin_dir.join("session-note.sh"),
        claude_templates::SESSION_NOTE_SH,
    )?;
    write_executable(
        &bin_dir.join("session-end.sh"),
        claude_templates::SESSION_END_SH,
    )?;

    // Write commands
    fs::write(
        commands_dir.join("session-start.md"),
        claude_templates::SESSION_START_MD,
    )?;
    fs::write(
        commands_dir.join("session-update.md"),
        claude_templates::SESSION_UPDATE_MD,
    )?;
    fs::write(
        commands_dir.join("session-note.md"),
        claude_templates::SESSION_NOTE_MD,
    )?;
    fs::write(
        commands_dir.join("session-end.md"),
        claude_templates::SESSION_END_MD,
    )?;
    fs::write(
        commands_dir.join("patina-review.md"),
        claude_templates::PATINA_REVIEW_MD,
    )?;

    // Write skills - epistemic-beliefs
    let epistemic_beliefs_dir = skills_dir.join("epistemic-beliefs");
    let epistemic_scripts_dir = epistemic_beliefs_dir.join("scripts");
    let epistemic_refs_dir = epistemic_beliefs_dir.join("references");
    fs::create_dir_all(&epistemic_scripts_dir)?;
    fs::create_dir_all(&epistemic_refs_dir)?;

    fs::write(
        epistemic_beliefs_dir.join("SKILL.md"),
        claude_templates::SKILL_EPISTEMIC_BELIEFS_MD,
    )?;
    write_executable(
        &epistemic_scripts_dir.join("create-belief.sh"),
        claude_templates::SKILL_EPISTEMIC_BELIEFS_CREATE_SH,
    )?;
    fs::write(
        epistemic_refs_dir.join("belief-example.md"),
        claude_templates::SKILL_EPISTEMIC_BELIEFS_EXAMPLE_MD,
    )?;

    Ok(())
}

// =============================================================================
// Gemini Templates Installation
// =============================================================================

fn install_gemini_templates(adapters_dir: &Path) -> Result<()> {
    let templates_dir = adapters_dir.join("gemini").join("templates");
    // Create .gemini/ structure inside templates/ so copy_to_project works correctly
    let gemini_dir = templates_dir.join(".gemini");
    let bin_dir = gemini_dir.join("bin");
    let commands_dir = gemini_dir.join("commands");

    // Create directories
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&commands_dir)?;

    // Write shell scripts
    write_executable(
        &bin_dir.join("session-start.sh"),
        gemini_templates::SESSION_START_SH,
    )?;
    write_executable(
        &bin_dir.join("session-update.sh"),
        gemini_templates::SESSION_UPDATE_SH,
    )?;
    write_executable(
        &bin_dir.join("session-note.sh"),
        gemini_templates::SESSION_NOTE_SH,
    )?;
    write_executable(
        &bin_dir.join("session-end.sh"),
        gemini_templates::SESSION_END_SH,
    )?;

    // Write commands (TOML format for Gemini)
    fs::write(
        commands_dir.join("session-start.toml"),
        gemini_templates::SESSION_START_TOML,
    )?;
    fs::write(
        commands_dir.join("session-update.toml"),
        gemini_templates::SESSION_UPDATE_TOML,
    )?;
    fs::write(
        commands_dir.join("session-note.toml"),
        gemini_templates::SESSION_NOTE_TOML,
    )?;
    fs::write(
        commands_dir.join("session-end.toml"),
        gemini_templates::SESSION_END_TOML,
    )?;
    fs::write(
        commands_dir.join("patina-review.toml"),
        gemini_templates::PATINA_REVIEW_TOML,
    )?;

    // Write GEMINI.md template
    fs::write(templates_dir.join("GEMINI.md"), gemini_templates::GEMINI_MD)?;

    Ok(())
}

// =============================================================================
// OpenCode Templates Installation
// =============================================================================

fn install_opencode_templates(adapters_dir: &Path) -> Result<()> {
    let templates_dir = adapters_dir.join("opencode").join("templates");
    // Create .opencode/ structure inside templates/ so copy_to_project works correctly
    let opencode_dir = templates_dir.join(".opencode");
    let bin_dir = opencode_dir.join("bin");
    let commands_dir = opencode_dir.join("commands");

    // Create directories
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&commands_dir)?;

    // Write shell scripts
    write_executable(
        &bin_dir.join("session-start.sh"),
        opencode_templates::SESSION_START_SH,
    )?;
    write_executable(
        &bin_dir.join("session-update.sh"),
        opencode_templates::SESSION_UPDATE_SH,
    )?;
    write_executable(
        &bin_dir.join("session-note.sh"),
        opencode_templates::SESSION_NOTE_SH,
    )?;
    write_executable(
        &bin_dir.join("session-end.sh"),
        opencode_templates::SESSION_END_SH,
    )?;

    // Write commands (markdown format, same as Claude)
    fs::write(
        commands_dir.join("session-start.md"),
        opencode_templates::SESSION_START_MD,
    )?;
    fs::write(
        commands_dir.join("session-update.md"),
        opencode_templates::SESSION_UPDATE_MD,
    )?;
    fs::write(
        commands_dir.join("session-note.md"),
        opencode_templates::SESSION_NOTE_MD,
    )?;
    fs::write(
        commands_dir.join("session-end.md"),
        opencode_templates::SESSION_END_MD,
    )?;
    fs::write(
        commands_dir.join("patina-review.md"),
        opencode_templates::PATINA_REVIEW_MD,
    )?;

    Ok(())
}

// =============================================================================
// Helpers
// =============================================================================

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

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    if !src.exists() {
        anyhow::bail!("Source directory does not exist: {}", src.display());
    }

    fs::create_dir_all(dest)
        .with_context(|| format!("Failed to create directory: {}", dest.display()))?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path).with_context(|| {
                format!(
                    "Failed to copy: {} -> {}",
                    src_path.display(),
                    dest_path.display()
                )
            })?;

            // Preserve executable permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let src_mode = fs::metadata(&src_path)?.permissions().mode();
                if src_mode & 0o111 != 0 {
                    let mut perms = fs::metadata(&dest_path)?.permissions();
                    perms.set_mode(src_mode);
                    fs::set_permissions(&dest_path, perms)?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_claude_templates_compile() {
        // Just verify templates are embedded correctly
        assert!(!claude_templates::SESSION_START_SH.is_empty());
        assert!(!claude_templates::SESSION_START_MD.is_empty());
        // Skills
        assert!(!claude_templates::SKILL_EPISTEMIC_BELIEFS_MD.is_empty());
        assert!(!claude_templates::SKILL_EPISTEMIC_BELIEFS_CREATE_SH.is_empty());
        assert!(!claude_templates::SKILL_EPISTEMIC_BELIEFS_EXAMPLE_MD.is_empty());
    }

    #[test]
    fn test_gemini_templates_compile() {
        // Just verify templates are embedded correctly
        assert!(!gemini_templates::SESSION_START_SH.is_empty());
        assert!(!gemini_templates::SESSION_START_TOML.is_empty());
        assert!(!gemini_templates::GEMINI_MD.is_empty());
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
        // Deprecated commands should not exist
        assert!(!templates_dir.join(".claude/bin/launch.sh").exists());
        assert!(!templates_dir.join(".claude/bin/persona-start.sh").exists());

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
        assert!(templates_dir.join("GEMINI.md").exists());
    }
}
