//! Internal implementation for session commands
//!
//! All session logic lives here. The public mod.rs exposes only the clean API.

use anyhow::{bail, Result};
use std::path::Path;

/// Path to active session file (transient, gitignored)
#[allow(dead_code)]
const ACTIVE_SESSION_PATH: &str = ".patina/local/active-session.md";

/// Path to last session pointer (transient, gitignored)
#[allow(dead_code)]
const LAST_SESSION_PATH: &str = ".patina/local/last-session.md";

/// Directory for archived session files (committed)
#[allow(dead_code)]
const SESSIONS_DIR: &str = "layer/sessions";

pub fn start_session(
    project_root: &Path,
    title: &str,
    adapter: Option<&str>,
) -> Result<()> {
    let adapter = resolve_adapter(adapter, project_root)?;
    println!(
        "patina session start: title={:?}, adapter={:?}",
        title, adapter
    );
    bail!("not yet implemented — step 5 in build order")
}

pub fn update_session(project_root: &Path) -> Result<()> {
    let _root = project_root; // will be used in step 4
    println!("patina session update");
    bail!("not yet implemented — step 4 in build order")
}

pub fn note_session(project_root: &Path, content: &str) -> Result<()> {
    let _root = project_root; // will be used in step 3
    println!("patina session note: {:?}", content);
    bail!("not yet implemented — step 3 in build order")
}

pub fn end_session(project_root: &Path) -> Result<()> {
    let _root = project_root; // will be used in step 6
    println!("patina session end");
    bail!("not yet implemented — step 6 in build order")
}

/// Resolve adapter name from explicit flag or project config.
///
/// Resolution chain: --adapter flag > config.adapters.default.
/// Function signature is honest about dependencies (Jon Gjengset principle).
pub fn resolve_adapter(explicit: Option<&str>, project_root: &Path) -> Result<String> {
    if let Some(name) = explicit {
        return Ok(name.to_string());
    }

    let config = patina::project::load(project_root)?;
    Ok(config.adapters.default)
}

