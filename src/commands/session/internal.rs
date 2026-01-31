//! Internal implementation for session commands
//!
//! All session logic lives here. The public mod.rs exposes only the clean API.

use anyhow::{bail, Result};
use chrono::{Local, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, Write};
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
const IMPORTANCE_KEYWORDS: &[&str] =
    &["breakthrough", "discovered", "solved", "fixed", "important"];

/// YAML frontmatter for session documents.
///
/// New sessions (step 7+) write this as `---\n<yaml>\n---` at the top of the
/// markdown file. Legacy sessions use `**Field**: value` lines instead.
/// `read_session_field` handles both formats transparently.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionFrontmatter {
    r#type: String,
    id: String,
    title: String,
    status: String,
    llm: String,
    created: String,
    start_timestamp: i64,
    git: SessionGit,
}

/// Git context embedded in session YAML frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionGit {
    branch: String,
    starting_commit: String,
    start_tag: String,
}

pub fn start_session(project_root: &Path, title: &str, adapter: Option<&str>) -> Result<()> {
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
                let archive_path = project_root
                    .join(SESSIONS_DIR)
                    .join(format!("{}.md", old_id));
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
        let is_work_related = branch == "work" || is_ancestor_of_head("work");

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
                println!(
                    "  Consider: git checkout work or git checkout -b work/{}",
                    branch
                );
            }
        } else if branch != "work" {
            println!("Staying on work sub-branch: {}", branch);
        }
    }

    // Re-read branch after potential switch
    let branch = git::current_branch().unwrap_or_else(|_| "none".to_string());

    // 6. Create session tag (detect collision from rapid starts)
    if git::is_git_repo().unwrap_or(false) {
        if git::tag_exists(&session_tag).unwrap_or(false) {
            bail!(
                "Session tag {} already exists.\n\
                 A session with this ID was already started. Wait a moment and retry.",
                session_tag
            );
        }
        match git::create_tag(&session_tag, &format!("Session start: {}", title)) {
            Ok(()) => println!("Session tagged: {}", session_tag),
            Err(e) => bail!("Failed to create session tag {}: {}", session_tag, e),
        }
    }

    // 7. Write active session markdown with YAML frontmatter
    let started_utc = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let start_timestamp = Utc::now().timestamp_millis();
    let time_str = now.format("%H:%M").to_string();

    let frontmatter = SessionFrontmatter {
        r#type: "session".to_string(),
        id: session_id.clone(),
        title: title.to_string(),
        status: "active".to_string(),
        llm: adapter.clone(),
        created: started_utc,
        start_timestamp,
        git: SessionGit {
            branch: branch.clone(),
            starting_commit: starting_commit.clone(),
            start_tag: session_tag.clone(),
        },
    };
    let yaml = serde_yaml::to_string(&frontmatter)?;

    let scaffold = format!(
        "---\n{yaml}---\n\n\
         ## Previous Session Context\n\
         <!-- AI: Summarize the last session from last-session.md -->\n\n\
         ## Goals\n\
         - [ ] {title}\n\n\
         ## Activity Log\n\
         ### {time_str} - Session Start\n\
         Session initialized with goal: {title}\n\
         Working on branch: {branch}\n\
         Tagged as: {session_tag}\n\n",
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
            println!(
                "- You're on '{}' (work sub-branch) - perfect for isolated experiments",
                branch
            );
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
        println!(
            "Please read {} and fill in the Previous Session Context section above.",
            LAST_SESSION_PATH
        );
    } else {
        println!("No previous session found. Starting fresh.");
    }
    println!(
        "Then ask: 'Would you like me to create todos for \"{}\"?'",
        title
    );

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
    let last_commit_time = git::last_commit_relative_time().unwrap_or_else(|_| "never".to_string());
    let last_commit_msg =
        git::last_commit_message().unwrap_or_else(|_| "no commits yet".to_string());

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
            println!("  Large changes detected ({}+ lines)", lines_changed);
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
        bail!(
            "No active session found at {}\nStart one with: patina session start \"<title>\"",
            ACTIVE_SESSION_PATH
        );
    }

    // 2. Get git context
    let branch = git::current_branch().unwrap_or_else(|_| "detached".to_string());
    let sha = git::short_sha().unwrap_or_else(|_| "no-commits".to_string());
    let git_context = format!("[{}@{}]", branch, sha);

    // 3. Append timestamped note to active session markdown
    let now = Local::now();
    let time_str = now.format("%H:%M").to_string();
    let note_section = format!("\n### {} - Note {}\n{}\n", time_str, git_context, content);

    let mut file = OpenOptions::new().append(true).open(&session_path)?;
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
    if IMPORTANCE_KEYWORDS
        .iter()
        .any(|kw| content_lower.contains(kw))
    {
        println!();
        println!("Important insight detected!");
        println!("  Consider committing current work to preserve this context:");
        println!("  git commit -am \"checkpoint: {}\"", truncate(content, 60));
    }

    Ok(())
}

pub fn end_session(project_root: &Path) -> Result<()> {
    let session_path = project_root.join(ACTIVE_SESSION_PATH);
    let last_update_path = project_root.join(LAST_UPDATE_PATH);
    let last_session_path = project_root.join(LAST_SESSION_PATH);

    // 1. Validate active session exists
    if !session_path.exists() {
        bail!(
            "No active session found at {}\nStart one with: patina session start \"<title>\"",
            ACTIVE_SESSION_PATH
        );
    }

    // 2. Read session metadata
    let session_id = read_session_id(&session_path)?;
    let session_title = read_session_field(&session_path, "# Session: ")?;
    let session_tag = read_session_field(&session_path, "**Session Tag**: ")?;
    let starting_commit = read_session_field(&session_path, "**Starting Commit**: ")?;
    let adapter = read_session_field(&session_path, "**LLM**: ")?;

    // 3. Create end session tag
    let end_tag = format!("session-{}-{}-end", session_id, adapter);
    if git::is_git_repo().unwrap_or(false) {
        match git::create_tag(&end_tag, &format!("Session end: {}", session_title)) {
            Ok(()) => println!("✅ Session end tagged: {}", end_tag),
            Err(_) => println!("⚠️  Could not create end tag (may already exist)"),
        }
    }

    // 4. Compute final metrics
    let changed_files = git::files_changed_since(&session_tag).unwrap_or_default();
    let files_changed = changed_files.len();
    let commits_made = git::commits_since_count(&starting_commit).unwrap_or(0);
    let patterns_modified = changed_files
        .iter()
        .filter(|f| f.starts_with("layer/") || f.ends_with(".md"))
        .count();

    // 5. Classify work type
    let classification = classify_work(commits_made, files_changed, patterns_modified);

    // 6. Check for uncommitted changes
    let uncommitted = git::status_count().unwrap_or(0);
    if uncommitted > 0 {
        println!();
        println!("⚠️  Uncommitted changes detected!");
        println!("   You have {} uncommitted files", uncommitted);
        println!("   Strongly recommend: Commit or stash before ending session");
    }

    // 7. Console output — session summary
    let branch = git::current_branch().unwrap_or_else(|_| "none".to_string());
    println!();
    println!("═══ Session Summary ═══");
    println!();
    println!("Working branch: {}", branch);
    println!("Session range: {}..{}", session_tag, end_tag);
    println!();
    println!("Session Metrics:");
    println!("- Files changed: {}", files_changed);
    println!("- Commits made: {}", commits_made);
    println!("- Patterns touched: {}", patterns_modified);
    println!("- Classification: {}", classification_label(classification));
    println!();
    println!("Session Preserved:");
    println!("View session work: git log {}..{}", session_tag, end_tag);
    println!("Diff session: git diff {}..{}", session_tag, end_tag);
    println!(
        "Cherry-pick to main: git cherry-pick {}..{}",
        session_tag, end_tag
    );

    // 8. Count beliefs captured during this session
    let (beliefs_captured, beliefs_summary) = count_beliefs_captured(project_root, &changed_files);
    println!();
    println!("Beliefs Captured: {}", beliefs_captured);
    if !beliefs_summary.is_empty() {
        for line in &beliefs_summary {
            println!("{}", line);
        }
    }

    // 9. Append beliefs section to active session markdown
    let mut appendix = String::new();
    appendix.push_str(&format!("\n## Beliefs Captured: {}\n", beliefs_captured));
    if beliefs_captured > 0 {
        for line in &beliefs_summary {
            appendix.push_str(&format!("{}\n", line));
        }
    } else {
        appendix.push_str("_No beliefs captured this session_\n");
    }

    // 10. Append classification section
    appendix.push_str("\n## Session Classification\n");
    appendix.push_str(&format!("- Work Type: {}\n", classification));
    appendix.push_str(&format!("- Files Changed: {}\n", files_changed));
    appendix.push_str(&format!("- Commits: {}\n", commits_made));
    appendix.push_str(&format!("- Patterns Modified: {}\n", patterns_modified));
    appendix.push_str(&format!("- Beliefs Captured: {}\n", beliefs_captured));
    appendix.push_str(&format!("- Session Tags: {}..{}\n", session_tag, end_tag));

    // 11. Extract user prompts from history.jsonl (if available)
    let prompts = extract_user_prompts(project_root, &session_path);
    if !prompts.is_empty() {
        appendix.push_str(&format!("\n## User Prompts ({})\n\n", prompts.len()));
        for (i, prompt) in prompts.iter().enumerate() {
            let display = truncate(prompt, 97);
            let display = display.replace('`', "\\`");
            appendix.push_str(&format!("{}. `{}`\n", i + 1, display));
        }
        println!("✅ Captured {} user prompts", prompts.len());
    }

    // 12. Write appendix to active session
    {
        let mut file = OpenOptions::new().append(true).open(&session_path)?;
        file.write_all(appendix.as_bytes())?;
    }

    // 13. Archive to layer/sessions/{ID}.md (mark status: archived)
    let archive_path = project_root
        .join(SESSIONS_DIR)
        .join(format!("{}.md", session_id));
    fs::create_dir_all(project_root.join(SESSIONS_DIR))?;
    let session_content = fs::read_to_string(&session_path)?;
    let archived_content = if session_content.starts_with("---") {
        // YAML frontmatter — update status for archive
        session_content.replacen("status: active", "status: archived", 1)
    } else {
        // Legacy format — archive as-is
        session_content
    };
    fs::write(&archive_path, archived_content)?;

    // 14. Update last-session.md pointer
    let last_session_content = format!(
        "# Last Session: {title}\n\n\
         See: {sessions_dir}/{id}.md\n\
         Tags: {start_tag}..{end_tag}\n\
         Classification: {classification}\n\n\
         Quick start: /session-start \"continue from {title}\"\n",
        title = session_title,
        sessions_dir = SESSIONS_DIR,
        id = session_id,
        start_tag = session_tag,
        end_tag = end_tag,
        classification = classification,
    );
    fs::write(&last_session_path, &last_session_content)?;

    // 15. Write session.ended event to eventlog
    let now = Local::now();
    let db_path = project_root.join(patina::eventlog::PATINA_DB);
    let conn = patina::eventlog::initialize(&db_path)?;
    let timestamp = now.to_rfc3339();
    let data = json!({
        "session_id": session_id,
        "title": session_title,
        "adapter": adapter,
        "classification": classification,
        "files_changed": files_changed,
        "commits_made": commits_made,
        "patterns_modified": patterns_modified,
        "beliefs_captured": beliefs_captured,
        "end_tag": end_tag,
        "session_tag": session_tag,
    });
    patina::eventlog::insert_event(
        &conn,
        "session.ended",
        &timestamp,
        &session_id,
        Some(&format!("{}/{}.md", SESSIONS_DIR, session_id)),
        &data.to_string(),
    )?;

    // 16. Clean up active session file and .last-update
    fs::remove_file(&session_path)?;
    if last_update_path.exists() {
        fs::remove_file(&last_update_path)?;
    }

    // 17. Output archive confirmation
    println!();
    println!("✓ Session archived:");
    println!("  - {}/{}.md", SESSIONS_DIR, session_id);
    println!("  - Updated last-session.md");
    println!();
    println!("✓ Session preserved via tags: {}..{}", session_tag, end_tag);
    println!("  View work: git log {}..{}", session_tag, end_tag);
    println!();
    println!("Session Memory:");
    println!("  Your work is preserved in Git history and can be found by:");
    println!("  - git log --grep=\"{}\"", session_title);
    println!("  - git tag | grep session");

    Ok(())
}

/// Read session ID from active session markdown.
///
/// Looks for `**ID**: <value>` in the frontmatter area.
fn read_session_id(session_path: &Path) -> Result<String> {
    read_session_field(session_path, "**ID**: ")
}

/// Read a field value from active session markdown.
///
/// Tries YAML frontmatter first (new format), falls back to line-matching
/// (legacy `**Field**: value` format) for backward compatibility with
/// 538 existing session files.
fn read_session_field(session_path: &Path, prefix: &str) -> Result<String> {
    let contents = fs::read_to_string(session_path)?;

    // Try YAML frontmatter first
    if let Some(fm) = parse_session_frontmatter(&contents) {
        let value = match prefix {
            "**ID**: " => Some(fm.id),
            "# Session: " => Some(fm.title),
            "**Started**: " => Some(fm.created.clone()),
            "**Start Timestamp**: " => Some(fm.start_timestamp.to_string()),
            "**LLM**: " => Some(fm.llm),
            "**Git Branch**: " => Some(fm.git.branch),
            "**Session Tag**: " => Some(fm.git.start_tag),
            "**Starting Commit**: " => Some(fm.git.starting_commit),
            _ => None,
        };
        if let Some(v) = value {
            return Ok(v);
        }
    }

    // Fall back to line-matching (legacy format)
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

/// Parse YAML frontmatter from a session markdown file.
///
/// Returns `None` if the file doesn't start with `---` or YAML parsing fails.
/// Used by `read_session_field` for the new frontmatter format.
fn parse_session_frontmatter(content: &str) -> Option<SessionFrontmatter> {
    let rest = content.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let yaml_str = &rest[..end];
    serde_yaml::from_str(yaml_str).ok()
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
            println!("Previous session \"{}\": no beliefs captured", prev_title);
        }
    }
}

/// Classify work type based on session metrics.
///
/// Matches shell script classification logic exactly:
/// - 0 commits → exploration
/// - patterns modified → pattern-work
/// - >10 files → major-feature
/// - <3 commits → experiment
/// - otherwise → feature
fn classify_work(commits: usize, files: usize, patterns: usize) -> &'static str {
    if commits == 0 {
        "exploration"
    } else if patterns > 0 {
        "pattern-work"
    } else if files > 10 {
        "major-feature"
    } else if commits < 3 {
        "experiment"
    } else {
        "feature"
    }
}

/// Human-readable classification label with emoji (console output only).
fn classification_label(classification: &str) -> &'static str {
    match classification {
        "exploration" => "🧪 EXPLORATION (no commits)",
        "pattern-work" => "📚 PATTERN-WORK (modified patterns)",
        "major-feature" => "🚀 MAJOR-FEATURE (many files)",
        "experiment" => "🔬 EXPERIMENT (few commits)",
        "feature" => "✨ FEATURE (normal work)",
        _ => "❓ UNKNOWN",
    }
}

/// Count beliefs captured during this session.
///
/// Scans `layer/surface/epistemic/beliefs/*.md` and checks which files
/// appear in the list of changed files since session start.
fn count_beliefs_captured(project_root: &Path, changed_files: &[String]) -> (usize, Vec<String>) {
    let beliefs_dir = project_root.join("layer/surface/epistemic/beliefs");
    if !beliefs_dir.is_dir() {
        return (0, vec![]);
    }

    let entries = match fs::read_dir(&beliefs_dir) {
        Ok(e) => e,
        Err(_) => return (0, vec![]),
    };

    let mut count = 0;
    let mut summaries = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if !name_str.ends_with(".md") || name_str.as_ref() == "_index.md" {
            continue;
        }

        // Check if this belief file appears in changed files
        let relative_path = format!("layer/surface/epistemic/beliefs/{}", name_str);
        if changed_files.iter().any(|f| f == &relative_path) {
            count += 1;

            // Extract belief ID and statement from file
            let belief_id = name_str.trim_end_matches(".md");
            let path = entry.path();
            let statement = fs::read_to_string(&path).ok().and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("statement:"))
                    .map(|l| l.trim_start_matches("statement:").trim().to_string())
            });

            if let Some(stmt) = statement {
                summaries.push(format!("  - **{}**: {}", belief_id, stmt));
            }
        }
    }

    (count, summaries)
}

/// Extract user prompts from Claude Code history.jsonl.
///
/// Reads `~/.claude/history.jsonl`, filters entries by start timestamp and
/// project path. Returns display text of matching prompts.
fn extract_user_prompts(project_root: &Path, session_path: &Path) -> Vec<String> {
    // Read start timestamp from session file
    let start_ts: i64 = match read_session_field(session_path, "**Start Timestamp**: ") {
        Ok(ts) => match ts.parse() {
            Ok(v) => v,
            Err(_) => return vec![],
        },
        Err(_) => return vec![],
    };

    // Locate history file (currently Claude-specific)
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return vec![],
    };
    let history_path = PathBuf::from(&home).join(".claude/history.jsonl");
    if !history_path.exists() {
        return vec![];
    }

    // Canonicalize project path for comparison
    let project_path = match project_root.canonicalize() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return vec![],
    };

    // Stream through JSONL file
    let file = match fs::File::open(&history_path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let reader = std::io::BufReader::new(file);
    let mut prompts = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.is_empty() {
            continue;
        }
        let entry: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let ts = entry.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
        let project = entry.get("project").and_then(|v| v.as_str()).unwrap_or("");
        let display = entry.get("display").and_then(|v| v.as_str()).unwrap_or("");

        if ts >= start_ts && project == project_path && !display.is_empty() {
            prompts.push(display.to_string());
        }
    }

    prompts
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
