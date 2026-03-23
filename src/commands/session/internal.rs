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

use crate::commands::spec;
use patina::git;
use patina::session::{
    self, ArchiveSessionRequest, BeginSessionRequest, InterfaceKind, SessionParticipant,
};
use patina::spec::SpecStatus;

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
    runtime_id: String,
    title: String,
    status: String,
    llm: String,
    interface: String,
    created: String,
    updated: String,
    start_timestamp: i64,
    persona: Option<String>,
    #[serde(default)]
    participants: Vec<SessionParticipantFrontmatter>,
    #[serde(default)]
    interfaces: Vec<String>,
    parent_session: Option<String>,
    handoff_from: Option<String>,
    #[serde(default)]
    handoff_to: Vec<String>,
    git: SessionGit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionParticipantFrontmatter {
    id: String,
    role: String,
    interface: String,
    adapter: Option<String>,
    display_name: Option<String>,
    joined_at: String,
    left_at: Option<String>,
}

/// Git context embedded in session YAML frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionGit {
    project_uid: Option<String>,
    branch: String,
    starting_commit: String,
    start_tag: String,
    end_tag: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SessionSurfaceMode {
    NativeInterface { interface_kind: InterfaceKind },
}

impl SessionSurfaceMode {
    fn interface_kind(self) -> InterfaceKind {
        match self {
            Self::NativeInterface { interface_kind } => interface_kind,
        }
    }

    fn participant_role(self) -> &'static str {
        match self {
            Self::NativeInterface { .. } => "interface",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SessionStartRequest {
    pub title: String,
    pub adapter: String,
    pub mode: SessionSurfaceMode,
}

impl SessionStartRequest {
    pub(crate) fn native(title: &str, adapter: &str) -> Self {
        Self {
            title: title.to_string(),
            adapter: adapter.to_string(),
            mode: SessionSurfaceMode::NativeInterface {
                interface_kind: InterfaceKind::from_adapter_name(adapter),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SessionStartResult {
    pub command: &'static str,
    pub session_id: String,
    pub runtime_id: String,
    pub title: String,
    pub adapter: String,
    pub interface: String,
    pub branch: String,
    pub starting_commit: String,
    pub start_tag: String,
    pub artifact_path: String,
    pub active_session_path: Option<String>,
    pub last_session_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SessionUpdateResult {
    pub command: &'static str,
    pub session_id: String,
    pub runtime_id: String,
    pub artifact_path: String,
    pub branch: String,
    pub start_tag: String,
    pub since: String,
    pub updated_at: String,
    pub commits_this_session: usize,
    pub recent_commits: Vec<String>,
    pub session_changed_files: Vec<String>,
    pub modified_files: usize,
    pub staged_files: usize,
    pub untracked_files: usize,
    pub lines_changed: usize,
    pub last_commit_time: String,
    pub last_commit_message: String,
    pub working_tree_clean: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SessionEndResult {
    pub command: &'static str,
    pub session_id: String,
    pub runtime_id: String,
    pub title: String,
    pub adapter: String,
    pub branch: String,
    pub start_tag: String,
    pub end_tag: String,
    pub artifact_path: String,
    pub last_session_path: String,
    pub classification: String,
    pub files_changed: usize,
    pub changed_files: Vec<String>,
    pub commits_made: usize,
    pub patterns_modified: usize,
    pub beliefs_captured: usize,
    pub beliefs_summary: Vec<String>,
    pub uncommitted_files: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SessionListResult {
    pub active: Vec<SessionListEntry>,
    pub stale: Vec<SessionListEntry>,
    pub recent: Vec<SessionListEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SessionListEntry {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    pub title: String,
    pub status: String,
    pub created: String,
    pub age: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
}

pub fn start_session(project_root: &Path, title: &str, adapter: Option<&str>) -> Result<()> {
    let adapter = resolve_adapter(adapter, project_root)?;
    let session_path = project_root.join(ACTIVE_SESSION_PATH);
    let last_update_path = project_root.join(LAST_UPDATE_PATH);
    let dev_branch = dev_branch_name(project_root);

    // 1. Handle incomplete previous session
    if session_path.exists() {
        // Check for stale session (>24h old)
        let content = fs::read_to_string(&session_path).unwrap_or_default();
        if let Some(summary) = parse_session_summary(&content) {
            let age_hours = session_age_hours(&summary.created);
            if age_hours >= 24 {
                let days = age_hours / 24;
                println!(
                    "Warning: Previous session is {}d old — archiving stale session",
                    days
                );
                println!("  {} ({})", summary.title, summary.id);
            } else {
                println!("Found incomplete session, cleaning up...");
            }
        } else {
            println!("Found incomplete session, cleaning up...");
        }

        let line_count = content.lines().count();
        if line_count > 10 {
            // Archive non-trivial session (mark as archived in YAML)
            if let Ok(old_id) = read_session_id(&session_path) {
                let archive_path = project_root
                    .join(SESSIONS_DIR)
                    .join(format!("{}.md", old_id));
                fs::create_dir_all(project_root.join(SESSIONS_DIR))?;
                let archived = content.replacen("status: active", "status: archived", 1);
                fs::write(&archive_path, archived)?;
                println!("  Archived to {}/{}.md", SESSIONS_DIR, old_id);
            }
        } else {
            println!("  Removed empty session file");
        }
        fs::remove_file(&session_path)?;
    }

    // 2. Git context (branch re-read after potential switch below)
    let branch = git::current_branch().unwrap_or_else(|_| "none".to_string());

    // 3. Check for uncommitted changes
    if !git::is_clean().unwrap_or(true) {
        println!("Warning: Uncommitted changes exist");
        println!("  Consider: git stash or git commit -am 'WIP: saving work'");
        println!();
    }

    // 4. Smart branch handling (dev branch from config, fallback "work")
    if git::is_git_repo().unwrap_or(false) {
        let is_dev_related = branch == dev_branch || is_ancestor_of_head(&dev_branch);

        if !is_dev_related {
            if branch == "main" || branch == "master" {
                // Switch to dev branch
                if git::branch_exists(&dev_branch).unwrap_or(false) {
                    git::checkout(&dev_branch)?;
                    println!("Switched to {} branch from {}", dev_branch, branch);
                } else {
                    git::checkout_new_branch(&dev_branch, &branch)?;
                    println!(
                        "Created and switched to {} branch from {}",
                        dev_branch, branch
                    );
                }
            } else {
                println!("On unrelated branch: {}", branch);
                println!(
                    "  Consider: git checkout {} or git checkout -b {}/{}",
                    dev_branch, dev_branch, branch
                );
            }
        } else if branch != dev_branch {
            println!("Staying on work sub-branch: {}", branch);
        }
    }

    // 5. Create Mother-backed live session and durable artifact
    let now = Local::now();
    let start = session::begin_session(
        project_root,
        BeginSessionRequest {
            title: title.to_string(),
            adapter_name: adapter.clone(),
            interface_kind: InterfaceKind::LegacyCli,
            persona_uid: None,
            parent_runtime_id: None,
            handoff_from_runtime_id: None,
            participant: Some(SessionParticipant {
                participant_id: format!(
                    "{}-{}",
                    std::env::var("USER").unwrap_or_else(|_| "operator".to_string()),
                    std::process::id()
                ),
                role: "operator".to_string(),
                interface_kind: InterfaceKind::LegacyCli,
                adapter_name: Some(adapter.clone()),
                display_name: std::env::var("USER").ok(),
            }),
        },
    )?;
    let session_id = start.handle.file_id.clone();
    let session_tag = start.handle.start_tag.clone();
    let branch = start.handle.branch.clone();
    let starting_commit = start.handle.starting_commit.clone();
    let time_str = now.format("%H:%M").to_string();
    let scaffold = start.document;

    fs::create_dir_all(session_path.parent().unwrap())?;
    fs::write(&session_path, &scaffold)?;

    // 8. Write .last-update marker
    fs::write(&last_update_path, &time_str)?;

    // 9. Write session.started event to events.db (runtime events)
    let conn = patina::eventlog::open_events_db_at(project_root)?;
    let timestamp = now.to_rfc3339();
    let data = json!({
        "session_id": session_id,
        "runtime_id": start.handle.runtime_id,
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

    // Git coaching (concise — skill reads stdout directly)
    if git::is_git_repo().unwrap_or(false) {
        println!();
        println!("Session Strategy:");
        if branch == dev_branch {
            println!(
                "- You're on the '{}' branch - all sessions happen here",
                dev_branch
            );
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

    // Spec landscape (Phase 5: session integration)
    show_spec_landscape();

    // Previous session reference (printed so skill can use stdout directly)
    let last_session_path = project_root.join(LAST_SESSION_PATH);
    if last_session_path.exists() {
        if let Ok(content) = fs::read_to_string(&last_session_path) {
            println!();
            for line in content.lines() {
                println!("{}", line);
            }
        }
        println!();
        println!(
            "Please read {} and fill in the Previous Session Context section above.",
            LAST_SESSION_PATH
        );
    } else {
        println!();
        println!("No previous session found. Starting fresh.");
    }
    println!(
        "Then ask: 'Would you like me to create todos for \"{}\"?'",
        title
    );

    Ok(())
}

pub(crate) fn start_session_value(
    project_root: &Path,
    request: SessionStartRequest,
) -> Result<SessionStartResult> {
    let adapter = resolve_adapter(
        (!request.adapter.is_empty()).then_some(request.adapter.as_str()),
        project_root,
    )?;
    let dev_branch = dev_branch_name(project_root);
    let mode = request.mode;

    if matches!(
        mode,
        SessionSurfaceMode::NativeInterface {
            interface_kind: InterfaceKind::Unknown
        }
    ) {
        bail!(
            "Native session mode requires a native interface adapter, got '{}'",
            adapter
        );
    }

    let branch = git::current_branch().unwrap_or_else(|_| "none".to_string());
    if git::is_git_repo().unwrap_or(false) {
        let is_dev_related = branch == dev_branch || is_ancestor_of_head(&dev_branch);
        if !is_dev_related && (branch == "main" || branch == "master") {
            if git::branch_exists(&dev_branch).unwrap_or(false) {
                git::checkout(&dev_branch)?;
            } else {
                git::checkout_new_branch(&dev_branch, &branch)?;
            }
        }
    }

    let now = Local::now();
    let start = session::begin_session(
        project_root,
        BeginSessionRequest {
            title: request.title.clone(),
            adapter_name: adapter.clone(),
            interface_kind: mode.interface_kind(),
            persona_uid: None,
            parent_runtime_id: None,
            handoff_from_runtime_id: None,
            participant: Some(SessionParticipant {
                participant_id: format!(
                    "{}-{}",
                    std::env::var("USER").unwrap_or_else(|_| "operator".to_string()),
                    std::process::id()
                ),
                role: mode.participant_role().to_string(),
                interface_kind: mode.interface_kind(),
                adapter_name: Some(adapter.clone()),
                display_name: std::env::var("USER").ok().or_else(|| {
                    (mode.interface_kind() != InterfaceKind::LegacyCli)
                        .then(|| Some(adapter.clone()))
                        .flatten()
                }),
            }),
        },
    )?;

    let conn = patina::eventlog::open_events_db_at(project_root)?;
    let timestamp = now.to_rfc3339();
    let data = json!({
        "session_id": start.handle.file_id,
        "runtime_id": start.handle.runtime_id,
        "title": request.title,
        "adapter": adapter,
        "branch": start.handle.branch,
        "starting_commit": start.handle.starting_commit,
        "tag": start.handle.start_tag,
    });
    let source_path = start.handle.artifact_path.display().to_string();
    patina::eventlog::insert_event(
        &conn,
        "session.started",
        &timestamp,
        &start.handle.file_id,
        Some(&source_path),
        &data.to_string(),
    )?;

    Ok(SessionStartResult {
        command: "start",
        session_id: start.handle.file_id.clone(),
        runtime_id: start.handle.runtime_id.clone(),
        title: start.handle.title.clone(),
        adapter: start.handle.adapter_name.clone(),
        interface: start.handle.interface_kind.as_str().to_string(),
        branch: start.handle.branch.clone(),
        starting_commit: start.handle.starting_commit.clone(),
        start_tag: start.handle.start_tag.clone(),
        artifact_path: start.handle.artifact_path.display().to_string(),
        active_session_path: None,
        last_session_path: project_root.join(LAST_SESSION_PATH).display().to_string(),
    })
}

pub(crate) fn update_session_json(project_root: &Path) -> Result<SessionUpdateResult> {
    let session_path = project_root.join(ACTIVE_SESSION_PATH);
    let last_update_path = project_root.join(LAST_UPDATE_PATH);
    if !session_path.exists() {
        bail!(
            "No active session found at {}\nStart one with: patina session start \"<title>\"",
            ACTIVE_SESSION_PATH
        );
    }

    let last_update = fs::read_to_string(&last_update_path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "session start".to_string());
    update_session_document_value(
        project_root,
        &session_path,
        ACTIVE_SESSION_PATH,
        last_update,
        Some(&last_update_path),
    )
}

pub(crate) fn update_live_session_value(
    project_root: &Path,
    handle: &session::LiveSessionHandle,
) -> Result<SessionUpdateResult> {
    update_session_document_value(
        project_root,
        &handle.artifact_path,
        &handle.artifact_path.display().to_string(),
        last_update_from_document(&handle.artifact_path),
        None,
    )
}

pub(crate) fn end_session_json(project_root: &Path) -> Result<SessionEndResult> {
    let session_path = project_root.join(ACTIVE_SESSION_PATH);
    if !session_path.exists() {
        bail!(
            "No active session found at {}\nStart one with: patina session start \"<title>\"",
            ACTIVE_SESSION_PATH
        );
    }

    end_session_document_value(project_root, &session_path, ACTIVE_SESSION_PATH, None, true)
}

pub(crate) fn end_live_session_value(
    project_root: &Path,
    handle: &session::LiveSessionHandle,
    note: Option<&str>,
) -> Result<SessionEndResult> {
    end_session_document_value(
        project_root,
        &handle.artifact_path,
        &handle.artifact_path.display().to_string(),
        note,
        false,
    )
}

pub(crate) fn resolve_live_session(
    project_root: &Path,
    selector: Option<&str>,
    adapter_filter: Option<&str>,
) -> Result<session::LiveSessionHandle> {
    if let Some(selector) = selector {
        return load_session(project_root, selector)?
            .ok_or_else(|| anyhow::anyhow!("No active session found for selector '{}'", selector));
    }

    if let Some(adapter) = adapter_filter
        .map(ToOwned::to_owned)
        .or_else(|| std::env::var("PATINA_AI_INTERFACE").ok())
        .filter(|value| !value.is_empty())
    {
        if let Some(handle) = session::load_current_interface_session(project_root, &adapter)? {
            return Ok(handle);
        }
    }

    if let Some(runtime_id) = std::env::var("PATINA_SESSION_RUNTIME_ID")
        .ok()
        .filter(|value| !value.is_empty())
    {
        if let Some(handle) = session::load_session(project_root, &runtime_id)? {
            return Ok(handle);
        }
    }

    if let Some(file_id) = std::env::var("PATINA_SESSION_ID")
        .ok()
        .filter(|value| !value.is_empty())
    {
        if let Some(handle) = session::load_session_by_file_id(project_root, &file_id)? {
            return Ok(handle);
        }
    }

    let mut sessions = session::list_active_sessions(project_root)?;
    if let Some(adapter) = adapter_filter
        .map(ToOwned::to_owned)
        .or_else(|| std::env::var("PATINA_AI_INTERFACE").ok())
        .filter(|value| !value.is_empty())
    {
        sessions.retain(|handle| handle.adapter_name == adapter);
    }

    match sessions.len() {
        1 => Ok(sessions.remove(0)),
        0 => bail!("No active session found"),
        _ => {
            let choices = sessions
                .iter()
                .map(|handle| format!("{} ({})", handle.file_id, handle.title))
                .collect::<Vec<_>>()
                .join(", ");
            bail!(
                "Multiple active sessions match. Retry with session=<id>. Choices: {}",
                choices
            )
        }
    }
}

fn load_session(project_root: &Path, selector: &str) -> Result<Option<session::LiveSessionHandle>> {
    if let Some(handle) = session::load_session(project_root, selector)? {
        return Ok(Some(handle));
    }
    session::load_session_by_file_id(project_root, selector)
}

fn update_session_document_value(
    project_root: &Path,
    session_path: &Path,
    source_path: &str,
    last_update: String,
    last_update_path: Option<&Path>,
) -> Result<SessionUpdateResult> {
    let session_id = read_session_id(session_path)?;
    let runtime_id = read_session_field(session_path, "**Runtime ID**: ")?;
    let starting_commit = read_session_field(session_path, "**Starting Commit**: ")?;
    let session_tag = read_session_field(session_path, "**Session Tag**: ").unwrap_or_default();

    let branch = git::current_branch().unwrap_or_else(|_| "detached".to_string());
    let commits_this_session = git::commits_since_count(&starting_commit).unwrap_or(0);
    let last_commit_time = git::last_commit_relative_time().unwrap_or_else(|_| "never".to_string());
    let last_commit_msg =
        git::last_commit_message().unwrap_or_else(|_| "no commits yet".to_string());

    let porcelain = git::status_porcelain().unwrap_or_default();
    let modified = porcelain.lines().filter(|l| l.starts_with(" M")).count();
    let staged = porcelain.lines().filter(|l| l.starts_with('M')).count();
    let untracked = porcelain.lines().filter(|l| l.starts_with("??")).count();
    let diff_summary = git::diff_stat_summary().unwrap_or_default();
    let lines_changed = parse_insertions(&diff_summary);
    let changed_files = if !session_tag.is_empty() {
        git::files_changed_since(&session_tag).unwrap_or_default()
    } else {
        Vec::new()
    };
    let commit_list = git::log_oneline(commits_this_session.min(20)).unwrap_or_default();
    let recent_commits = commit_list
        .lines()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    let now = Local::now();
    let time_str = now.format("%H:%M").to_string();
    let mut update_section = format!(
        "\n### {} - Update (covering since {})\n",
        time_str, last_update
    );
    update_section.push_str("\n**Git Activity:**\n");
    update_section.push_str(&format!(
        "- Commits this session: {}{}\n",
        commits_this_session,
        if !commit_list.is_empty() {
            format!(
                " ({})",
                commit_list
                    .lines()
                    .map(|l| {
                        let sha = l.split_whitespace().next().unwrap_or("");
                        format!("[[commit-{}]]", sha)
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            String::new()
        }
    ));
    update_section.push_str(&format!(
        "- Files changed: {}\n",
        if !changed_files.is_empty() {
            format!(
                "{} (`{}`)",
                changed_files.len(),
                changed_files
                    .iter()
                    .take(4)
                    .map(|f| f.rsplit('/').next().unwrap_or(f).to_string())
                    .collect::<Vec<_>>()
                    .join("`, `")
            )
        } else {
            (modified + staged + untracked).to_string()
        }
    ));
    update_section.push_str(&format!("- Last commit: {}\n", last_commit_time));
    update_section.push('\n');

    let mut file = OpenOptions::new().append(true).open(session_path)?;
    file.write_all(update_section.as_bytes())?;
    let current_markdown = fs::read_to_string(session_path)?;
    let synced = session::sync_session_document(project_root, &runtime_id, &current_markdown)?;

    if let Some(path) = last_update_path {
        fs::write(path, &time_str)?;
    }

    let conn = patina::eventlog::open_events_db_at(project_root)?;
    let timestamp = now.to_rfc3339();
    let data = json!({
        "session_id": session_id,
        "runtime_id": runtime_id,
        "commits_this_session": commits_this_session,
        "files_changed": modified + staged + untracked,
        "last_commit_time": last_commit_time,
        "lines_changed": lines_changed,
        "branch": branch,
    });
    patina::eventlog::insert_event(
        &conn,
        "session.update",
        &timestamp,
        &session_id,
        Some(source_path),
        &data.to_string(),
    )?;

    Ok(SessionUpdateResult {
        command: "update",
        session_id,
        runtime_id,
        artifact_path: synced.artifact_path.display().to_string(),
        branch,
        start_tag: session_tag,
        since: last_update,
        updated_at: timestamp,
        commits_this_session,
        recent_commits,
        session_changed_files: changed_files,
        modified_files: modified,
        staged_files: staged,
        untracked_files: untracked,
        lines_changed,
        last_commit_time,
        last_commit_message: last_commit_msg,
        working_tree_clean: modified + staged + untracked == 0,
    })
}

fn end_session_document_value(
    project_root: &Path,
    session_path: &Path,
    source_path: &str,
    note: Option<&str>,
    cleanup_compatibility: bool,
) -> Result<SessionEndResult> {
    let last_update_path = project_root.join(LAST_UPDATE_PATH);
    let last_session_path = project_root.join(LAST_SESSION_PATH);

    let session_id = read_session_id(session_path)?;
    let runtime_id = read_session_field(session_path, "**Runtime ID**: ")?;
    let session_title = read_session_field(session_path, "# Session: ")?;
    let session_tag = read_session_field(session_path, "**Session Tag**: ")?;
    let starting_commit = read_session_field(session_path, "**Starting Commit**: ")?;
    let adapter = read_session_field(session_path, "**LLM**: ")?;

    {
        let content = fs::read_to_string(session_path)?;
        let updated =
            session::rewrite_document_status(&content, "completed", &Utc::now().to_rfc3339(), None)
                .unwrap_or(content);
        fs::write(session_path, &updated)?;
    }

    let end_tag = format!("session-{}-{}-end", session_id, adapter);
    if git::is_git_repo().unwrap_or(false) {
        let _ = git::create_tag(&end_tag, &format!("Session end: {}", session_title));
    }

    let changed_files = git::files_changed_since(&session_tag).unwrap_or_default();
    let files_changed = changed_files.len();
    let commits_made = git::commits_since_count(&starting_commit).unwrap_or(0);
    let patterns_modified = changed_files
        .iter()
        .filter(|f| {
            f.starts_with("layer/core/")
                || f.starts_with("layer/surface/")
                || f.starts_with("layer/topics/")
        })
        .count();
    let classification = classify_work(commits_made, files_changed, patterns_modified);
    let uncommitted = git::status_count().unwrap_or(0);
    let (beliefs_captured, beliefs_summary) = count_beliefs_captured(project_root, &changed_files);

    let mut session_content = fs::read_to_string(session_path)?;
    if let Some(note) = note {
        session_content = append_outcome_note(&session_content, note);
    }

    let mut appendix = String::new();
    appendix.push_str(&format!("\n## Beliefs Captured: {}\n", beliefs_captured));
    if beliefs_captured > 0 {
        for line in &beliefs_summary {
            appendix.push_str(&format!("{line}\n"));
        }
    } else {
        appendix.push_str("_No beliefs captured this session_\n");
    }

    appendix.push_str("\n## Session Classification\n");
    appendix.push_str(&format!("- Work Type: {classification}\n"));
    appendix.push_str(&format!("- Files Changed: {files_changed}\n"));
    appendix.push_str(&format!("- Commits: {commits_made}\n"));
    appendix.push_str(&format!("- Patterns Modified: {patterns_modified}\n"));
    appendix.push_str(&format!("- Beliefs Captured: {beliefs_captured}\n"));
    appendix.push_str(&format!("- Session Tags: {session_tag}..{end_tag}\n"));

    let prompts = extract_user_prompts(project_root, session_path);
    if !prompts.is_empty() {
        appendix.push_str(&format!("\n## User Prompts ({})\n\n", prompts.len()));
        for (i, prompt) in prompts.iter().enumerate() {
            let display = truncate(prompt, 97).replace('`', "\\`");
            appendix.push_str(&format!("{}. `{}`\n", i + 1, display));
        }
    }

    session_content.push_str(&appendix);
    fs::write(session_path, &session_content)?;

    let archived = session::archive_session(
        project_root,
        ArchiveSessionRequest {
            runtime_id: runtime_id.clone(),
            markdown: session_content,
            end_tag: Some(end_tag.clone()),
        },
    )?;

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

    let now = Local::now();
    let timestamp = now.to_rfc3339();
    let conn = patina::eventlog::open_events_db_at(project_root)?;
    let data = json!({
        "session_id": session_id,
        "runtime_id": runtime_id,
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
        Some(source_path),
        &data.to_string(),
    )?;
    crate::commands::events::export_best_effort();

    if cleanup_compatibility {
        fs::remove_file(session_path)?;
        if last_update_path.exists() {
            fs::remove_file(last_update_path)?;
        }
    }

    Ok(SessionEndResult {
        command: "end",
        session_id,
        runtime_id,
        title: session_title,
        adapter,
        branch: git::current_branch().unwrap_or_else(|_| "none".to_string()),
        start_tag: session_tag,
        end_tag,
        artifact_path: archived.artifact_path.display().to_string(),
        last_session_path: last_session_path.display().to_string(),
        classification: classification.to_string(),
        files_changed,
        changed_files,
        commits_made,
        patterns_modified,
        beliefs_captured,
        beliefs_summary,
        uncommitted_files: uncommitted,
    })
}

fn last_update_from_document(session_path: &Path) -> String {
    parse_session_frontmatter(&fs::read_to_string(session_path).unwrap_or_default())
        .map(|fm| {
            chrono::DateTime::parse_from_rfc3339(&fm.updated)
                .map(|dt| dt.with_timezone(&Local).format("%H:%M").to_string())
                .unwrap_or_else(|_| "session start".to_string())
        })
        .unwrap_or_else(|| "session start".to_string())
}

fn append_outcome_note(markdown: &str, note: &str) -> String {
    if let Some((head, tail)) = markdown.split_once("## Outcome\n") {
        return format!("{head}## Outcome\n{note}\n\n{tail}");
    }
    format!("{markdown}\n\n## Outcome\n{note}\n")
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

    // 6. Get commit list and files changed for structured output
    let session_tag = read_session_field(&session_path, "**Session Tag**: ").unwrap_or_default();
    let changed_files = if !session_tag.is_empty() {
        git::files_changed_since(&session_tag).unwrap_or_default()
    } else {
        vec![]
    };
    let commit_list = git::log_oneline(commits_this_session.min(20)).unwrap_or_default();

    // Print structured commit/file info for skill consumption
    if !commit_list.is_empty() {
        println!();
        println!("Commits this session:");
        for line in commit_list.lines() {
            println!("  {}", line);
        }
    }

    if !changed_files.is_empty() {
        println!();
        println!("Files changed this session ({}):", changed_files.len());
        for f in changed_files.iter().take(20) {
            println!("  {}", f);
        }
        if changed_files.len() > 20 {
            println!("  ... and {} more", changed_files.len() - 20);
        }
    }

    // Spec status changes (Phase 5: session integration)
    show_spec_status_in_update(&changed_files);

    // 7. Append update section to active session markdown
    let now = Local::now();
    let time_str = now.format("%H:%M").to_string();
    let mut update_section = format!(
        "\n### {} - Update (covering since {})\n",
        time_str, last_update
    );
    update_section.push_str("\n**Git Activity:**\n");
    update_section.push_str(&format!(
        "- Commits this session: {}{}\n",
        commits_this_session,
        if !commit_list.is_empty() {
            format!(
                " ({})",
                commit_list
                    .lines()
                    .map(|l| {
                        let sha = l.split_whitespace().next().unwrap_or("");
                        format!("[[commit-{}]]", sha)
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            String::new()
        }
    ));
    update_section.push_str(&format!(
        "- Files changed: {}\n",
        if !changed_files.is_empty() {
            format!(
                "{} (`{}`)",
                changed_files.len(),
                changed_files
                    .iter()
                    .take(4)
                    .map(|f| f.rsplit('/').next().unwrap_or(f).to_string())
                    .collect::<Vec<_>>()
                    .join("`, `")
            )
        } else {
            total_changes.to_string()
        }
    ));
    update_section.push_str(&format!("- Last commit: {}\n", last_commit_time));
    update_section.push('\n');

    let mut file = OpenOptions::new().append(true).open(&session_path)?;
    file.write_all(update_section.as_bytes())?;
    let runtime_id = read_session_field(&session_path, "**Runtime ID**: ")?;
    let current_markdown = fs::read_to_string(&session_path)?;
    let _ = session::sync_session_document(project_root, &runtime_id, &current_markdown);

    // 7. Update last update timestamp
    fs::write(&last_update_path, &time_str)?;

    // 8. Write session.update event to events.db (runtime events)
    let conn = patina::eventlog::open_events_db_at(project_root)?;
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

    note_session_document(project_root, &session_path, ACTIVE_SESSION_PATH, content)
}

pub(crate) fn note_live_session(
    project_root: &Path,
    handle: &session::LiveSessionHandle,
    content: &str,
) -> Result<()> {
    note_session_document(
        project_root,
        &handle.artifact_path,
        &handle.artifact_path.display().to_string(),
        content,
    )
}

fn note_session_document(
    project_root: &Path,
    session_path: &Path,
    source_path: &str,
    content: &str,
) -> Result<()> {
    // 2. Get git context
    let branch = git::current_branch().unwrap_or_else(|_| "detached".to_string());
    let sha = git::short_sha().unwrap_or_else(|_| "no-commits".to_string());
    let git_context = format!("[{}@{}]", branch, sha);

    // 3. Append timestamped note to active session markdown
    let now = Local::now();
    let time_str = now.format("%H:%M").to_string();
    let note_section = format!("\n### {} - Note {}\n{}\n", time_str, git_context, content);

    let mut file = OpenOptions::new().append(true).open(session_path)?;
    file.write_all(note_section.as_bytes())?;
    let runtime_id = read_session_field(session_path, "**Runtime ID**: ")?;
    let current_markdown = fs::read_to_string(session_path)?;
    let _ = session::sync_session_document(project_root, &runtime_id, &current_markdown);

    // 4. Write session.observation event to eventlog
    //    Read session ID from the active session file for the source_id
    let session_id = read_session_id(session_path)?;
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
        Some(source_path),
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
    let runtime_id = read_session_field(&session_path, "**Runtime ID**: ")?;
    let session_title = read_session_field(&session_path, "# Session: ")?;
    let session_tag = read_session_field(&session_path, "**Session Tag**: ")?;
    let starting_commit = read_session_field(&session_path, "**Starting Commit**: ")?;
    let adapter = read_session_field(&session_path, "**LLM**: ")?;

    // 3. Atomic status flip — mark completed FIRST, before any other mutations.
    //    If later steps (metrics, archive) fail, the session is at least marked done.
    {
        let content = fs::read_to_string(&session_path)?;
        let updated =
            session::rewrite_document_status(&content, "completed", &Utc::now().to_rfc3339(), None)
                .unwrap_or(content);
        fs::write(&session_path, &updated)?;
    }

    // 4. Create end session tag
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
        .filter(|f| {
            f.starts_with("layer/core/")
                || f.starts_with("layer/surface/")
                || f.starts_with("layer/topics/")
        })
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

    // Spec integration (Phase 5: next suggestion + unblock detection)
    show_spec_end_summary(&changed_files);

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
    let session_content = fs::read_to_string(&session_path)?;
    session::archive_session(
        project_root,
        ArchiveSessionRequest {
            runtime_id: runtime_id.clone(),
            markdown: session_content,
            end_tag: Some(end_tag.clone()),
        },
    )?;

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

    // 15. Write session.ended event to events.db (runtime events)
    let now = Local::now();
    let conn = patina::eventlog::open_events_db_at(project_root)?;
    let timestamp = now.to_rfc3339();
    let data = json!({
        "session_id": session_id,
        "runtime_id": runtime_id,
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

    // 15b. Export new events to JSONL replica (best-effort, don't block archival)
    crate::commands::events::export_best_effort();

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
            "**Runtime ID**: " => Some(fm.runtime_id),
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

/// Truncate a string to max_len bytes, appending "..." if truncated.
/// Rounds down to the nearest char boundary to avoid panicking on multi-byte UTF-8.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        // Find the last char boundary at or before max_len
        let mut end = max_len;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
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

/// Show spec landscape: active, paused, blocked, drafts, and recommended next.
///
/// Wires into spec data functions (Phase 5 session integration).
/// Errors are swallowed — spec landscape is informational, not blocking.
fn show_spec_landscape() {
    let all_specs = match spec::get_all_specs(&spec::ListFilters::default()) {
        Ok(specs) => specs,
        Err(_) => return, // DB not initialized or no specs — skip silently
    };

    if all_specs.is_empty() {
        return;
    }

    let active: Vec<_> = all_specs
        .iter()
        .filter(|s| s.status == Some(SpecStatus::Active))
        .collect();
    let paused: Vec<_> = all_specs
        .iter()
        .filter(|s| s.status == Some(SpecStatus::Paused))
        .collect();
    let blocked: Vec<_> = all_specs
        .iter()
        .filter(|s| s.status == Some(SpecStatus::Blocked))
        .collect();
    let ready: Vec<_> = all_specs
        .iter()
        .filter(|s| s.status == Some(SpecStatus::Ready))
        .collect();
    let drafts: Vec<_> = all_specs
        .iter()
        .filter(|s| s.status == Some(SpecStatus::Draft))
        .collect();

    println!();
    println!("Spec landscape:");

    for s in &active {
        println!("  Active:  {}", s.id);
    }

    // Blocked specs with blocker info
    if !blocked.is_empty() {
        let blocked_specs = spec::get_blocked_specs().unwrap_or_default();
        for s in &blocked {
            let blocker_info = blocked_specs
                .iter()
                .find(|b| b.id == s.id)
                .map(|b| {
                    b.blocked_by
                        .iter()
                        .map(|bl| format!("{} ({})", bl.id, bl.status))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            if blocker_info.is_empty() {
                println!("  Blocked: {}", s.id);
            } else {
                println!("  Blocked: {} (waiting on {})", s.id, blocker_info);
            }
        }
    }

    for s in &paused {
        let age = spec::spec_age_days_from_list(s);
        let age_str = if age > 0 {
            format!(" ({}d)", age)
        } else {
            String::new()
        };
        let warning = if age > 14 { " — overdue" } else { "" };
        println!(
            "  Paused:  {}{} — resolve before starting new work{}",
            s.id, age_str, warning
        );
    }

    if !ready.is_empty() {
        println!("  Ready:   {} available", ready.len());
    }

    if !drafts.is_empty() {
        println!("  Drafts:  {} available", drafts.len());
    }

    // Recommendation (lightweight version of next_spec logic)
    if !active.is_empty() {
        println!();
        println!("Recommended: continue {} (active)", active[0].id);
    } else if !paused.is_empty() {
        let top = &paused[0];
        let age = spec::spec_age_days_from_list(top);
        println!();
        println!("Recommended: resume {} (paused {}d)", top.id, age);
    } else if !ready.is_empty() {
        // Pick highest-impact ready spec
        let dep_counts = spec::load_dep_counts();
        let top = ready
            .iter()
            .max_by_key(|s| dep_counts.get(&s.id).copied().unwrap_or(0))
            .unwrap();
        println!();
        println!("Recommended: start {} (ready)", top.id);
    } else if !drafts.is_empty() {
        println!();
        println!("Recommended: promote a draft to ready");
    }
}

/// Show spec status changes in session update.
///
/// Checks for SPEC.md files in changed files (indicates spec status mutations),
/// and warns about paused specs that are aging.
fn show_spec_status_in_update(changed_files: &[String]) {
    // Check if any spec files were changed this session
    let spec_changes: Vec<_> = changed_files
        .iter()
        .filter(|f| f.starts_with("layer/surface/build/") && f.ends_with("SPEC.md"))
        .collect();

    if !spec_changes.is_empty() {
        println!();
        println!("Spec files changed this session:");
        for f in &spec_changes {
            println!("  {}", f);
        }
    }

    // Warn about paused specs aging
    if let Ok(all_specs) = spec::get_all_specs(&spec::ListFilters::default()) {
        let paused: Vec<_> = all_specs
            .iter()
            .filter(|s| s.status == Some(SpecStatus::Paused))
            .collect();
        if !paused.is_empty() {
            println!();
            for s in &paused {
                let age = spec::spec_age_days_from_list(s);
                if age > 14 {
                    println!(
                        "Warning: {} paused for {}d — overdue for resolution",
                        s.id, age
                    );
                } else if age > 0 {
                    println!(
                        "Paused spec: {} ({}d) — resolve before pausing another",
                        s.id, age
                    );
                } else {
                    println!("Paused spec: {} — resolve before pausing another", s.id);
                }
            }
        }
    }
}

/// Show spec summary at session end: next recommendation and unblock detection.
///
/// If spec files were changed, detects completed specs and shows what they unblock.
/// Always shows the next recommended spec.
fn show_spec_end_summary(changed_files: &[String]) {
    let all_specs = match spec::get_all_specs(&spec::ListFilters::default()) {
        Ok(specs) => specs,
        Err(_) => return,
    };

    if all_specs.is_empty() {
        return;
    }

    // Check for spec completions in changed files
    let spec_changes: Vec<_> = changed_files
        .iter()
        .filter(|f| f.starts_with("layer/surface/build/") && f.ends_with("SPEC.md"))
        .collect();

    if !spec_changes.is_empty() {
        // Check what specs got completed/paused this session
        let blocked_specs = spec::get_blocked_specs().unwrap_or_default();
        let unblocked: Vec<_> = blocked_specs
            .iter()
            .filter(|b| {
                b.blocked_by.is_empty() || b.blocked_by.iter().all(|bl| bl.status.is_terminal())
            })
            .collect();

        if !unblocked.is_empty() {
            println!();
            println!("Specs unblocked:");
            for s in &unblocked {
                println!("  {} — ready to resume", s.id);
            }
        }
    }

    // Next spec recommendation
    let active: Vec<_> = all_specs
        .iter()
        .filter(|s| s.status == Some(SpecStatus::Active))
        .collect();
    let paused: Vec<_> = all_specs
        .iter()
        .filter(|s| s.status == Some(SpecStatus::Paused))
        .collect();
    let ready: Vec<_> = all_specs
        .iter()
        .filter(|s| s.status == Some(SpecStatus::Ready))
        .collect();
    let drafts: Vec<_> = all_specs
        .iter()
        .filter(|s| s.status == Some(SpecStatus::Draft))
        .collect();

    println!();
    println!("Next session:");
    if !active.is_empty() {
        println!("  Continue: {} (active)", active[0].id);
    } else if !paused.is_empty() {
        let top = &paused[0];
        let age = spec::spec_age_days_from_list(top);
        println!("  Resume: {} (paused {}d)", top.id, age);
    } else if !ready.is_empty() {
        println!("  Start: {} (ready)", ready[0].id);
    } else if !drafts.is_empty() {
        println!("  Promote a draft to ready");
    } else {
        println!("  No specs queued — create a new spec");
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

pub(crate) fn list_sessions_value(project_root: &Path) -> Result<SessionListResult> {
    let session_path = project_root.join(ACTIVE_SESSION_PATH);
    let sessions_dir = project_root.join(SESSIONS_DIR);

    // 1. Check for compatibility active session
    let compatibility_active = if session_path.exists() {
        let content = fs::read_to_string(&session_path)?;
        parse_session_summary(&content)
    } else {
        None
    };

    let mut active: Vec<SessionSummary> = Vec::new();
    if let Some(summary) = compatibility_active {
        active.push(summary);
    }
    for live in session::list_active_sessions(project_root)? {
        if active.iter().any(|existing| existing.id == live.file_id) {
            continue;
        }
        active.push(SessionSummary {
            id: live.file_id,
            title: live.title,
            status: "active".to_string(),
            created: live.created_at,
            runtime_id: Some(live.runtime_id),
            adapter: Some(live.adapter_name),
            interface: Some(live.interface_kind.as_str().to_string()),
            artifact_path: Some(live.artifact_path.display().to_string()),
        });
    }

    // 2. Scan layer/sessions/ for archived + stale
    let mut stale: Vec<SessionSummary> = Vec::new();
    let mut recent: Vec<SessionSummary> = Vec::new();

    if sessions_dir.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(&sessions_dir)?
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .collect();

        // Sort by filename descending (newest first, since filenames are timestamps)
        entries.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

        for entry in &entries {
            let content = match fs::read_to_string(entry.path()) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let Some(summary) = parse_session_summary(&content) else {
                continue;
            };

            if active.iter().any(|existing| existing.id == summary.id) {
                continue;
            }

            if summary.status == "active" {
                stale.push(summary);
            } else if recent.len() < 5 {
                recent.push(summary);
            }

            // Stop scanning after we have enough recent + checked all stale candidates
            // (stale could be anywhere, but in practice they're rare)
            if recent.len() >= 5 && stale.len() >= 20 {
                break;
            }
        }
    }

    let now = Utc::now();

    Ok(SessionListResult {
        active: active
            .iter()
            .map(|s| session_summary_entry(s, &now))
            .collect(),
        stale: stale
            .iter()
            .map(|s| session_summary_entry(s, &now))
            .collect(),
        recent: recent
            .iter()
            .map(|s| session_summary_entry(s, &now))
            .collect(),
    })
}

pub fn list_sessions(project_root: &Path, json_output: bool) -> Result<()> {
    let list = list_sessions_value(project_root)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&list)?);
        return Ok(());
    }

    // Human output
    let mut any_output = false;

    for s in &list.active {
        println!("ACTIVE  {}  {} ({})", s.id, s.title, s.age);
        any_output = true;
    }

    if !list.stale.is_empty() {
        if any_output {
            println!();
        }
        for s in &list.stale {
            println!("STALE   {}  {} ({}, never ended)", s.id, s.title, s.age);
        }
        any_output = true;
    }

    if !list.recent.is_empty() {
        if any_output {
            println!();
        }
        for s in &list.recent {
            println!("RECENT  {}  {} ({})", s.id, s.title, s.age);
        }
        any_output = true;
    }

    if !any_output {
        println!("No sessions found.");
    }

    Ok(())
}

/// Lightweight session summary for list display.
struct SessionSummary {
    id: String,
    title: String,
    status: String,
    created: String,
    runtime_id: Option<String>,
    adapter: Option<String>,
    interface: Option<String>,
    artifact_path: Option<String>,
}

/// Parse a session file into a lightweight summary.
/// Handles both YAML frontmatter (new) and legacy `**Field**: value` format.
fn parse_session_summary(content: &str) -> Option<SessionSummary> {
    // Try YAML frontmatter first
    if let Some(fm) = parse_session_frontmatter(content) {
        return Some(SessionSummary {
            id: fm.id,
            title: fm.title,
            status: fm.status,
            created: fm.created,
            runtime_id: Some(fm.runtime_id),
            adapter: Some(fm.llm),
            interface: Some(fm.interface),
            artifact_path: None,
        });
    }

    // Legacy format: extract from **Field**: value lines
    let mut id = None;
    let mut title = None;
    let mut created = None;

    for line in content.lines() {
        if let Some(v) = line.strip_prefix("**ID**: ") {
            id = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("# Session: ") {
            title = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("**Started**: ") {
            created = Some(v.trim().to_string());
        }
    }

    Some(SessionSummary {
        id: id?,
        title: title.unwrap_or_else(|| "untitled".to_string()),
        status: "archived".to_string(), // legacy sessions are all completed
        created: created.unwrap_or_default(),
        runtime_id: None,
        adapter: None,
        interface: None,
        artifact_path: None,
    })
}

/// Format a session's age from its created timestamp to now.
fn format_session_age(created: &str, now: &chrono::DateTime<Utc>) -> String {
    let Ok(created_dt) = chrono::DateTime::parse_from_rfc3339(created).or_else(|_| {
        chrono::DateTime::parse_from_rfc3339(&format!("{}+00:00", created.trim_end_matches('Z')))
    }) else {
        // Try parsing just the date portion from the ID
        return "unknown age".to_string();
    };

    let duration = *now - created_dt.with_timezone(&Utc);
    let days = duration.num_days();
    let hours = duration.num_hours();

    if days > 0 {
        format!("{}d ago", days)
    } else if hours > 0 {
        format!("{}h ago", hours)
    } else {
        "just now".to_string()
    }
}

/// Compute a session's age in hours from its created timestamp.
fn session_age_hours(created: &str) -> i64 {
    let Ok(created_dt) = chrono::DateTime::parse_from_rfc3339(created).or_else(|_| {
        chrono::DateTime::parse_from_rfc3339(&format!("{}+00:00", created.trim_end_matches('Z')))
    }) else {
        return 0;
    };
    (Utc::now() - created_dt.with_timezone(&Utc)).num_hours()
}

/// Convert a SessionSummary to JSON value.
fn session_summary_entry(s: &SessionSummary, now: &chrono::DateTime<Utc>) -> SessionListEntry {
    SessionListEntry {
        id: s.id.clone(),
        runtime_id: s.runtime_id.clone(),
        title: s.title.clone(),
        status: s.status.clone(),
        created: s.created.clone(),
        age: format_session_age(&s.created, now),
        adapter: s.adapter.clone(),
        interface: s.interface.clone(),
        artifact_path: s.artifact_path.clone(),
    }
}

/// Get the configured development branch name.
///
/// Reads from .patina/config.toml [project] branch, falls back to "work".
fn dev_branch_name(project_root: &Path) -> String {
    patina::project::load(project_root)
        .map(|c| c.project.branch)
        .unwrap_or_else(|_| "work".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use patina::project::{self, ProjectConfig};
    use tempfile::TempDir;

    fn setup_project() -> TempDir {
        let temp = TempDir::new().unwrap();
        let mut config = ProjectConfig::with_name("patina");
        config.adapters.allowed = vec!["opencode".to_string(), "gemini".to_string()];
        config.adapters.default = "opencode".to_string();
        project::save(temp.path(), &config).unwrap();
        fs::create_dir_all(temp.path().join(".patina/local/data")).unwrap();
        temp
    }

    fn in_project<T>(project_root: &Path, f: impl FnOnce() -> T) -> T {
        let _guard = patina::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let old_dir = std::env::current_dir().ok();
        let patina_home = project_root.join("patina-home");
        fs::create_dir_all(&patina_home).unwrap();
        let old_patina_home = std::env::var_os("PATINA_HOME");
        std::env::set_current_dir(project_root).unwrap();
        unsafe {
            std::env::set_var("PATINA_HOME", &patina_home);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        if let Some(path) = old_dir {
            let _ = std::env::set_current_dir(path);
        }
        match old_patina_home {
            Some(value) => unsafe {
                std::env::set_var("PATINA_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("PATINA_HOME");
            },
        }
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    fn session_list_value_serializes_recent_archived_sessions() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join(".patina")).unwrap();
        fs::create_dir_all(temp.path().join("layer/sessions")).unwrap();
        fs::write(
            temp.path().join("layer/sessions/20260311-101500-ABCD.md"),
            r#"---
type: session
id: 20260311-101500-ABCD
runtime_id: runtime-123
title: Build OpenCode slice
status: archived
llm: opencode
interface: opencode
created: 2026-03-11T10:15:00Z
updated: 2026-03-11T11:00:00Z
start_timestamp: 1741688100000
git:
  project_uid: proj-123
  branch: patina
  starting_commit: abc123
  start_tag: session-20260311-101500-ABCD-opencode-start
  end_tag: session-20260311-101500-ABCD-opencode-end
---

## Outcome
"#,
        )
        .unwrap();

        let result = list_sessions_value(temp.path()).unwrap();
        let json = serde_json::to_value(&result).unwrap();

        assert_eq!(json["recent"].as_array().unwrap().len(), 1);
        assert_eq!(
            json["recent"][0]["id"].as_str(),
            Some("20260311-101500-ABCD")
        );
        assert_eq!(json["recent"][0]["status"].as_str(), Some("archived"));
    }

    #[test]
    fn session_update_result_json_is_machine_readable() {
        let result = SessionUpdateResult {
            command: "update",
            session_id: "20260311-101500-ABCD".to_string(),
            runtime_id: "runtime-123".to_string(),
            artifact_path: "/tmp/session.md".to_string(),
            branch: "patina".to_string(),
            start_tag: "session-20260311-101500-ABCD-opencode-start".to_string(),
            since: "10:15".to_string(),
            updated_at: "2026-03-11T11:00:00Z".to_string(),
            commits_this_session: 2,
            recent_commits: vec!["abc123 first".to_string()],
            session_changed_files: vec!["src/mcp/server/session.rs".to_string()],
            modified_files: 1,
            staged_files: 0,
            untracked_files: 0,
            lines_changed: 42,
            last_commit_time: "5 minutes ago".to_string(),
            last_commit_message: "feat: wire session mcp".to_string(),
            working_tree_clean: false,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["command"].as_str(), Some("update"));
        assert_eq!(json["commits_this_session"].as_u64(), Some(2));
        assert!(json["recent_commits"].is_array());
        assert!(json["session_changed_files"].is_array());
    }

    #[test]
    fn resolve_live_session_prefers_native_interface_filter() {
        let temp = setup_project();
        let started = in_project(temp.path(), || {
            start_session_value(
                temp.path(),
                SessionStartRequest::native("Native session", "opencode"),
            )
            .unwrap()
        });

        unsafe {
            std::env::set_var("PATINA_AI_INTERFACE", "opencode");
        }
        let resolved = in_project(temp.path(), || {
            resolve_live_session(temp.path(), None, None)
        })
        .unwrap();
        unsafe {
            std::env::remove_var("PATINA_AI_INTERFACE");
        }

        assert_eq!(resolved.file_id, started.session_id);
        assert_eq!(resolved.interface_kind, InterfaceKind::OpenCode);
    }

    #[test]
    fn resolve_live_session_prefers_explicit_selector_over_pointer_and_env() {
        let temp = setup_project();
        let first = in_project(temp.path(), || {
            start_session_value(
                temp.path(),
                SessionStartRequest::native("First native session", "opencode"),
            )
            .unwrap()
        });
        let second = in_project(temp.path(), || {
            start_session_value(
                temp.path(),
                SessionStartRequest::native("Second native session", "gemini"),
            )
            .unwrap()
        });

        let resolved = in_project(temp.path(), || {
            unsafe {
                std::env::set_var("PATINA_SESSION_RUNTIME_ID", &first.runtime_id);
                std::env::set_var("PATINA_AI_INTERFACE", "opencode");
            }
            let result = resolve_live_session(temp.path(), Some(&second.runtime_id), None);
            unsafe {
                std::env::remove_var("PATINA_SESSION_RUNTIME_ID");
                std::env::remove_var("PATINA_AI_INTERFACE");
            }
            result
        })
        .unwrap();

        assert_eq!(resolved.file_id, second.session_id);
        assert_ne!(resolved.file_id, first.session_id);
        assert_eq!(resolved.interface_kind, InterfaceKind::Gemini);
    }

    #[test]
    fn resolve_live_session_prefers_interface_pointer_over_stale_launch_env() {
        let temp = setup_project();
        let first = in_project(temp.path(), || {
            start_session_value(
                temp.path(),
                SessionStartRequest::native("First native session", "opencode"),
            )
            .unwrap()
        });
        let second = in_project(temp.path(), || {
            start_session_value(
                temp.path(),
                SessionStartRequest::native("Second native session", "opencode"),
            )
            .unwrap()
        });

        let resolved = in_project(temp.path(), || {
            unsafe {
                std::env::set_var("PATINA_SESSION_RUNTIME_ID", &first.runtime_id);
                std::env::set_var("PATINA_AI_INTERFACE", "opencode");
            }
            let result = resolve_live_session(temp.path(), None, None);
            unsafe {
                std::env::remove_var("PATINA_SESSION_RUNTIME_ID");
                std::env::remove_var("PATINA_AI_INTERFACE");
            }
            result
        })
        .unwrap();

        assert_eq!(resolved.file_id, second.session_id);
        assert_ne!(resolved.file_id, first.session_id);
        assert_eq!(resolved.interface_kind, InterfaceKind::OpenCode);
    }

    #[test]
    fn note_live_session_appends_to_native_artifact() {
        let temp = setup_project();
        let started = in_project(temp.path(), || {
            start_session_value(
                temp.path(),
                SessionStartRequest::native("Native session", "opencode"),
            )
            .unwrap()
        });

        let handle = in_project(temp.path(), || {
            resolve_live_session(temp.path(), Some(&started.runtime_id), None).unwrap()
        });

        in_project(temp.path(), || {
            note_live_session(temp.path(), &handle, "captured wrapper-first UX").unwrap()
        });

        let artifact = fs::read_to_string(&started.artifact_path).unwrap();
        assert!(artifact.contains("captured wrapper-first UX"));
        assert!(artifact.contains("### "));
        assert!(!temp.path().join(ACTIVE_SESSION_PATH).exists());
    }

    #[test]
    fn end_live_session_clears_native_interface_pointer() {
        let temp = setup_project();
        let started = in_project(temp.path(), || {
            start_session_value(
                temp.path(),
                SessionStartRequest::native("Native session", "opencode"),
            )
            .unwrap()
        });
        let pointer_path = temp
            .path()
            .join(".patina/local/interface-sessions/opencode.toml");
        assert!(pointer_path.exists());

        let handle = in_project(temp.path(), || {
            resolve_live_session(temp.path(), Some(&started.runtime_id), None).unwrap()
        });
        in_project(temp.path(), || {
            end_live_session_value(temp.path(), &handle, Some("archive")).unwrap()
        });

        assert!(!pointer_path.exists());
    }
}
