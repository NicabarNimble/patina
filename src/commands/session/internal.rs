//! Internal implementation for session commands
//!
//! All session logic lives here. The public mod.rs exposes only the clean API.

use anyhow::{bail, Result};
use chrono::Local;
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use patina::git;

/// Path to active session file (transient, gitignored)
const ACTIVE_SESSION_PATH: &str = ".patina/local/active-session.md";

/// Path to last session pointer (transient, gitignored)
#[allow(dead_code)]
const LAST_SESSION_PATH: &str = ".patina/local/last-session.md";

/// Directory for archived session files (committed)
#[allow(dead_code)]
const SESSIONS_DIR: &str = "layer/sessions";

/// Importance keywords that suggest a checkpoint commit
const IMPORTANCE_KEYWORDS: &[&str] = &["breakthrough", "discovered", "solved", "fixed", "important"];

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
    let session_path = project_root.join(ACTIVE_SESSION_PATH);

    // 1. Validate active session exists
    if !session_path.exists() {
        bail!("No active session found at {}\nStart one with: patina session start \"<title>\"", ACTIVE_SESSION_PATH);
    }

    // 2. Get git context
    let branch = git::current_branch().unwrap_or_else(|_| "detached".to_string());
    let sha = git::short_sha().unwrap_or_else(|_| "no-commits".to_string());
    let git_context = format!("[{}@{}]", branch, sha);

    // 3. Append timestamped note to active session markdown
    let now = Local::now();
    let time_str = now.format("%H:%M").to_string();
    let note_section = format!("\n### {} - Note {}\n{}\n", time_str, git_context, content);

    let mut file = OpenOptions::new()
        .append(true)
        .open(&session_path)?;
    file.write_all(note_section.as_bytes())?;

    // 4. Write session.observation event to eventlog
    //    Read session ID from the active session file for the source_id
    let session_id = read_session_id(&session_path)?;
    let db_path = project_root.join(patina::eventlog::PATINA_DB);
    let conn = patina::eventlog::initialize(&db_path)?;
    let timestamp = now.to_rfc3339();
    let data = json!({
        "session_id": session_id,
        "content": content,
        "branch": branch,
        "sha": sha,
    });
    patina::eventlog::insert_event(
        &conn,
        "session.observation",
        &timestamp,
        &session_id,
        Some(ACTIVE_SESSION_PATH),
        &data.to_string(),
    )?;

    // 5. Output confirmation
    println!("Note added to session {}", git_context);

    // 6. Detect importance keywords, suggest checkpoint commit
    let content_lower = content.to_lowercase();
    if IMPORTANCE_KEYWORDS.iter().any(|kw| content_lower.contains(kw)) {
        println!();
        println!("Important insight detected!");
        println!("  Consider committing current work to preserve this context:");
        println!("  git commit -am \"checkpoint: {}\"", truncate(content, 60));
    }

    Ok(())
}

pub fn end_session(project_root: &Path) -> Result<()> {
    let _root = project_root; // will be used in step 6
    println!("patina session end");
    bail!("not yet implemented — step 6 in build order")
}

/// Read session ID from active session markdown.
///
/// Looks for `**ID**: <value>` in the frontmatter area.
fn read_session_id(session_path: &Path) -> Result<String> {
    let contents = fs::read_to_string(session_path)?;
    for line in contents.lines() {
        if let Some(id) = line.strip_prefix("**ID**: ") {
            return Ok(id.trim().to_string());
        }
    }
    bail!("Could not find session ID in {}", session_path.display())
}

/// Truncate a string to max_len, appending "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
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

