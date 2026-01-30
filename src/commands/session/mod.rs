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
pub fn execute(command: SessionCommands) -> Result<()> {
    match command {
        SessionCommands::Start { title, adapter } => start(&title, adapter.as_deref()),
        SessionCommands::Update => update(),
        SessionCommands::Note { content } => note(&content),
        SessionCommands::End => end(),
    }
}

/// Start a new development session
///
/// Creates git tag, writes active session markdown, writes session.started event.
/// Handles incomplete previous sessions (cleanup/archive).
pub fn start(title: &str, adapter: Option<&str>) -> Result<()> {
    internal::start_session(title, adapter)
}

/// Record progress update with git metrics
///
/// Computes git metrics (commits, files changed, last commit time),
/// appends timestamped update section, writes session.update event.
pub fn update() -> Result<()> {
    internal::update_session()
}

/// Add a timestamped note to the active session
///
/// Appends timestamped note with [branch@sha], writes session.observation event.
pub fn note(content: &str) -> Result<()> {
    internal::note_session(content)
}

/// End the active session
///
/// Creates end tag, computes final metrics, classifies work,
/// archives to layer/sessions/, writes session.ended event.
pub fn end() -> Result<()> {
    internal::end_session()
}
