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

// Embed launch command from resources
const LAUNCH_SH: &str = include_str!("../../../../resources/claude/.claude/bin/launch.sh");
const LAUNCH_MD: &str = include_str!("../../../../resources/claude/launch.md");

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

    // launch command
    write_script(&bin_path.join("launch.sh"), LAUNCH_SH)?;
    fs::write(commands_path.join("launch.md"), LAUNCH_MD)?;

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
