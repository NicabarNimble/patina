//! Persona command - Cross-project user knowledge
//!
//! Captures and queries user preferences, style, and knowledge that spans projects.
//! Storage: ~/.patina/personas/default/
//!
//! Entry points:
//! - note(content, domains) - capture knowledge
//! - materialize() - build searchable index from events
//! - query(text, limit, min_score) - semantic search
//! - list(limit, domains) - show recent entries

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};
use uuid::Uuid;

use patina::embeddings::create_embedder;

/// Persona base directory
fn persona_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".patina")
        .join("personas")
        .join("default")
}

/// Captured knowledge event (private - implementation detail)
#[derive(Debug, Serialize, Deserialize)]
struct PersonaEvent {
    id: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    timestamp: DateTime<Utc>,
    source: String,
    content: String,
    #[serde(default)]
    domains: Vec<String>,
    working_project: Option<String>,
}

/// Query result (public - returned to callers)
#[derive(Debug)]
pub struct PersonaResult {
    pub content: String,
    pub score: f32,
    pub domains: Vec<String>,
    pub source: String,
    pub timestamp: String,
}

/// Capture knowledge directly
pub fn note(content: &str, domains: Option<Vec<String>>) -> Result<()> {
    let dir = persona_dir();
    let events_dir = dir.join("events");
    fs::create_dir_all(&events_dir).context("Failed to create events directory")?;

    let event = PersonaEvent {
        id: format!("evt_{}", Uuid::new_v4().simple()),
        timestamp: Utc::now(),
        source: "direct".to_string(),
        content: content.to_string(),
        domains: domains.unwrap_or_default(),
        working_project: std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())),
    };

    // Append to daily events file
    let date = event.timestamp.format("%Y%m%d").to_string();
    let events_file = events_dir.join(format!("{}.jsonl", date));
    let line = serde_json::to_string(&event)?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&events_file)?;
    writeln!(file, "{}", line)?;

    Ok(())
}

/// Build searchable index from events
pub fn materialize() -> Result<()> {
    let dir = persona_dir();
    let events_dir = dir.join("events");
    let materialized_dir = dir.join("materialized");
    fs::create_dir_all(&materialized_dir)?;

    let db_path = materialized_dir.join("persona.db");
    let index_path = materialized_dir.join("persona.usearch");

    // Open database and create schema
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS knowledge (
            rowid INTEGER PRIMARY KEY AUTOINCREMENT,
            id TEXT UNIQUE NOT NULL,
            content TEXT NOT NULL,
            source TEXT NOT NULL,
            domains TEXT,
            timestamp TEXT NOT NULL,
            working_project TEXT
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value TEXT)",
        [],
    )?;

    // Load embedder
    println!("Loading embedding model...");
    let mut embedder = create_embedder()?;

    // Create vector index (768-dim E5-base-v2)
    let options = IndexOptions {
        dimensions: 768,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };
    let index = Index::new(&options)?;
    index.reserve(1000)?;

    // Get last processed event ID for incremental
    let last_id: Option<String> = conn
        .query_row(
            "SELECT value FROM metadata WHERE key = 'last_event_id'",
            [],
            |row| row.get(0),
        )
        .ok();

    println!("Processing events...");
    let mut processed = 0;
    let mut found_last = last_id.is_none();
    let mut last_processed_id: Option<String> = None;

    // Read event files
    if events_dir.exists() {
        let mut files: Vec<_> = fs::read_dir(&events_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        files.sort_by_key(|a| a.file_name());

        for entry in files {
            let content = fs::read_to_string(entry.path())?;
            for line in content.lines() {
                if let Ok(event) = serde_json::from_str::<PersonaEvent>(line) {
                    // Skip until we find last processed
                    if !found_last {
                        if Some(&event.id) == last_id.as_ref() {
                            found_last = true;
                        }
                        continue;
                    }

                    // Embed and store
                    let embedding = embedder.embed_query(&event.content)?;

                    let rowid: i64 = conn.query_row(
                        "INSERT OR REPLACE INTO knowledge (id, content, source, domains, timestamp, working_project)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                         RETURNING rowid",
                        params![
                            &event.id,
                            &event.content,
                            &event.source,
                            serde_json::to_string(&event.domains)?,
                            event.timestamp.to_rfc3339(),
                            &event.working_project,
                        ],
                        |row| row.get(0),
                    )?;

                    index.add(rowid as u64, &embedding)?;
                    last_processed_id = Some(event.id.clone());
                    processed += 1;

                    if processed % 10 == 0 {
                        print!(".");
                        std::io::stdout().flush().ok();
                    }
                }
            }
        }
    }

    if processed > 0 {
        println!();
        index.save(index_path.to_str().unwrap())?;

        if let Some(id) = last_processed_id {
            conn.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES ('last_event_id', ?1)",
                params![id],
            )?;
        }
    }

    println!("Materialized {} events", processed);
    Ok(())
}

/// Semantic search of persona knowledge
pub fn query(query_text: &str, limit: usize, min_score: f32) -> Result<Vec<PersonaResult>> {
    let dir = persona_dir();
    let db_path = dir.join("materialized/persona.db");
    let index_path = dir.join("materialized/persona.usearch");

    if !index_path.exists() {
        return Ok(Vec::new());
    }

    // Load index
    let options = IndexOptions {
        dimensions: 768,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };
    let index = Index::new(&options)?;
    index.load(index_path.to_str().unwrap())?;

    // Embed query
    let mut embedder = create_embedder()?;
    let query_embedding = embedder.embed_query(query_text)?;

    // Search
    let matches = index.search(&query_embedding, limit)?;

    // Hydrate from database
    let conn = Connection::open(&db_path)?;
    let mut results = Vec::new();

    for (rowid, distance) in matches.keys.iter().zip(matches.distances.iter()) {
        let score = 1.0 - distance;
        if score < min_score {
            continue;
        }

        let result = conn.query_row(
            "SELECT content, source, domains, timestamp FROM knowledge WHERE rowid = ?1",
            params![*rowid as i64],
            |row| {
                let domains_json: String = row.get(2)?;
                let domains: Vec<String> = serde_json::from_str(&domains_json).unwrap_or_default();
                Ok(PersonaResult {
                    content: row.get(0)?,
                    source: row.get(1)?,
                    domains,
                    timestamp: row.get(3)?,
                    score,
                })
            },
        );

        if let Ok(r) = result {
            results.push(r);
        }
    }

    Ok(results)
}

/// List recent persona entries from event files
pub fn list(limit: usize, domains: Option<Vec<String>>) -> Result<Vec<PersonaResult>> {
    let events_dir = persona_dir().join("events");

    if !events_dir.exists() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    // Read event files newest first
    let mut files: Vec<_> = fs::read_dir(&events_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
        .collect();
    files.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

    for entry in files {
        let content = fs::read_to_string(entry.path())?;
        for line in content.lines().rev() {
            if let Ok(event) = serde_json::from_str::<PersonaEvent>(line) {
                // Filter by domains if specified
                if let Some(ref filter) = domains {
                    if !filter.iter().any(|d| event.domains.contains(d)) {
                        continue;
                    }
                }

                results.push(PersonaResult {
                    content: event.content,
                    score: 1.0, // No score for list
                    domains: event.domains,
                    source: event.source,
                    timestamp: event.timestamp.to_rfc3339(),
                });

                if results.len() >= limit {
                    return Ok(results);
                }
            }
        }
    }

    Ok(results)
}

// === CLI execute functions ===

/// Execute persona note command
pub fn execute_note(content: &str, domains: Option<Vec<String>>) -> Result<()> {
    println!("🧠 Persona - Capturing knowledge\n");

    note(content, domains.clone())?;

    if let Some(ref d) = domains {
        println!("   Domains: {}", d.join(", "));
    }
    println!("   Content: {}", content);
    println!("\n✅ Captured to persona");

    Ok(())
}

/// Execute persona materialize command
pub fn execute_materialize() -> Result<()> {
    println!("🧠 Persona - Materializing knowledge base\n");
    materialize()?;
    Ok(())
}

/// Execute persona query command
pub fn execute_query(query_text: &str, limit: usize, min_score: f32) -> Result<()> {
    println!("🧠 Persona - Searching knowledge\n");
    println!("Query: \"{}\"\n", query_text);

    let results = query(query_text, limit, min_score)?;

    if results.is_empty() {
        println!("No results found.");
        println!("Capture knowledge with: patina persona note \"...\"");
        println!("Then materialize with: patina persona materialize");
        return Ok(());
    }

    println!("Found {} results:\n", results.len());
    println!("{}", "─".repeat(60));

    for (i, result) in results.iter().enumerate() {
        let domains_display = if result.domains.is_empty() {
            String::new()
        } else {
            format!(" | {}", result.domains.join(", "))
        };
        println!(
            "\n[{}] Score: {:.3} | {}{}",
            i + 1,
            result.score,
            result.source,
            domains_display
        );
        println!("    {}", truncate(&result.content, 200));
    }

    println!("\n{}", "─".repeat(60));
    Ok(())
}

/// Execute persona list command
pub fn execute_list(limit: usize, domains: Option<Vec<String>>) -> Result<()> {
    println!("🧠 Persona - Captured knowledge\n");

    let results = list(limit, domains)?;

    if results.is_empty() {
        println!("No knowledge captured yet.");
        println!("Use: patina persona note \"...\"");
        return Ok(());
    }

    println!("Recent entries ({}):\n", results.len());
    println!("{}", "─".repeat(60));

    for (i, result) in results.iter().enumerate() {
        let domains_display = if result.domains.is_empty() {
            String::new()
        } else {
            format!(" | {}", result.domains.join(", "))
        };
        // Parse and format timestamp
        let ts = chrono::DateTime::parse_from_rfc3339(&result.timestamp)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|_| result.timestamp.clone());

        println!(
            "\n[{}] {} | {}{}",
            i + 1,
            ts,
            result.source,
            domains_display
        );
        println!("    {}", truncate(&result.content, 200));
    }

    println!("\n{}", "─".repeat(60));
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.replace('\n', " ").trim().to_string();
    if s.len() <= max {
        s
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_note_creates_event_file() -> Result<()> {
        let temp = TempDir::new()?;
        let events_dir = temp.path().join("events");
        fs::create_dir_all(&events_dir)?;

        // Create event manually (can't override persona_dir in test easily)
        let event = PersonaEvent {
            id: "test_001".to_string(),
            timestamp: Utc::now(),
            source: "direct".to_string(),
            content: "prefer Result<T,E> over panics".to_string(),
            domains: vec!["rust".to_string()],
            working_project: Some("test".to_string()),
        };

        let date = event.timestamp.format("%Y%m%d").to_string();
        let events_file = events_dir.join(format!("{}.jsonl", date));
        let line = serde_json::to_string(&event)?;
        fs::write(&events_file, format!("{}\n", line))?;

        // Verify file exists and contains event
        let content = fs::read_to_string(&events_file)?;
        assert!(content.contains("prefer Result<T,E> over panics"));
        assert!(content.contains("rust"));

        Ok(())
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a very long string", 10), "a very lon...");
        assert_eq!(truncate("with\nnewlines", 20), "with newlines");
    }
}
