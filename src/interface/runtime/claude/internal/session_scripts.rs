//! Session script generation for Claude interface
//!
//! Deploys command definitions (.md) and thin wrapper scripts that forward
//! to the native AI session backend while preserving local script entrypoints.

use anyhow::Result;
use std::fs;
use std::path::Path;

use super::paths;
use crate::mother::skills;

fn wrapper_start() -> String {
    "#!/bin/bash\nexec env PATINA_AI_INTERFACE=claude patina ai session new --json --interface claude \"$@\"\n".to_string()
}

fn wrapper_update() -> String {
    "#!/bin/bash\nexec env PATINA_AI_INTERFACE=claude patina ai session update --json \"$@\"\n"
        .to_string()
}

fn wrapper_note() -> String {
    "#!/bin/bash\nexec env PATINA_AI_INTERFACE=claude patina ai session note \"$@\"\n".to_string()
}

fn wrapper_end() -> String {
    "#!/bin/bash\nexec env PATINA_AI_INTERFACE=claude patina ai session end --json \"$@\"\n"
        .to_string()
}

/// Create all session scripts and command definitions
pub fn create_session_scripts(project_path: &Path) -> Result<()> {
    let commands_path = paths::get_commands_path(project_path);
    let bin_path = paths::get_bin_path(project_path);

    // Ensure directories exist
    fs::create_dir_all(&commands_path)?;
    fs::create_dir_all(&bin_path)?;

    // Deploy wrapper scripts (backward compatibility)
    write_script(&bin_path.join("session-new.sh"), &wrapper_start())?;
    write_script(&bin_path.join("session-update.sh"), &wrapper_update())?;
    write_script(&bin_path.join("session-note.sh"), &wrapper_note())?;
    write_script(&bin_path.join("session-end.sh"), &wrapper_end())?;

    // Deploy command definitions
    fs::write(
        commands_path.join("session-new.md"),
        skills::skill_content("claude", "session-new")
            .and_then(|content| content.files.first().map(|file| file.bytes))
            .unwrap_or_default(),
    )?;
    fs::write(
        commands_path.join("session-update.md"),
        skills::skill_content("claude", "session-update")
            .and_then(|content| content.files.first().map(|file| file.bytes))
            .unwrap_or_default(),
    )?;
    fs::write(
        commands_path.join("session-note.md"),
        skills::skill_content("claude", "session-note")
            .and_then(|content| content.files.first().map(|file| file.bytes))
            .unwrap_or_default(),
    )?;
    fs::write(
        commands_path.join("session-end.md"),
        skills::skill_content("claude", "session-end")
            .and_then(|content| content.files.first().map(|file| file.bytes))
            .unwrap_or_default(),
    )?;

    // patina-review command (no shell script, just prompt)
    fs::write(
        commands_path.join("patina-review.md"),
        skills::skill_content("claude", "patina-review")
            .and_then(|content| content.files.first().map(|file| file.bytes))
            .unwrap_or_default(),
    )?;

    // /spec skill (no shell script, just prompt — uses MCP tools)
    fs::write(
        commands_path.join("spec.md"),
        skills::skill_content("claude", "spec")
            .and_then(|content| content.files.first().map(|file| file.bytes))
            .unwrap_or_default(),
    )?;

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
