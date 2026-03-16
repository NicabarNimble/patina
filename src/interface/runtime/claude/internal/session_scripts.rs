//! Session script generation for Claude adapter
//!
//! Deploys command definitions (.md) and thin wrapper scripts that forward
//! to the native AI session backend while preserving local script entrypoints.

use anyhow::Result;
use std::fs;
use std::path::Path;

use super::paths;

// Embed command definitions from resources
const SESSION_START_MD: &str = include_str!("../../../../../resources/claude/session-start.md");
const SESSION_UPDATE_MD: &str = include_str!("../../../../../resources/claude/session-update.md");
const SESSION_NOTE_MD: &str = include_str!("../../../../../resources/claude/session-note.md");
const SESSION_END_MD: &str = include_str!("../../../../../resources/claude/session-end.md");

// Embed patina-review command from resources
const PATINA_REVIEW_MD: &str = include_str!("../../../../../resources/claude/patina-review.md");

// Embed /spec skill from resources
const SPEC_MD: &str = include_str!("../../../../../resources/claude/spec.md");

fn wrapper_start() -> String {
    "#!/bin/bash\nexec env PATINA_AI_INTERFACE=claude patina ai session start --json --adapter claude \"$@\"\n".to_string()
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
    write_script(&bin_path.join("session-start.sh"), &wrapper_start())?;
    write_script(&bin_path.join("session-update.sh"), &wrapper_update())?;
    write_script(&bin_path.join("session-note.sh"), &wrapper_note())?;
    write_script(&bin_path.join("session-end.sh"), &wrapper_end())?;

    // Deploy command definitions
    fs::write(commands_path.join("session-start.md"), SESSION_START_MD)?;
    fs::write(commands_path.join("session-update.md"), SESSION_UPDATE_MD)?;
    fs::write(commands_path.join("session-note.md"), SESSION_NOTE_MD)?;
    fs::write(commands_path.join("session-end.md"), SESSION_END_MD)?;

    // patina-review command (no shell script, just prompt)
    fs::write(commands_path.join("patina-review.md"), PATINA_REVIEW_MD)?;

    // /spec skill (no shell script, just prompt — uses MCP tools)
    fs::write(commands_path.join("spec.md"), SPEC_MD)?;

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
