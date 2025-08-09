//! Template management for Claude adapter
//! 
//! This module handles all template generation and file creation for Claude integration.
//! Templates are embedded in the binary for easy distribution.

use anyhow::Result;
use std::path::Path;
use std::fs;

/// Path constants for Claude adapter
pub(super) mod paths {
    pub const ADAPTER_DIR: &str = ".claude";
    pub const CONTEXT_FILE: &str = "CLAUDE.md";
    pub const MCP_DIR: &str = "mcp";
    pub const COMMANDS_DIR: &str = "commands";
    pub const BIN_DIR: &str = "bin";
    pub const CONTEXT_DIR: &str = "context";
    pub const SESSIONS_DIR: &str = "sessions";
    pub const MANIFEST_FILE: &str = "adapter-manifest.json";
}

/// Session management script templates
pub(super) struct SessionScripts;

impl SessionScripts {
    pub fn session_start() -> &'static str {
        include_str!("../../../resources/claude/session-start.sh")
    }

    pub fn session_update() -> &'static str {
        include_str!("../../../resources/claude/session-update.sh")
    }

    pub fn session_note() -> &'static str {
        include_str!("../../../resources/claude/session-note.sh")
    }

    pub fn session_end() -> &'static str {
        include_str!("../../../resources/claude/session-end.sh")
    }
}

/// Creates a file with the given content and makes it executable on Unix
pub(super) fn create_executable_script(path: &Path, content: &str) -> Result<()> {
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

/// Template for README.md in .claude directory
pub(super) fn claude_readme_template() -> &'static str {
    r#"# Claude Integration

This directory contains Claude-specific files for AI-assisted development.

## Structure

- `CLAUDE.md` - Main context file for Claude
- `bin/` - Session management scripts
- `context/` - Session tracking and context management
- `mcp/` - Model Context Protocol directory (if needed)

## Session Management

Use the provided commands to manage development sessions:
- `/session-start [name]` - Start a new session
- `/session-update` - Update current session
- `/session-note [text]` - Add a note to session
- `/session-end` - End and archive session

## Updating

Run `patina update` to update Claude integration files to the latest version.
"#
}

/// Template for .claude/.gitignore
pub(super) fn claude_gitignore_template() -> &'static str {
    r#"# Claude adapter files
context/sessions/
context/active-session.md
context/last-session.md
mcp/

# Keep the structure but ignore session data
!context/.gitkeep
!bin/
"#
}