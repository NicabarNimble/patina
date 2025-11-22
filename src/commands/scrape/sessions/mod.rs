//! Session file scraper - extracts sessions, goals, and observations from markdown
//!
//! Uses unified eventlog pattern:
//! - Inserts session.* events into eventlog table
//! - Creates materialized views (sessions, observations, goals) from eventlog

use anyhow::Result;
use regex::Regex;
use rusqlite::Connection;
use serde_json::json;
use std::path::Path;
use std::time::Instant;

use super::database;
use super::ScrapeStats;

const SESSIONS_DIR: &str = "layer/sessions";

/// Parsed session from markdown file
#[derive(Debug)]
struct ParsedSession {
    id: String,
    title: String,
    started_at: Option<String>,
    ended_at: Option<String>,
    branch: Option<String>,
    classification: Option<String>,
    files_changed: i32,
    commits_made: i32,
    goals: Vec<Goal>,
    observations: Vec<Observation>,
}

#[derive(Debug)]
struct Goal {
    content: String,
    completed: bool,
}

#[derive(Debug)]
struct Observation {
    content: String,
    observation_type: String,
    timestamp: Option<String>,
}

/// Create materialized views for session events
///
/// Views are derived from eventlog WHERE event_type LIKE 'session.%'
fn create_materialized_views(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Sessions view (materialized from session.started events)
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT,
            started_at TEXT,
            ended_at TEXT,
            branch TEXT,
            classification TEXT,
            files_changed INTEGER,
            commits_made INTEGER,
            file_path TEXT
        );

        -- Observations extracted from sessions (from session.observation events)
        CREATE TABLE IF NOT EXISTS observations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT,
            content TEXT,
            observation_type TEXT,
            timestamp TEXT,
            FOREIGN KEY (session_id) REFERENCES sessions(id)
        );

        -- Goals per session (from session.goal events)
        CREATE TABLE IF NOT EXISTS goals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT,
            content TEXT,
            completed INTEGER,
            FOREIGN KEY (session_id) REFERENCES sessions(id)
        );

        -- Indexes
        CREATE INDEX IF NOT EXISTS idx_observations_session ON observations(session_id);
        CREATE INDEX IF NOT EXISTS idx_observations_type ON observations(observation_type);
        CREATE INDEX IF NOT EXISTS idx_goals_session ON goals(session_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_branch ON sessions(branch);
        "#,
    )?;

    Ok(())
}

/// Parse a session markdown file
fn parse_session_file(path: &Path) -> Result<ParsedSession> {
    let content = std::fs::read_to_string(path)?;

    // Extract ID from filename (e.g., 20251121-113107.md -> 20251121-113107)
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Extract title from first line: # Session: <title>
    let title_re = Regex::new(r"^# Session: (.+)$").unwrap();
    let title = content
        .lines()
        .find_map(|line| title_re.captures(line).map(|c| c[1].to_string()))
        .unwrap_or_else(|| id.clone());

    // Extract metadata fields
    let started_at = extract_field(&content, "Started");
    let branch = extract_field(&content, "Git Branch");
    let classification = extract_classification(&content);
    let (files_changed, commits_made) = extract_stats(&content);

    // Extract goals
    let goals = extract_goals(&content);

    // Extract observations from various sections
    let observations = extract_observations(&content);

    Ok(ParsedSession {
        id,
        title,
        started_at,
        ended_at: None, // Could parse from end timestamp if available
        branch,
        classification,
        files_changed,
        commits_made,
        goals,
        observations,
    })
}

/// Extract a **Field**: value pattern
fn extract_field(content: &str, field: &str) -> Option<String> {
    let pattern = format!(r"\*\*{}\*\*:\s*(.+)", regex::escape(field));
    let re = Regex::new(&pattern).ok()?;
    re.captures(content).map(|c| c[1].trim().to_string())
}

/// Extract classification from Session Classification section
fn extract_classification(content: &str) -> Option<String> {
    let re = Regex::new(r"Work Type:\s*(\w+)").ok()?;
    re.captures(content).map(|c| c[1].to_string())
}

/// Extract files changed and commits from stats
fn extract_stats(content: &str) -> (i32, i32) {
    let files_re = Regex::new(r"Files Changed:\s*(\d+)").ok();
    let commits_re = Regex::new(r"Commits:\s*(\d+)").ok();

    let files = files_re
        .and_then(|re| re.captures(content))
        .and_then(|c| c[1].parse().ok())
        .unwrap_or(0);

    let commits = commits_re
        .and_then(|re| re.captures(content))
        .and_then(|c| c[1].parse().ok())
        .unwrap_or(0);

    (files, commits)
}

/// Extract goals from ## Goals section
fn extract_goals(content: &str) -> Vec<Goal> {
    let mut goals = Vec::new();

    // Find Goals section
    let goals_section = content
        .split("## Goals")
        .nth(1)
        .and_then(|s| s.split("\n## ").next())
        .unwrap_or("");

    // Parse checkbox items: - [ ] or - [x]
    let checkbox_re = Regex::new(r"- \[([xX ])\] (.+)").unwrap();
    for cap in checkbox_re.captures_iter(goals_section) {
        let completed = &cap[1] != " ";
        let content = cap[2].trim().to_string();
        goals.push(Goal { content, completed });
    }

    goals
}

/// Extract observations from various sections
fn extract_observations(content: &str) -> Vec<Observation> {
    let mut observations = Vec::new();

    // Extract from Key Decisions sections
    if let Some(decisions) = extract_section(content, "Key Decisions") {
        for line in decisions.lines() {
            let line = line.trim();
            if line.starts_with('-') || line.starts_with('*') {
                let text = line.trim_start_matches('-').trim_start_matches('*').trim();
                if !text.is_empty() {
                    observations.push(Observation {
                        content: text.to_string(),
                        observation_type: "decision".to_string(),
                        timestamp: None,
                    });
                }
            }
        }
    }

    // Extract from Patterns Observed sections
    if let Some(patterns) = extract_section(content, "Patterns Observed") {
        for line in patterns.lines() {
            let line = line.trim();
            if line.starts_with('-') || line.starts_with('*') {
                let text = line.trim_start_matches('-').trim_start_matches('*').trim();
                if !text.is_empty() {
                    observations.push(Observation {
                        content: text.to_string(),
                        observation_type: "pattern".to_string(),
                        timestamp: None,
                    });
                }
            }
        }
    }

    // Extract from Work Completed sections (numbered items)
    let work_re = Regex::new(r"^\d+\.\s+(.+)$").unwrap();
    if let Some(work) = extract_section(content, "Work Completed") {
        for line in work.lines() {
            if let Some(cap) = work_re.captures(line.trim()) {
                observations.push(Observation {
                    content: cap[1].to_string(),
                    observation_type: "work".to_string(),
                    timestamp: None,
                });
            }
        }
    }

    // Extract Previous Session Context as insight
    if let Some(ctx) = extract_section(content, "Previous Session Context") {
        let ctx = ctx.trim();
        if !ctx.is_empty() && !ctx.starts_with("<!--") {
            observations.push(Observation {
                content: ctx.to_string(),
                observation_type: "context".to_string(),
                timestamp: None,
            });
        }
    }

    observations
}

/// Extract a section by header
fn extract_section(content: &str, header: &str) -> Option<String> {
    // Look for **Header:** pattern or ## Header pattern
    let bold_pattern = format!(r"\*\*{}:\*\*\s*", regex::escape(header));
    let heading_pattern = format!(r"## {}\s*", regex::escape(header));

    // Try bold pattern first
    if let Ok(re) = Regex::new(&bold_pattern) {
        if let Some(m) = re.find(content) {
            let start = m.end();
            let rest = &content[start..];
            // Find next section marker
            let end = rest
                .find("\n**")
                .or_else(|| rest.find("\n## "))
                .unwrap_or(rest.len());
            return Some(rest[..end].to_string());
        }
    }

    // Try heading pattern
    if let Ok(re) = Regex::new(&heading_pattern) {
        if let Some(m) = re.find(content) {
            let start = m.end();
            let rest = &content[start..];
            let end = rest.find("\n## ").unwrap_or(rest.len());
            return Some(rest[..end].to_string());
        }
    }

    None
}

/// Insert a parsed session into eventlog and materialized views
///
/// Dual-write pattern:
/// 1. Insert session.started, session.goal, session.observation events into eventlog
/// 2. Update materialized views (sessions, goals, observations) for fast queries
fn insert_session(conn: &Connection, session: &ParsedSession, file_path: &str) -> Result<()> {
    // Delete existing data for this session (for re-scrapes)
    conn.execute(
        "DELETE FROM observations WHERE session_id = ?1",
        [&session.id],
    )?;
    conn.execute("DELETE FROM goals WHERE session_id = ?1", [&session.id])?;
    conn.execute("DELETE FROM sessions WHERE id = ?1", [&session.id])?;

    // Determine timestamp (use started_at if available, otherwise use ID-based timestamp)
    let timestamp = session.started_at.as_deref().unwrap_or(&session.id);

    // 1. Insert session.started event into eventlog
    let session_event = json!({
        "title": &session.title,
        "started_at": &session.started_at,
        "ended_at": &session.ended_at,
        "branch": &session.branch,
        "classification": &session.classification,
        "files_changed": session.files_changed,
        "commits_made": session.commits_made,
        "file_path": file_path,
    });

    database::insert_event(
        conn,
        "session.started",
        timestamp,
        &session.id,
        Some(file_path),
        &session_event.to_string(),
    )?;

    // 2. Insert materialized session view
    conn.execute(
        "INSERT INTO sessions (id, title, started_at, ended_at, branch, classification, files_changed, commits_made, file_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &session.id,
            &session.title,
            &session.started_at,
            &session.ended_at,
            &session.branch,
            &session.classification,
            session.files_changed,
            session.commits_made,
            file_path,
        ],
    )?;

    // 3. Insert goal events and materialized views
    let mut goal_stmt =
        conn.prepare("INSERT INTO goals (session_id, content, completed) VALUES (?1, ?2, ?3)")?;

    for goal in &session.goals {
        // Insert session.goal event
        let goal_event = json!({
            "session_id": &session.id,
            "content": &goal.content,
            "completed": goal.completed,
        });

        database::insert_event(
            conn,
            "session.goal",
            timestamp,
            &session.id,
            Some(file_path),
            &goal_event.to_string(),
        )?;

        // Insert materialized view
        goal_stmt.execute(rusqlite::params![
            &session.id,
            &goal.content,
            goal.completed as i32
        ])?;
    }

    // 4. Insert observation events and materialized views
    let mut obs_stmt = conn.prepare(
        "INSERT INTO observations (session_id, content, observation_type, timestamp) VALUES (?1, ?2, ?3, ?4)",
    )?;

    for obs in &session.observations {
        // Insert session.observation event (or more specific types)
        let event_type = match obs.observation_type.as_str() {
            "decision" => "session.decision",
            "pattern" => "session.pattern",
            "work" => "session.work",
            "context" => "session.context",
            _ => "session.observation",
        };

        let obs_event = json!({
            "session_id": &session.id,
            "content": &obs.content,
            "observation_type": &obs.observation_type,
        });

        database::insert_event(
            conn,
            event_type,
            obs.timestamp.as_deref().unwrap_or(timestamp),
            &session.id,
            Some(file_path),
            &obs_event.to_string(),
        )?;

        // Insert materialized view
        obs_stmt.execute(rusqlite::params![
            &session.id,
            &obs.content,
            &obs.observation_type,
            &obs.timestamp,
        ])?;
    }

    Ok(())
}

/// Main entry point for sessions scraping
pub fn run(full: bool) -> Result<ScrapeStats> {
    let start = Instant::now();
    let db_path = Path::new(database::PATINA_DB);
    let sessions_dir = Path::new(SESSIONS_DIR);

    if !sessions_dir.exists() {
        anyhow::bail!("Sessions directory not found: {}", SESSIONS_DIR);
    }

    // Initialize unified database with eventlog
    let conn = database::initialize(db_path)?;

    // Create materialized views for session events
    create_materialized_views(&conn)?;

    // Get list of already processed sessions for incremental
    let processed: std::collections::HashSet<String> = if full {
        std::collections::HashSet::new()
    } else {
        let mut stmt = conn.prepare("SELECT id FROM sessions")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.filter_map(|r| r.ok()).collect()
    };

    if full {
        println!("📚 Full session scrape...");
    } else {
        println!(
            "📚 Incremental session scrape ({} already processed)...",
            processed.len()
        );
    }

    // Find all session files
    let mut session_files: Vec<_> = std::fs::read_dir(sessions_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();

    session_files.sort_by_key(|e| e.path());

    let mut processed_count = 0;
    let mut total_observations = 0;
    let mut skipped = 0;

    for entry in &session_files {
        let path = entry.path();
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        // Skip if already processed (incremental mode)
        if !full && processed.contains(&id) {
            skipped += 1;
            continue;
        }

        match parse_session_file(&path) {
            Ok(session) => {
                total_observations += session.observations.len();
                if let Err(e) = insert_session(&conn, &session, path.to_string_lossy().as_ref()) {
                    eprintln!("  Warning: failed to insert {}: {}", id, e);
                } else {
                    processed_count += 1;
                }
            }
            Err(e) => {
                eprintln!("  Warning: failed to parse {}: {}", path.display(), e);
            }
        }
    }

    println!(
        "  Processed {} sessions ({} skipped)",
        processed_count, skipped
    );
    println!("  Extracted {} observations", total_observations);

    let elapsed = start.elapsed();
    let db_size = std::fs::metadata(db_path)
        .map(|m| m.len() / 1024)
        .unwrap_or(0);

    Ok(ScrapeStats {
        items_processed: processed_count,
        time_elapsed: elapsed,
        database_size_kb: db_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_goals() {
        let content = r#"## Goals
- [ ] implement feature
- [x] fix bug
- [ ] write tests
"#;
        let goals = extract_goals(content);
        assert_eq!(goals.len(), 3);
        assert!(!goals[0].completed);
        assert!(goals[1].completed);
        assert_eq!(goals[0].content, "implement feature");
    }

    #[test]
    fn test_extract_field() {
        let content = "**Started**: 2025-11-21T16:31:07Z\n**Branch**: main";
        assert_eq!(
            extract_field(content, "Started"),
            Some("2025-11-21T16:31:07Z".to_string())
        );
    }
}
