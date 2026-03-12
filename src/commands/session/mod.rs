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

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Record progress update with git metrics
    Update {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a timestamped note to the active session
    Note {
        /// Note content (e.g., "discovered edge case in parser")
        content: String,
    },

    /// End the active session (tag, classify, archive)
    End {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List active, stale, and recent sessions
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Execute a session subcommand
///
/// Resolves project root once, then threads it to all internal functions.
pub fn execute(command: SessionCommands) -> Result<()> {
    let project_root = std::env::current_dir()?;

    match command {
        SessionCommands::Start {
            title,
            adapter,
            json,
        } => start(&project_root, &title, adapter.as_deref(), json),
        SessionCommands::Update { json } => update(&project_root, json),
        SessionCommands::Note { content } => note(&project_root, &content),
        SessionCommands::End { json } => end(&project_root, json),
        SessionCommands::List { json } => list(&project_root, json),
    }
}

pub(crate) use internal::{
    end_live_session_value, resolve_live_session, start_session_value, update_live_session_value,
    SessionStartRequest,
};

/// Start a new development session
///
/// Creates git tag, writes active session markdown, writes session.started event.
/// Handles incomplete previous sessions (cleanup/archive).
pub fn start(project_root: &Path, title: &str, adapter: Option<&str>, json: bool) -> Result<()> {
    if json {
        let result = internal::start_session_value(
            project_root,
            SessionStartRequest::compatibility_cli(title, adapter),
        )?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        Ok(())
    } else {
        internal::start_session(project_root, title, adapter)
    }
}

/// Record progress update with git metrics
///
/// Computes git metrics (commits, files changed, last commit time),
/// appends timestamped update section, writes session.update event.
pub fn update(project_root: &Path, json: bool) -> Result<()> {
    if json {
        let result = internal::update_session_json(project_root)?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        Ok(())
    } else {
        internal::update_session(project_root)
    }
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
pub fn end(project_root: &Path, json: bool) -> Result<()> {
    if json {
        let result = internal::end_session_json(project_root)?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        Ok(())
    } else {
        internal::end_session(project_root)
    }
}

/// List active, stale, and recent sessions
///
/// Shows active session with age, stale sessions (>24h, never ended),
/// and recent completed sessions.
pub fn list(project_root: &Path, json: bool) -> Result<()> {
    internal::list_sessions(project_root, json)
}
