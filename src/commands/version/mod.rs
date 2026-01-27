//! Version management for Patina projects
//!
//! This module follows the dependable-rust pattern:
//! - Public interface (this file): clean API for version operations
//! - Internal implementation: all logic in internal.rs
//!
//! # Versioning Model
//!
//! Patina uses Phase.Milestone versioning: `MAJOR.PHASE.MILESTONE`
//!
//! - **MAJOR**: Production-ready declaration (0.x = pre-production)
//! - **PHASE**: Major development era (new theme = phase bump)
//! - **MILESTONE**: Significant completion within a phase
//!
//! # Example
//!
//! ```no_run
//! use patina::commands::version;
//!
//! // Show current version
//! version::show(false).expect("Failed to show version");
//!
//! // Bump milestone (0.8.1 -> 0.8.2)
//! version::milestone("Implemented feature X").expect("Failed to bump milestone");
//!
//! // Start new phase (0.8.x -> 0.9.0)
//! version::phase("Production Ready").expect("Failed to start phase");
//! ```

mod internal;

use anyhow::Result;

/// Version CLI subcommands (used by main.rs via clap)
#[derive(Debug, Clone, clap::Subcommand)]
pub enum VersionCommands {
    /// Show current version, phase, and milestone (default)
    Show {
        /// Output as JSON
        #[arg(short, long)]
        json: bool,

        /// Show component versions (git, external tools)
        #[arg(short, long)]
        components: bool,
    },

    /// Complete current spec milestone and bump version
    Milestone {
        /// Override description (default: from spec milestone name)
        #[arg(short, long)]
        description: Option<String>,

        /// Skip creating git tag
        #[arg(long)]
        no_tag: bool,

        /// Dry run - show what would change without modifying files
        #[arg(long)]
        dry_run: bool,
    },

    /// Start a new development phase (0.8.x -> 0.9.0)
    Phase {
        /// Name of the new phase
        name: String,

        /// Skip creating git tag
        #[arg(long)]
        no_tag: bool,

        /// Dry run - show what would change without modifying files
        #[arg(long)]
        dry_run: bool,
    },

    /// Initialize version tracking for this project
    Init {
        /// Phase number to start at (default: 1)
        #[arg(long, default_value = "1")]
        phase: u32,

        /// Phase name
        #[arg(long, default_value = "Initial")]
        phase_name: String,

        /// Milestone number to start at (default: 0)
        #[arg(long, default_value = "0")]
        milestone: u32,
    },
}

/// Execute version command from CLI arguments
///
/// Handles both subcommand form (`patina version show`) and
/// bare form (`patina version` defaults to show).
pub fn execute(json: bool, components: bool) -> Result<()> {
    // Default behavior: show version (backwards compatible)
    show(json, components)
}

/// Execute version subcommand
pub fn execute_subcommand(command: VersionCommands) -> Result<()> {
    match command {
        VersionCommands::Show { json, components } => show(json, components),
        VersionCommands::Milestone {
            description,
            no_tag,
            dry_run,
        } => milestone(description.as_deref(), no_tag, dry_run),
        VersionCommands::Phase {
            name,
            no_tag,
            dry_run,
        } => phase(&name, no_tag, dry_run),
        VersionCommands::Init {
            phase,
            phase_name,
            milestone,
        } => init(phase, &phase_name, milestone),
    }
}

/// Show current version information
///
/// Displays the current version from Cargo.toml along with phase
/// and milestone information from `.patina/version.toml`.
///
/// With `--components`, also shows git info and external tool versions.
pub fn show(json: bool, components: bool) -> Result<()> {
    internal::show_version(json, components)
}

/// Complete current spec milestone and bump version
///
/// Reads current milestone from spec (via index) and:
/// - Marks it complete in spec YAML
/// - Advances current_milestone to next pending
/// - Updates `Cargo.toml` version (if owned repo)
/// - Re-scrapes layer to sync index
/// - Creates annotated git tag (unless `--no-tag`)
///
/// # Arguments
///
/// * `description` - Override description (default: from spec milestone name)
/// * `no_tag` - Skip creating git tag
/// * `dry_run` - Show changes without writing files
pub fn milestone(description: Option<&str>, no_tag: bool, dry_run: bool) -> Result<()> {
    internal::bump_milestone(description, no_tag, dry_run)
}

/// Start a new development phase
///
/// Increments the phase number and resets milestone to 0 (0.8.5 -> 0.9.0):
/// - Updates `.patina/version.toml` with new phase name
/// - Updates `Cargo.toml` version field
/// - Appends entry to version history
/// - Creates annotated git tag (unless `--no-tag`)
///
/// # Arguments
///
/// * `name` - Name of the new phase (e.g., "Production Ready")
/// * `no_tag` - Skip creating git tag
/// * `dry_run` - Show changes without writing files
pub fn phase(name: &str, no_tag: bool, dry_run: bool) -> Result<()> {
    internal::bump_phase(name, no_tag, dry_run)
}

/// Initialize version tracking for a project
///
/// Creates `.patina/version.toml` with initial state.
/// Reads current version from Cargo.toml if present.
pub fn init(phase: u32, phase_name: &str, milestone: u32) -> Result<()> {
    internal::init_version(phase, phase_name, milestone)
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

        let milestone = VersionCommands::Milestone {
            description: Some("Test milestone".to_string()),
            no_tag: false,
            dry_run: true,
        };
        assert!(matches!(milestone, VersionCommands::Milestone { .. }));
    }
}
