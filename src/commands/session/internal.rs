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
const LAST_SESSION_PATH: &str = ".patina/local/last-session.md";

/// Path to last update timestamp (transient, gitignored)
const LAST_UPDATE_PATH: &str = ".patina/local/.last-update";

/// Directory for archived session files (committed)
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
    let session_path = project_root.join(ACTIVE_SESSION_PATH);
    let last_update_path = project_root.join(LAST_UPDATE_PATH);

    // 1. Validate active session exists
    if !session_path.exists() {
        bail!(
            "No active session found at {}\nStart one with: patina session start \"<title>\"",
            ACTIVE_SESSION_PATH
        );
    }

    // 2. Read session metadata
    let session_id = read_session_id(&session_path)?;
    let starting_commit = read_session_field(&session_path, "**Starting Commit**: ")?;

    // 3. Read last update time (or "session start" if first update)
    let last_update = fs::read_to_string(&last_update_path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "session start".to_string());

    // 4. Compute git metrics
    let branch = git::current_branch().unwrap_or_else(|_| "detached".to_string());
    let commits_this_session = git::commits_since_count(&starting_commit).unwrap_or(0);
    let last_commit_time = git::last_commit_relative_time()
        .unwrap_or_else(|_| "never".to_string());
    let last_commit_msg = git::last_commit_message()
        .unwrap_or_else(|_| "no commits yet".to_string());

    // Parse working tree status
    let porcelain = git::status_porcelain().unwrap_or_default();
    let modified = porcelain.lines().filter(|l| l.starts_with(" M")).count();
    let staged = porcelain.lines().filter(|l| l.starts_with('M')).count();
    let untracked = porcelain.lines().filter(|l| l.starts_with("??")).count();
    let total_changes = modified + staged + untracked;

    // Get diff stat for lines changed
    let diff_summary = git::diff_stat_summary().unwrap_or_default();
    let lines_changed = parse_insertions(&diff_summary);

    // 5. Console output — git status summary
    println!("Git Status Check");
    println!();
    println!("Current branch: {}", branch);

    let recent = git::log_oneline(5).unwrap_or_default();
    if !recent.is_empty() {
        println!();
        println!("Recent commits:");
        for line in recent.lines() {
            println!("  {}", line);
        }
    }

    println!();
    println!("Working tree status:");
    if total_changes == 0 {
        println!("  Clean working tree - all changes committed");
        println!("  Last commit: {} - {}", last_commit_time, last_commit_msg);
    } else {
        println!("  Modified files: {}", modified);
        println!("  Staged files: {}", staged);
        println!("  Untracked files: {}", untracked);
        println!("  Lines changed: ~{}", lines_changed);
        println!("  Last commit: {}", last_commit_time);

        // Commit coaching
        println!();
        if last_commit_time.contains("hour") {
            println!("  Last commit was {}", last_commit_time);
            println!("  Strong recommendation: Commit your work soon");
            println!(
                "  Suggested: git add -p && git commit -m \"checkpoint: progress on session goals\""
            );
        } else if lines_changed > 100 {
            println!(
                "  Large changes detected ({}+ lines)",
                lines_changed
            );
            println!("  Consider: Breaking into smaller commits");
            println!("  Use: git add -p to stage selectively");
        }
    }

    // Changes summary
    if !diff_summary.is_empty() {
        println!();
        println!("Changes summary:");
        println!("  {}", diff_summary);
    }

    // Session health
    println!();
    if total_changes == 0 {
        println!("Session Health: Excellent (clean working tree)");
    } else if last_commit_time.contains("hour") {
        println!("Session Health: Good (commit recommended)");
    } else {
        println!("Session Health: Good (active development)");
    }

    // 6. Append update section to active session markdown
    let now = Local::now();
    let time_str = now.format("%H:%M").to_string();
    let mut update_section = format!(
        "\n### {} - Update (covering since {})\n",
        time_str, last_update
    );
    update_section.push_str("\n**Git Activity:**\n");
    update_section.push_str(&format!(
        "- Commits this session: {}\n",
        commits_this_session
    ));
    update_section.push_str(&format!("- Files changed: {}\n", total_changes));
    update_section.push_str(&format!("- Last commit: {}\n", last_commit_time));
    update_section.push('\n');

    let mut file = OpenOptions::new().append(true).open(&session_path)?;
    file.write_all(update_section.as_bytes())?;

    // 7. Update last update timestamp
    fs::write(&last_update_path, &time_str)?;

    // 8. Write session.update event to eventlog
    let db_path = project_root.join(patina::eventlog::PATINA_DB);
    let conn = patina::eventlog::initialize(&db_path)?;
    let timestamp = now.to_rfc3339();
    let data = json!({
        "session_id": session_id,
        "commits_this_session": commits_this_session,
        "files_changed": total_changes,
        "last_commit_time": last_commit_time,
        "lines_changed": lines_changed,
        "branch": branch,
    });
    patina::eventlog::insert_event(
        &conn,
        "session.update",
        &timestamp,
        &session_id,
        Some(ACTIVE_SESSION_PATH),
        &data.to_string(),
    )?;

    // 9. Prompt for LLM to fill in
    println!();
    println!("Please fill in the update section in active-session.md with:");
    println!("- Work completed since {}", last_update);
    println!("- Key decisions and reasoning");
    println!("- Patterns observed");
    println!();
    println!("Update marker added: {} -> {}", last_update, time_str);

    Ok(())
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
    read_session_field(session_path, "**ID**: ")
}

/// Read a field value from active session markdown.
///
/// Looks for lines matching `prefix<value>` and returns the value.
fn read_session_field(session_path: &Path, prefix: &str) -> Result<String> {
    let contents = fs::read_to_string(session_path)?;
    for line in contents.lines() {
        if let Some(value) = line.strip_prefix(prefix) {
            return Ok(value.trim().to_string());
        }
    }
    bail!(
        "Could not find '{}' in {}",
        prefix.trim(),
        session_path.display()
    )
}

/// Parse insertion count from git diff --stat summary line.
///
/// Input like "3 files changed, 45 insertions(+), 10 deletions(-)" → 45
fn parse_insertions(summary: &str) -> usize {
    summary
        .split(',')
        .find(|s| s.contains("insertion"))
        .and_then(|s| s.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
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

