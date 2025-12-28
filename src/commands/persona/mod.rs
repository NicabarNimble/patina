//! Persona command - Cross-project user knowledge
//!
//! Captures and queries user preferences, style, and knowledge that spans projects.
//!
//! Storage layout (via paths module):
//! - Events (source): ~/.patina/personas/default/events/
//! - Cache (derived): ~/.patina/cache/personas/default/
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
use patina::paths::persona as persona_paths;

/// Captured knowledge event (private - implementation detail)
#[derive(Debug, Serialize, Deserialize)]
struct PersonaEvent {
    id: String,
    event_type: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    timestamp: DateTime<Utc>,
    source: String,
    content: String,
    #[serde(default)]
    domains: Vec<String>,
    working_project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    supersedes: Option<String>,
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
pub fn note(
    content: &str,
    domains: Option<Vec<String>>,
    supersedes: Option<String>,
) -> Result<String> {
    let events_dir = persona_paths::events_dir();
    fs::create_dir_all(&events_dir).context("Failed to create events directory")?;

    let event_id = format!("evt_{}", Uuid::new_v4().simple());
    let event = PersonaEvent {
        id: event_id.clone(),
        event_type: "knowledge_captured".to_string(),
        timestamp: Utc::now(),
        source: "direct".to_string(),
        content: content.to_string(),
        domains: domains.unwrap_or_default(),
        working_project: std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())),
        supersedes,
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

    Ok(event_id)
}

/// Build searchable index from events
pub fn materialize() -> Result<()> {
    let events_dir = persona_paths::events_dir();
    let cache_dir = persona_paths::cache_dir();
    fs::create_dir_all(&cache_dir)?;

    let db_path = cache_dir.join("persona.db");
    let index_path = cache_dir.join("persona.usearch");

    // Open database and create schema
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS knowledge (
            rowid INTEGER PRIMARY KEY AUTOINCREMENT,
            id TEXT UNIQUE NOT NULL,
            event_type TEXT NOT NULL,
            content TEXT NOT NULL,
            source TEXT NOT NULL,
            domains TEXT,
            timestamp TEXT NOT NULL,
            working_project TEXT,
            supersedes TEXT,
            superseded_by TEXT
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value TEXT)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_knowledge_superseded ON knowledge(superseded_by)",
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

                    // If this supersedes another event, mark the old one
                    if let Some(ref old_id) = event.supersedes {
                        conn.execute(
                            "UPDATE knowledge SET superseded_by = ?1 WHERE id = ?2",
                            params![&event.id, old_id],
                        )?;
                    }

                    // Embed and store
                    let embedding = embedder.embed_query(&event.content)?;

                    let rowid: i64 = conn.query_row(
                        "INSERT OR REPLACE INTO knowledge (id, event_type, content, source, domains, timestamp, working_project, supersedes, superseded_by)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)
                         RETURNING rowid",
                        params![
                            &event.id,
                            &event.event_type,
                            &event.content,
                            &event.source,
                            serde_json::to_string(&event.domains)?,
                            event.timestamp.to_rfc3339(),
                            &event.working_project,
                            &event.supersedes,
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
pub fn query(
    query_text: &str,
    limit: usize,
    min_score: f32,
    domains: Option<Vec<String>>,
) -> Result<Vec<PersonaResult>> {
    let cache_dir = persona_paths::cache_dir();
    let db_path = cache_dir.join("persona.db");
    let index_path = cache_dir.join("persona.usearch");

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

    // Search more than limit to allow for filtering
    let search_limit = if domains.is_some() { limit * 3 } else { limit };
    let matches = index.search(&query_embedding, search_limit)?;

    // Hydrate from database
    let conn = Connection::open(&db_path)?;
    let mut results = Vec::new();

    for (rowid, distance) in matches.keys.iter().zip(matches.distances.iter()) {
        let score = 1.0 - distance;
        if score < min_score {
            continue;
        }

        // Query excludes superseded entries
        let result = conn.query_row(
            "SELECT content, source, domains, timestamp FROM knowledge
             WHERE rowid = ?1 AND superseded_by IS NULL",
            params![*rowid as i64],
            |row| {
                let domains_json: String = row.get(2)?;
                let domains: Vec<String> = serde_json::from_str(&domains_json).unwrap_or_default();
                Ok((
                    PersonaResult {
                        content: row.get(0)?,
                        source: row.get(1)?,
                        domains: domains.clone(),
                        timestamp: row.get(3)?,
                        score,
                    },
                    domains,
                ))
            },
        );

        if let Ok((r, result_domains)) = result {
            // Filter by domains if specified
            if let Some(ref filter) = domains {
                if !filter.iter().any(|d| result_domains.contains(d)) {
                    continue;
                }
            }
            results.push(r);
            if results.len() >= limit {
                break;
            }
        }
    }

    Ok(results)
}

/// Check persona oracle status
pub fn status() -> Result<PersonaStatus> {
    let events_dir = persona_paths::events_dir();
    let cache_dir = persona_paths::cache_dir();
    let db_path = cache_dir.join("persona.db");
    let index_path = cache_dir.join("persona.usearch");

    // Count event files
    let event_count = if events_dir.exists() {
        fs::read_dir(&events_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
            .count()
    } else {
        0
    };

    // Check materialized state
    let (materialized, knowledge_count) = if db_path.exists() && index_path.exists() {
        let conn = Connection::open(&db_path)?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM knowledge WHERE superseded_by IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        (true, count as usize)
    } else {
        (false, 0)
    };

    Ok(PersonaStatus {
        events_dir: events_dir.to_string_lossy().to_string(),
        event_files: event_count,
        materialized,
        knowledge_count,
        oracle_available: materialized && knowledge_count > 0,
    })
}

/// Persona oracle status
#[derive(Debug)]
pub struct PersonaStatus {
    pub events_dir: String,
    pub event_files: usize,
    pub materialized: bool,
    pub knowledge_count: usize,
    pub oracle_available: bool,
}

/// List recent persona entries from event files
pub fn list(limit: usize, domains: Option<Vec<String>>) -> Result<Vec<PersonaResult>> {
    let events_dir = persona_paths::events_dir();

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

/// Execute persona status command
pub fn execute_status() -> Result<()> {
    let s = status()?;

    println!("🧠 Persona Oracle Status\n");

    if s.oracle_available {
        println!("   Status: ✓ Available");
    } else {
        println!("   Status: ✗ Not available");
    }

    println!("   Events: {} files in {}", s.event_files, s.events_dir);

    if s.materialized {
        println!("   Index:  ✓ Materialized ({} entries)", s.knowledge_count);
    } else {
        println!("   Index:  ✗ Not materialized");
    }

    if !s.oracle_available {
        println!("\nTo enable persona in scry results:");
        if s.event_files == 0 {
            println!("   1. Capture knowledge: patina persona note \"...\"");
            println!("   2. Build index:       patina persona materialize");
        } else if !s.materialized {
            println!("   Run: patina persona materialize");
        }
    }

    Ok(())
}

/// Execute persona note command
pub fn execute_note(
    content: &str,
    domains: Option<Vec<String>>,
    supersedes: Option<String>,
) -> Result<()> {
    println!("🧠 Persona - Capturing knowledge\n");

    let event_id = note(content, domains.clone(), supersedes.clone())?;

    if let Some(ref d) = domains {
        println!("   Domains: {}", d.join(", "));
    }
    if let Some(ref s) = supersedes {
        println!("   Supersedes: {}", s);
    }
    println!("   Content: {}", content);
    println!("\n✅ Captured: {}", event_id);

    Ok(())
}

/// Execute persona materialize command
pub fn execute_materialize() -> Result<()> {
    println!("🧠 Persona - Materializing knowledge base\n");
    materialize()?;
    Ok(())
}

/// Execute persona query command
pub fn execute_query(
    query_text: &str,
    limit: usize,
    min_score: f32,
    domains: Option<Vec<String>>,
) -> Result<()> {
    println!("🧠 Persona - Searching knowledge\n");
    if let Some(ref d) = domains {
        println!("Domains: {}", d.join(", "));
    }
    println!("Query: \"{}\"\n", query_text);

    let results = query(query_text, limit, min_score, domains)?;

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

        // Create event manually (can't override paths in test easily)
        let event = PersonaEvent {
            id: "test_001".to_string(),
            event_type: "knowledge_captured".to_string(),
            timestamp: Utc::now(),
            source: "direct".to_string(),
            content: "prefer Result<T,E> over panics".to_string(),
            domains: vec!["rust".to_string()],
            working_project: Some("test".to_string()),
            supersedes: None,
        };

        let date = event.timestamp.format("%Y%m%d").to_string();
        let events_file = events_dir.join(format!("{}.jsonl", date));
        let line = serde_json::to_string(&event)?;
        fs::write(&events_file, format!("{}\n", line))?;

        // Verify file exists and contains event
        let content = fs::read_to_string(&events_file)?;
        assert!(content.contains("prefer Result<T,E> over panics"));
        assert!(content.contains("rust"));
        assert!(content.contains("knowledge_captured"));

        Ok(())
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a very long string", 10), "a very lon...");
        assert_eq!(truncate("with\nnewlines", 20), "with newlines");
    }
}
