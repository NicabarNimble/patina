//! Version management for Claude adapter
//!
//! This module handles version tracking and changelog for the Claude adapter.
//! It's separated from the main implementation to keep concerns isolated.

/// Version of the Claude adapter - increment when scripts/commands change
pub const CLAUDE_ADAPTER_VERSION: &str = "0.5.0";

/// Changelog for adapter versions
pub(super) const VERSION_CHANGES: &[(&str, &[&str])] = &[
    (
        "0.5.0",
        &[
            "Simplified: Removed content-capture agent references",
            "Fixed: session-start now always prompts for todos",
            "Changed: session-update now prompts directly for content",
            "Changed: session-end no longer auto-runs update (manual control)",
            "Updated: All markdown templates for clarity and consistency",
            "Improved: Cleaner separation between script actions and Claude actions",
        ],
    ),
    (
        "0.4.0",
        &[
            "New: Active session management with active-session.md",
            "New: Content-capture agent for automatic session documentation",
            "Enhanced: session-start now cleans up incomplete sessions",
            "Enhanced: session-update invokes agent for automatic content capture",
            "Enhanced: session-end invokes agent before archiving",
            "Fixed: All commands now use consistent active-session.md file",
            "Changed: Session workflow is now stateful with AI participation",
        ],
    ),
    (
        "0.3.0",
        &[
            "Enhanced: session-update now captures interest marks",
            "Fixed: Arguments are now properly used by session-update script",
            "Improved: Unified workflow for marking + context filling",
            "Changed: Both marks and context are captured for richer sessions",
        ],
    ),
    (
        "0.2.0",
        &[
            "Fixed: Scripts now properly stored in .claude/bin/ directory",
            "Added: Adapter versioning and update mechanism",
            "Changed: Update command now updates adapter files instead of CLAUDE.md",
            "Improved: Session commands now use correct script paths",
        ],
    ),
    (
        "0.1.0",
        &[
            "Initial Claude adapter implementation",
            "Session management commands",
            "CLAUDE.md context generation",
        ],
    ),
];

/// Get version changes for a specific version
pub(super) fn get_version_changes(version: &str) -> Option<Vec<String>> {
    VERSION_CHANGES
        .iter()
        .find(|(v, _)| *v == version)
        .map(|(_, changes)| changes.iter().map(|s| s.to_string()).collect())
}

/// Get all changes since a given version
pub(super) fn get_changelog_since(from_version: &str) -> Vec<String> {
    let mut changes = Vec::new();
    let mut found_version = false;

    for (version, version_changes) in VERSION_CHANGES {
        if *version == from_version {
            found_version = true;
            break;
        }
        for change in *version_changes {
            changes.push(format!("{version}: {change}"));
        }
    }

    if !found_version && from_version != "0.0.0" {
        // Version not found, return all changes
        changes.clear();
        for (version, version_changes) in VERSION_CHANGES {
            for change in *version_changes {
                changes.push(format!("{version}: {change}"));
            }
        }
    }

    changes
}
