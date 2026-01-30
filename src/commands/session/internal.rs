//! Internal implementation for session commands
//!
//! All session logic lives here. The public mod.rs exposes only the clean API.

use anyhow::{bail, Result};
use chrono::{Local, Utc};
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

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
    let session_path = project_root.join(ACTIVE_SESSION_PATH);
    let last_update_path = project_root.join(LAST_UPDATE_PATH);

    // 1. Handle incomplete previous session
    if session_path.exists() {
        println!("Found incomplete session, cleaning up...");
        let line_count = fs::read_to_string(&session_path)
            .map(|s| s.lines().count())
            .unwrap_or(0);
        if line_count > 10 {
            // Archive non-trivial session
            if let Ok(old_id) = read_session_id(&session_path) {
                let archive_path = project_root.join(SESSIONS_DIR).join(format!("{}.md", old_id));
                fs::create_dir_all(project_root.join(SESSIONS_DIR))?;
                fs::copy(&session_path, &archive_path)?;
                println!("  Archived to {}/{}.md", SESSIONS_DIR, old_id);
            }
        } else {
            println!("  Removed empty session file");
        }
        fs::remove_file(&session_path)?;
    }

    // 2. Generate session ID and tag
    let now = Local::now();
    let session_id = now.format("%Y%m%d-%H%M%S").to_string();
    let session_tag = format!("session-{}-{}-start", session_id, adapter);

    // 3. Git context
    let branch = git::current_branch().unwrap_or_else(|_| "none".to_string());
    let starting_commit = git::head_sha().unwrap_or_else(|_| "none".to_string());

    // 4. Check for uncommitted changes
    if !git::is_clean().unwrap_or(true) {
        println!("Warning: Uncommitted changes exist");
        println!("  Consider: git stash or git commit -am 'WIP: saving work'");
        println!();
    }

    // 5. Smart branch handling
    if git::is_git_repo().unwrap_or(false) {
        let is_work_related = branch == "work"
            || is_ancestor_of_head("work");

        if !is_work_related {
            if branch == "main" || branch == "master" {
                // Switch to work branch
                if git::branch_exists("work").unwrap_or(false) {
                    git::checkout("work")?;
                    println!("Switched to work branch from {}", branch);
                } else {
                    git::checkout_new_branch("work", &branch)?;
                    println!("Created and switched to work branch from {}", branch);
                }
            } else {
                println!("On unrelated branch: {}", branch);
                println!("  Consider: git checkout work or git checkout -b work/{}", branch);
            }
        } else if branch != "work" {
            println!("Staying on work sub-branch: {}", branch);
        }
    }

    // Re-read branch after potential switch
    let branch = git::current_branch().unwrap_or_else(|_| "none".to_string());

    // 6. Create session tag
    if git::is_git_repo().unwrap_or(false) {
        match git::create_tag(&session_tag, &format!("Session start: {}", title)) {
            Ok(()) => println!("Session tagged: {}", session_tag),
            Err(_) => println!("Could not create tag (may already exist)"),
        }
    }

    // 7. Write active session markdown
    let started_utc = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let start_timestamp = Utc::now().timestamp_millis();
    let time_str = now.format("%H:%M").to_string();

    let scaffold = format!(
        "# Session: {title}
**ID**: {session_id}
**Started**: {started_utc}
**Start Timestamp**: {start_timestamp}
**LLM**: {adapter}
**Git Branch**: {branch}
**Session Tag**: {session_tag}
**Starting Commit**: {starting_commit}

## Previous Session Context
<!-- AI: Summarize the last session from last-session.md -->

## Goals
- [ ] {title}

## Activity Log
### {time_str} - Session Start
Session initialized with goal: {title}
Working on branch: {branch}
Tagged as: {session_tag}

"
    );

    fs::create_dir_all(session_path.parent().unwrap())?;
    fs::write(&session_path, &scaffold)?;

    // 8. Write .last-update marker
    fs::write(&last_update_path, &time_str)?;

    // 9. Write session.started event to eventlog
    let db_path = project_root.join(patina::eventlog::PATINA_DB);
    let conn = patina::eventlog::initialize(&db_path)?;
    let timestamp = now.to_rfc3339();
    let data = json!({
        "session_id": session_id,
        "title": title,
        "adapter": adapter,
        "branch": branch,
        "starting_commit": starting_commit,
        "tag": session_tag,
    });
    patina::eventlog::insert_event(
        &conn,
        "session.started",
        &timestamp,
        &session_id,
        Some(ACTIVE_SESSION_PATH),
        &data.to_string(),
    )?;

    // 10. Console output — session confirmation
    println!("Session started: {}", title);
    println!("  ID: {}", session_id);
    println!("  Branch: {}", branch);
    println!("  Tag: {}", session_tag);

    // Git coaching
    if git::is_git_repo().unwrap_or(false) {
        println!();
        println!("Session Strategy:");
        if branch == "work" {
            println!("- You're on the 'work' branch - all sessions happen here");
        } else {
            println!("- You're on '{}' (work sub-branch) - perfect for isolated experiments", branch);
        }
        println!("- Session tagged as: {}", session_tag);
        println!("- Commit early and often - each commit is a checkpoint");
        println!("- Failed attempts are valuable memory");
    }

    // Beliefs context
    show_beliefs_context(project_root);

    // Previous session beliefs
    show_previous_session_beliefs(project_root);

    // Prompt LLM to fill in context
    println!();
    let last_session_path = project_root.join(LAST_SESSION_PATH);
    if last_session_path.exists() {
        println!("Please read {} and fill in the Previous Session Context section above.", LAST_SESSION_PATH);
    } else {
        println!("No previous session found. Starting fresh.");
    }
    println!("Then ask: 'Would you like me to create todos for \"{}\"?'", title);

    Ok(())
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

/// Check if a branch is an ancestor of HEAD.
fn is_ancestor_of_head(branch: &str) -> bool {
    std::process::Command::new("git")
        .args(["merge-base", "--is-ancestor", branch, "HEAD"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Show beliefs context (count + recent beliefs).
fn show_beliefs_context(project_root: &Path) {
    let beliefs_dir = project_root.join("layer/surface/epistemic/beliefs");
    if !beliefs_dir.is_dir() {
        return;
    }

    let mut belief_files: Vec<PathBuf> = fs::read_dir(&beliefs_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.ends_with(".md") && name != "_index.md"
        })
        .map(|e| e.path())
        .collect();

    if belief_files.is_empty() {
        return;
    }

    // Sort by modification time (newest first)
    belief_files.sort_by(|a, b| {
        let ma = a.metadata().and_then(|m| m.modified()).ok();
        let mb = b.metadata().and_then(|m| m.modified()).ok();
        mb.cmp(&ma)
    });

    println!();
    println!("Epistemic Beliefs: {} total", belief_files.len());
    println!("  Recent beliefs:");
    for path in belief_files.iter().take(5) {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            println!("  - {}", stem);
        }
    }
}

/// Show previous session's captured beliefs.
fn show_previous_session_beliefs(project_root: &Path) {
    let last_session_path = project_root.join(LAST_SESSION_PATH);
    if !last_session_path.exists() {
        return;
    }

    let content = match fs::read_to_string(&last_session_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Extract session file path from "See: layer/sessions/XXXX.md"
    let prev_path = content
        .lines()
        .find_map(|l| l.strip_prefix("See: "))
        .map(|s| s.trim().to_string());
    let prev_title = content
        .lines()
        .find_map(|l| l.strip_prefix("# Last Session: "))
        .map(|s| s.trim().to_string());

    let (Some(prev_path), Some(prev_title)) = (prev_path, prev_title) else {
        return;
    };

    let full_path = project_root.join(&prev_path);
    let prev_content = match fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Find "## Beliefs Captured: N"
    let beliefs_line = prev_content
        .lines()
        .find(|l| l.starts_with("## Beliefs Captured:"));

    if let Some(line) = beliefs_line {
        let count_str = line.trim_start_matches("## Beliefs Captured:").trim();
        let count: usize = count_str.parse().unwrap_or(0);
        println!();
        if count > 0 {
            println!(
                "Previous session \"{}\" captured {} belief(s):",
                prev_title, count
            );
            // Extract belief list items between "## Beliefs Captured:" and next "##"
            let mut in_section = false;
            for line in prev_content.lines() {
                if line.starts_with("## Beliefs Captured:") {
                    in_section = true;
                    continue;
                }
                if in_section && line.starts_with("## ") {
                    break;
                }
                if in_section && line.trim_start().starts_with('-') {
                    println!("{}", line);
                }
            }
        } else {
            println!(
                "Previous session \"{}\": no beliefs captured",
                prev_title
            );
        }
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

