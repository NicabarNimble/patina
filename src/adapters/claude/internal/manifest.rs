//! Version and manifest management for Claude adapter

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::paths;

/// Version of the Claude adapter - increment when scripts/commands change
pub const CLAUDE_ADAPTER_VERSION: &str = "0.6.0";

/// Changelog for adapter versions
const VERSION_CHANGES: &[(&str, &[&str])] = &[
    (
        "0.6.0",
        &[
            "Major: Refactored to dependable-rust architecture",
            "Major: Drastically reduced .claude/CLAUDE.md size (~100 lines vs 1000+)",
            "Changed: Pattern references instead of full content embedding",
            "Changed: Minimal environment info only",
            "Added: Tip about CLAUDE.local.md for personal notes",
            "Fixed: Avoid context duplication between files",
        ],
    ),
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
];

#[derive(Serialize, Deserialize)]
pub struct AdapterManifest {
    adapter: String,
    version: String,
    installed_at: String,
    files: HashMap<String, String>,
}

/// Create or update the adapter manifest file
pub fn create_adapter_manifest(project_path: &Path) -> Result<()> {
    let manifest = AdapterManifest {
        adapter: "claude".to_string(),
        version: CLAUDE_ADAPTER_VERSION.to_string(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        files: HashMap::new(),
    };

    let manifest_path = paths::get_manifest_path(project_path);
    fs::write(manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    Ok(())
}

/// Check if adapter files need updating
pub fn check_for_updates(project_path: &Path) -> Result<Option<(String, String)>> {
    let manifest_path = paths::get_manifest_path(project_path);

    if !manifest_path.exists() {
        // No manifest means old installation
        return Ok(Some((
            "0.0.0".to_string(),
            CLAUDE_ADAPTER_VERSION.to_string(),
        )));
    }

    let manifest: AdapterManifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;

    if manifest.version != CLAUDE_ADAPTER_VERSION {
        Ok(Some((manifest.version, CLAUDE_ADAPTER_VERSION.to_string())))
    } else {
        Ok(None)
    }
}

/// Get version changes for a specific version
pub fn get_version_changes(version: &str) -> Option<Vec<String>> {
    for (v, changes) in VERSION_CHANGES {
        if *v == version {
            return Some(changes.iter().map(|s| s.to_string()).collect());
        }
    }
    None
}

/// Get all changes since a given version
pub fn get_changelog_since(from_version: &str) -> Vec<String> {
    let mut changes = Vec::new();
    let mut found_version = false;

    for (version, version_changes) in VERSION_CHANGES {
        if found_version {
            changes.extend(version_changes.iter().map(|s| s.to_string()));
        }
        if *version == from_version {
            found_version = true;
        }
    }

    changes
}
