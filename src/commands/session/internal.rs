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
use patina::session::{
    self, ArchiveSessionRequest, BeginSessionRequest, InterfaceKind, SessionParticipant,
};

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
    let db_path = patina::eventlog::resolve_patina_db_path(project_root);
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
        fs::create_dir_all(temp.path().join(".patina").join("local").join("data")).unwrap();
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
        assert!(!temp.path().join(".patina/local/active-session.md").exists());
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
