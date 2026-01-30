//! Session lifecycle management for Patina projects
//!
//! Replaces ~640 lines of bash (session-{start,update,note,end}.sh) with native
//! Rust commands. Sessions track development work with dual-write: markdown is the
//! collaboration artifact (LLM reads/writes), events are the structured query layer.
//!
//! This module follows the dependable-rust pattern:
//! - Public interface (this file): clean API + clap subcommands
//! - Internal implementation: all logic in internal.rs

mod internal;

use anyhow::Result;
use std::path::Path;

/// Session CLI subcommands (used by main.rs via clap)
#[derive(Debug, Clone, clap::Subcommand)]
pub enum SessionCommands {
    /// Start a new development session
    Start {
        /// Session title (e.g., "complete 0.9.2")
        title: String,

        /// LLM adapter override (default: from config.adapters.default)
        #[arg(long)]
        adapter: Option<String>,
    },

    /// Record progress update with git metrics
    Update,

    /// Add a timestamped note to the active session
    Note {
        /// Note content (e.g., "discovered edge case in parser")
        content: String,
    },

    /// End the active session (tag, classify, archive)
    End,
}

/// Execute a session subcommand
///
/// Resolves project root once, then threads it to all internal functions.
pub fn execute(command: SessionCommands) -> Result<()> {
    let project_root = std::env::current_dir()?;

    match command {
        SessionCommands::Start { title, adapter } => {
            start(&project_root, &title, adapter.as_deref())
        }
        SessionCommands::Update => update(&project_root),
        SessionCommands::Note { content } => note(&project_root, &content),
        SessionCommands::End => end(&project_root),
    }
}

/// Start a new development session
///
/// Creates git tag, writes active session markdown, writes session.started event.
/// Handles incomplete previous sessions (cleanup/archive).
pub fn start(project_root: &Path, title: &str, adapter: Option<&str>) -> Result<()> {
    internal::start_session(project_root, title, adapter)
}

/// Record progress update with git metrics
///
/// Computes git metrics (commits, files changed, last commit time),
/// appends timestamped update section, writes session.update event.
pub fn update(project_root: &Path) -> Result<()> {
    internal::update_session(project_root)
}

/// Add a timestamped note to the active session
///
/// Appends timestamped note with [branch@sha], writes session.observation event.
pub fn note(project_root: &Path, content: &str) -> Result<()> {
    internal::note_session(project_root, content)
}

/// End the active session
///
/// Creates end tag, computes final metrics, classifies work,
/// archives to layer/sessions/, writes session.ended event.
pub fn end(project_root: &Path) -> Result<()> {
    internal::end_session(project_root)
}
