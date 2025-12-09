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

use crate::workspace;

// =============================================================================
// Embedded Templates - Claude
// =============================================================================

mod claude_templates {
    // Shell scripts
    pub const SESSION_START_SH: &str =
        include_str!("../../resources/claude/.claude/bin/session-start.sh");
    pub const SESSION_UPDATE_SH: &str =
        include_str!("../../resources/claude/.claude/bin/session-update.sh");
    pub const SESSION_NOTE_SH: &str =
        include_str!("../../resources/claude/.claude/bin/session-note.sh");
    pub const SESSION_END_SH: &str =
        include_str!("../../resources/claude/.claude/bin/session-end.sh");
    pub const SESSION_GIT_START_SH: &str =
        include_str!("../../resources/claude/.claude/bin/session-git-start.sh");
    pub const SESSION_GIT_UPDATE_SH: &str =
        include_str!("../../resources/claude/.claude/bin/session-git-update.sh");
    pub const SESSION_GIT_NOTE_SH: &str =
        include_str!("../../resources/claude/.claude/bin/session-git-note.sh");
    pub const SESSION_GIT_END_SH: &str =
        include_str!("../../resources/claude/.claude/bin/session-git-end.sh");

    // Commands (markdown)
    pub const SESSION_START_MD: &str =
        include_str!("../../resources/claude/.claude/commands/session-start.md");
    pub const SESSION_UPDATE_MD: &str =
        include_str!("../../resources/claude/.claude/commands/session-update.md");
    pub const SESSION_NOTE_MD: &str =
        include_str!("../../resources/claude/.claude/commands/session-note.md");
    pub const SESSION_END_MD: &str =
        include_str!("../../resources/claude/.claude/commands/session-end.md");
    pub const SESSION_GIT_START_MD: &str =
        include_str!("../../resources/claude/.claude/commands/session-git-start.md");
    pub const SESSION_GIT_UPDATE_MD: &str =
        include_str!("../../resources/claude/.claude/commands/session-git-update.md");
    pub const SESSION_GIT_NOTE_MD: &str =
        include_str!("../../resources/claude/.claude/commands/session-git-note.md");
    pub const SESSION_GIT_END_MD: &str =
        include_str!("../../resources/claude/.claude/commands/session-git-end.md");

    // Context template
    pub const CLAUDE_MD: &str = include_str!("../../resources/claude/.claude/CLAUDE.md");
}

// =============================================================================
// Embedded Templates - Gemini
// =============================================================================

mod gemini_templates {
    // Shell scripts
    pub const SESSION_START_SH: &str =
        include_str!("../../resources/gemini/.gemini/bin/session-start.sh");
    pub const SESSION_UPDATE_SH: &str =
        include_str!("../../resources/gemini/.gemini/bin/session-update.sh");
    pub const SESSION_NOTE_SH: &str =
        include_str!("../../resources/gemini/.gemini/bin/session-note.sh");
    pub const SESSION_END_SH: &str =
        include_str!("../../resources/gemini/.gemini/bin/session-end.sh");
    pub const SESSION_GIT_START_SH: &str =
        include_str!("../../resources/gemini/.gemini/bin/session-git-start.sh");
    pub const SESSION_GIT_UPDATE_SH: &str =
        include_str!("../../resources/gemini/.gemini/bin/session-git-update.sh");
    pub const SESSION_GIT_NOTE_SH: &str =
        include_str!("../../resources/gemini/.gemini/bin/session-git-note.sh");
    pub const SESSION_GIT_END_SH: &str =
        include_str!("../../resources/gemini/.gemini/bin/session-git-end.sh");

    // Commands (TOML format for Gemini)
    pub const SESSION_START_TOML: &str =
        include_str!("../../resources/gemini/.gemini/commands/session-start.toml");
    pub const SESSION_UPDATE_TOML: &str =
        include_str!("../../resources/gemini/.gemini/commands/session-update.toml");
    pub const SESSION_NOTE_TOML: &str =
        include_str!("../../resources/gemini/.gemini/commands/session-note.toml");
    pub const SESSION_END_TOML: &str =
        include_str!("../../resources/gemini/.gemini/commands/session-end.toml");
    pub const SESSION_GIT_START_TOML: &str =
        include_str!("../../resources/gemini/.gemini/commands/session-git-start.toml");
    pub const SESSION_GIT_UPDATE_TOML: &str =
        include_str!("../../resources/gemini/.gemini/commands/session-git-update.toml");
    pub const SESSION_GIT_NOTE_TOML: &str =
        include_str!("../../resources/gemini/.gemini/commands/session-git-note.toml");
    pub const SESSION_GIT_END_TOML: &str =
        include_str!("../../resources/gemini/.gemini/commands/session-git-end.toml");

    // Context template (at root level for Gemini)
    pub const GEMINI_MD: &str = include_str!("../../resources/gemini/GEMINI.md");
}

// =============================================================================
// Public API
// =============================================================================

/// Extract all templates to ~/.patina/adapters/
///
/// Called during first-run setup. Creates the full template structure
/// for all supported frontends.
pub fn install_all(adapters_dir: &Path) -> Result<()> {
    install_claude_templates(adapters_dir)?;
    install_gemini_templates(adapters_dir)?;
    // install_codex_templates(adapters_dir)?; // Future
    Ok(())
}

/// Copy adapter templates to project
///
/// Copies the adapter-specific directory (.claude/, .gemini/) from
/// central templates to the project.
pub fn copy_to_project(frontend: &str, project_path: &Path) -> Result<()> {
    let templates_dir = workspace::adapters_dir().join(frontend).join("templates");

    let adapter_dir_name = format!(".{}", frontend);
    let src = templates_dir.join(&adapter_dir_name);
    let dest = project_path.join(&adapter_dir_name);

    if !src.exists() {
        // Templates not installed yet, install from embedded
        let adapters = workspace::adapters_dir();
        install_all(&adapters)?;
    }

    copy_dir_recursive(&src, &dest)?;
    Ok(())
}

/// Check if templates are installed for a frontend
pub fn templates_installed(frontend: &str) -> bool {
    let templates_dir = workspace::adapters_dir().join(frontend).join("templates");
    templates_dir.exists() && templates_dir.join(format!(".{}", frontend)).exists()
}

// =============================================================================
// Claude Templates Installation
// =============================================================================

fn install_claude_templates(adapters_dir: &Path) -> Result<()> {
    let templates_dir = adapters_dir.join("claude").join("templates");
    let claude_dir = templates_dir.join(".claude");
    let bin_dir = claude_dir.join("bin");
    let commands_dir = claude_dir.join("commands");
    let context_dir = claude_dir.join("context");

    // Create directories
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&commands_dir)?;
    fs::create_dir_all(&context_dir)?;

    // Write shell scripts
    write_executable(&bin_dir.join("session-start.sh"), claude_templates::SESSION_START_SH)?;
    write_executable(&bin_dir.join("session-update.sh"), claude_templates::SESSION_UPDATE_SH)?;
    write_executable(&bin_dir.join("session-note.sh"), claude_templates::SESSION_NOTE_SH)?;
    write_executable(&bin_dir.join("session-end.sh"), claude_templates::SESSION_END_SH)?;
    write_executable(&bin_dir.join("session-git-start.sh"), claude_templates::SESSION_GIT_START_SH)?;
    write_executable(&bin_dir.join("session-git-update.sh"), claude_templates::SESSION_GIT_UPDATE_SH)?;
    write_executable(&bin_dir.join("session-git-note.sh"), claude_templates::SESSION_GIT_NOTE_SH)?;
    write_executable(&bin_dir.join("session-git-end.sh"), claude_templates::SESSION_GIT_END_SH)?;

    // Write commands
    fs::write(commands_dir.join("session-start.md"), claude_templates::SESSION_START_MD)?;
    fs::write(commands_dir.join("session-update.md"), claude_templates::SESSION_UPDATE_MD)?;
    fs::write(commands_dir.join("session-note.md"), claude_templates::SESSION_NOTE_MD)?;
    fs::write(commands_dir.join("session-end.md"), claude_templates::SESSION_END_MD)?;
    fs::write(commands_dir.join("session-git-start.md"), claude_templates::SESSION_GIT_START_MD)?;
    fs::write(commands_dir.join("session-git-update.md"), claude_templates::SESSION_GIT_UPDATE_MD)?;
    fs::write(commands_dir.join("session-git-note.md"), claude_templates::SESSION_GIT_NOTE_MD)?;
    fs::write(commands_dir.join("session-git-end.md"), claude_templates::SESSION_GIT_END_MD)?;

    // Write CLAUDE.md template
    fs::write(claude_dir.join("CLAUDE.md"), claude_templates::CLAUDE_MD)?;

    Ok(())
}

// =============================================================================
// Gemini Templates Installation
// =============================================================================

fn install_gemini_templates(adapters_dir: &Path) -> Result<()> {
    let templates_dir = adapters_dir.join("gemini").join("templates");
    let gemini_dir = templates_dir.join(".gemini");
    let bin_dir = gemini_dir.join("bin");
    let commands_dir = gemini_dir.join("commands");
    let context_dir = gemini_dir.join("context");

    // Create directories
    fs::create_dir_all(&bin_dir)?;
    fs::create_dir_all(&commands_dir)?;
    fs::create_dir_all(&context_dir)?;

    // Write shell scripts
    write_executable(&bin_dir.join("session-start.sh"), gemini_templates::SESSION_START_SH)?;
    write_executable(&bin_dir.join("session-update.sh"), gemini_templates::SESSION_UPDATE_SH)?;
    write_executable(&bin_dir.join("session-note.sh"), gemini_templates::SESSION_NOTE_SH)?;
    write_executable(&bin_dir.join("session-end.sh"), gemini_templates::SESSION_END_SH)?;
    write_executable(&bin_dir.join("session-git-start.sh"), gemini_templates::SESSION_GIT_START_SH)?;
    write_executable(&bin_dir.join("session-git-update.sh"), gemini_templates::SESSION_GIT_UPDATE_SH)?;
    write_executable(&bin_dir.join("session-git-note.sh"), gemini_templates::SESSION_GIT_NOTE_SH)?;
    write_executable(&bin_dir.join("session-git-end.sh"), gemini_templates::SESSION_GIT_END_SH)?;

    // Write commands (TOML format for Gemini)
    fs::write(commands_dir.join("session-start.toml"), gemini_templates::SESSION_START_TOML)?;
    fs::write(commands_dir.join("session-update.toml"), gemini_templates::SESSION_UPDATE_TOML)?;
    fs::write(commands_dir.join("session-note.toml"), gemini_templates::SESSION_NOTE_TOML)?;
    fs::write(commands_dir.join("session-end.toml"), gemini_templates::SESSION_END_TOML)?;
    fs::write(commands_dir.join("session-git-start.toml"), gemini_templates::SESSION_GIT_START_TOML)?;
    fs::write(commands_dir.join("session-git-update.toml"), gemini_templates::SESSION_GIT_UPDATE_TOML)?;
    fs::write(commands_dir.join("session-git-note.toml"), gemini_templates::SESSION_GIT_NOTE_TOML)?;
    fs::write(commands_dir.join("session-git-end.toml"), gemini_templates::SESSION_GIT_END_TOML)?;

    // Write GEMINI.md template at templates root (not inside .gemini/)
    fs::write(templates_dir.join("GEMINI.md"), gemini_templates::GEMINI_MD)?;

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
            fs::copy(&src_path, &dest_path)
                .with_context(|| format!("Failed to copy: {} -> {}", src_path.display(), dest_path.display()))?;

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
        assert!(!claude_templates::CLAUDE_MD.is_empty());
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

        let claude_dir = temp.path().join("claude/templates/.claude");
        assert!(claude_dir.join("bin/session-start.sh").exists());
        assert!(claude_dir.join("commands/session-start.md").exists());
        assert!(claude_dir.join("CLAUDE.md").exists());
    }

    #[test]
    fn test_install_gemini_templates() {
        let temp = TempDir::new().unwrap();
        install_gemini_templates(temp.path()).unwrap();

        let gemini_dir = temp.path().join("gemini/templates/.gemini");
        assert!(gemini_dir.join("bin/session-start.sh").exists());
        assert!(gemini_dir.join("commands/session-start.toml").exists());

        // GEMINI.md should be at templates root, not inside .gemini/
        assert!(temp.path().join("gemini/templates/GEMINI.md").exists());
    }
}
