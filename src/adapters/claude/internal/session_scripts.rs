//! Session script generation for Claude adapter

use anyhow::Result;
use std::fs;
use std::path::Path;

use super::paths;

// Embed session scripts from resources
const SESSION_START_SH: &str = include_str!("../../../../resources/claude/session-start.sh");
const SESSION_START_MD: &str = include_str!("../../../../resources/claude/session-start.md");
const SESSION_UPDATE_SH: &str = include_str!("../../../../resources/claude/session-update.sh");
const SESSION_UPDATE_MD: &str = include_str!("../../../../resources/claude/session-update.md");
const SESSION_NOTE_SH: &str = include_str!("../../../../resources/claude/session-note.sh");
const SESSION_NOTE_MD: &str = include_str!("../../../../resources/claude/session-note.md");
const SESSION_END_SH: &str = include_str!("../../../../resources/claude/session-end.sh");
const SESSION_END_MD: &str = include_str!("../../../../resources/claude/session-end.md");

// Embed git scripts from resources
const GIT_START_SH: &str = include_str!("../../../../resources/claude/git-start.sh");
const GIT_START_MD: &str = include_str!("../../../../resources/claude/git-start.md");
const GIT_UPDATE_SH: &str = include_str!("../../../../resources/claude/git-update.sh");
const GIT_UPDATE_MD: &str = include_str!("../../../../resources/claude/git-update.md");
const GIT_NOTE_SH: &str = include_str!("../../../../resources/claude/git-note.sh");
const GIT_NOTE_MD: &str = include_str!("../../../../resources/claude/git-note.md");
const GIT_END_SH: &str = include_str!("../../../../resources/claude/git-end.sh");
const GIT_END_MD: &str = include_str!("../../../../resources/claude/git-end.md");

/// Create all session scripts and command definitions
pub fn create_session_scripts(project_path: &Path) -> Result<()> {
    let commands_path = paths::get_commands_path(project_path);
    let bin_path = paths::get_bin_path(project_path);

    // Ensure directories exist
    fs::create_dir_all(&commands_path)?;
    fs::create_dir_all(&bin_path)?;

    // Create session-start
    write_script(&bin_path.join("session-start.sh"), SESSION_START_SH)?;
    fs::write(commands_path.join("session-start.md"), SESSION_START_MD)?;

    // Create session-update
    write_script(&bin_path.join("session-update.sh"), SESSION_UPDATE_SH)?;
    fs::write(commands_path.join("session-update.md"), SESSION_UPDATE_MD)?;

    // Create session-note
    write_script(&bin_path.join("session-note.sh"), SESSION_NOTE_SH)?;
    fs::write(commands_path.join("session-note.md"), SESSION_NOTE_MD)?;

    // Create session-end
    write_script(&bin_path.join("session-end.sh"), SESSION_END_SH)?;
    fs::write(commands_path.join("session-end.md"), SESSION_END_MD)?;

    // Create git-start
    write_script(&bin_path.join("git-start.sh"), GIT_START_SH)?;
    fs::write(commands_path.join("git-start.md"), GIT_START_MD)?;

    // Create git-update
    write_script(&bin_path.join("git-update.sh"), GIT_UPDATE_SH)?;
    fs::write(commands_path.join("git-update.md"), GIT_UPDATE_MD)?;

    // Create git-note
    write_script(&bin_path.join("git-note.sh"), GIT_NOTE_SH)?;
    fs::write(commands_path.join("git-note.md"), GIT_NOTE_MD)?;

    // Create git-end
    write_script(&bin_path.join("git-end.sh"), GIT_END_SH)?;
    fs::write(commands_path.join("git-end.md"), GIT_END_MD)?;

    Ok(())
}

/// Write a script file and make it executable on Unix
fn write_script(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content)?;
    make_executable(path)?;
    Ok(())
}

/// Make a file executable on Unix platforms
#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

/// No-op on non-Unix platforms
#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}
