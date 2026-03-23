//! Version management for Patina projects
//!
//! This module follows the dependable-rust pattern:
//! - Public interface (this file): clean API for version operations
//! - Internal implementation: all logic in internal.rs
//!
//! # Versioning Model
//!
//! Patina uses semver: `MAJOR.MINOR.PATCH`
//!
//! Version bumps are driven by spec completion (`patina spec complete <id>`)
//! via `ReleaseStrategy`. The version command provides `show` (display) and
//! `hotfix` (emergency escape hatch).
//!
//! # Example
//!
//! ```no_run
//! use patina::commands::version;
//!
//! // Show current version
//! version::show(false, false).expect("Failed to show version");
//!
//! // Emergency patch (Cargo strategy only)
//! version::hotfix("fix critical auth bypass").expect("Failed to hotfix");
//! ```

mod internal;

use anyhow::Result;

/// Version CLI subcommands (used by main.rs via clap)
#[derive(Debug, Clone, clap::Subcommand)]
pub enum VersionCommands {
    /// Show current version (default)
    Show {
        /// Output as JSON
        #[arg(short, long)]
        json: bool,

        /// Show component versions (git, external tools)
        #[arg(short, long)]
        components: bool,
    },

    /// Emergency patch bump without spec ceremony (Cargo strategy only)
    Hotfix {
        /// Description for the hotfix
        description: String,
    },
}

/// Execute version command from CLI arguments
///
/// Handles both subcommand form (`patina version show`) and
/// bare form (`patina version` defaults to show).
pub fn execute(json: bool, components: bool) -> Result<()> {
    show(json, components)
}

/// Execute version subcommand
pub fn execute_subcommand(command: VersionCommands) -> Result<()> {
    match command {
        VersionCommands::Show { json, components } => show(json, components),
        VersionCommands::Hotfix { description } => hotfix(&description),
    }
}

/// Show current version information
///
/// Displays the current version from Cargo.toml.
/// With `--components`, also shows git info and external tool versions.
pub fn show(json: bool, components: bool) -> Result<()> {
    internal::show_version(json, components)
}

/// Emergency patch bump without spec ceremony
///
/// Uses `ReleaseStrategy::preflight()` → `PreparedRelease::execute()` with
/// `BumpType::Patch`. Same safeguards as spec-driven releases. Cargo-only.
///
/// Prints a reminder to create a spec for traceability.
pub fn hotfix(description: &str) -> Result<()> {
    internal::hotfix(description)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_commands_variants() {
        let show = VersionCommands::Show {
            json: false,
            components: true,
        };
        assert!(matches!(show, VersionCommands::Show { .. }));

        let hotfix = VersionCommands::Hotfix {
            description: "fix critical bug".to_string(),
        };
        assert!(matches!(hotfix, VersionCommands::Hotfix { .. }));
    }
}
