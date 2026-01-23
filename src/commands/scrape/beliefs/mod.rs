//! Belief scraper - extracts epistemic beliefs from layer/surface/epistemic/beliefs/
//!
//! Uses unified eventlog pattern:
//! - Inserts belief.surface events into eventlog table
//! - Creates materialized views (beliefs) from eventlog

use anyhow::Result;
use regex::Regex;
use rusqlite::Connection;
use serde_json::json;
use std::path::Path;
use std::time::Instant;

use super::database;
use super::ScrapeStats;

const BELIEFS_DIR: &str = "layer/surface/epistemic/beliefs";

/// Parsed belief from markdown file
#[derive(Debug)]
struct ParsedBelief {
    id: String,
    statement: String,       // One-sentence statement after # heading
    persona: String,         // architect, etc.
    facets: Vec<String>,     // Domain tags
    confidence: f64,         // 0.0-1.0
    entrenchment: String,    // low/medium/high/very-high
    status: String,          // active/scoped/defeated/archived
    extracted: Option<String>,
    revised: Option<String>,
    content: String,         // Full markdown for embedding
    file_path: String,
}

/// Create materialized views for belief events
fn create_materialized_views(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Beliefs view (materialized from belief.* events)
        CREATE TABLE IF NOT EXISTS beliefs (
            id TEXT PRIMARY KEY,
            statement TEXT,
            persona TEXT,
            facets TEXT,
            confidence REAL,
            entrenchment TEXT,
            status TEXT,
            extracted TEXT,
            revised TEXT,
            file_path TEXT
        );

        -- FTS5 for belief content search
        CREATE VIRTUAL TABLE IF NOT EXISTS belief_fts USING fts5(
            id,
            statement,
            facets,
            content,
            tokenize='porter unicode61'
        );

        -- Indexes
        CREATE INDEX IF NOT EXISTS idx_beliefs_persona ON beliefs(persona);
        CREATE INDEX IF NOT EXISTS idx_beliefs_status ON beliefs(status);
        CREATE INDEX IF NOT EXISTS idx_beliefs_entrenchment ON beliefs(entrenchment);
        "#,
    )?;

    Ok(())
}

/// Parse a belief markdown file with YAML frontmatter
fn parse_belief_file(path: &Path) -> Result<ParsedBelief> {
    let content = std::fs::read_to_string(path)?;
    let file_path = path.to_string_lossy().to_string();

    // Defaults
    let mut id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut persona = "architect".to_string();
    let mut facets = Vec::new();
    let mut confidence = 0.5;
    let mut entrenchment = "medium".to_string();
    let mut status = "active".to_string();
    let mut extracted = None;
    let mut revised = None;

    // Parse YAML frontmatter if present
    if let Some(after_start) = content.strip_prefix("---") {
        if let Some(end) = after_start.find("---") {
            let frontmatter = &after_start[..end];

            // Extract id
            if let Some(cap) = regex::RegexBuilder::new(r"^id:\s*(.+)$")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                id = cap[1].trim().to_string();
            }

            // Extract persona
            if let Some(cap) = regex::RegexBuilder::new(r"^persona:\s*(.+)$")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                persona = cap[1].trim().to_string();
            }

            // Extract facets (YAML array)
            if let Some(cap) = Regex::new(r"facets:\s*\[([^\]]+)\]")
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                facets = cap[1]
                    .split(',')
                    .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }

            // Extract confidence score (nested YAML)
            if let Some(cap) = regex::RegexBuilder::new(r"^\s+score:\s*([\d.]+)")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                confidence = cap[1].trim().parse().unwrap_or(0.5);
            }

            // Extract entrenchment
            if let Some(cap) = regex::RegexBuilder::new(r"^entrenchment:\s*(.+)$")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                entrenchment = cap[1].trim().to_string();
            }

            // Extract status
            if let Some(cap) = regex::RegexBuilder::new(r"^status:\s*(.+)$")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                status = cap[1].trim().to_string();
            }

            // Extract extracted date
            if let Some(cap) = regex::RegexBuilder::new(r"^extracted:\s*(.+)$")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                extracted = Some(cap[1].trim().to_string());
            }

            // Extract revised date
            if let Some(cap) = regex::RegexBuilder::new(r"^revised:\s*(.+)$")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                revised = Some(cap[1].trim().to_string());
            }
        }
    }

    // Extract one-sentence statement (line after # id heading)
    let statement = extract_statement(&content, &id);

    Ok(ParsedBelief {
        id,
        statement,
        persona,
        facets,
        confidence,
        entrenchment,
        status,
        extracted,
        revised,
        content,
        file_path,
    })
}

/// Extract the one-sentence statement after the # heading
fn extract_statement(content: &str, id: &str) -> String {
    let heading_pattern = format!(r"^#\s+{}\s*$", regex::escape(id));
    let heading_re = Regex::new(&heading_pattern).ok();

    let mut found_heading = false;
    for line in content.lines() {
        if found_heading {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        if let Some(ref re) = heading_re {
            if re.is_match(line) {
                found_heading = true;
            }
        }
    }

    // Fallback: use id as statement
    id.replace('-', " ")
}

/// Insert a parsed belief into eventlog and materialized views
fn insert_belief(conn: &Connection, belief: &ParsedBelief) -> Result<()> {
    let event_type = "belief.surface";
    let timestamp = belief
        .revised
        .as_deref()
        .or(belief.extracted.as_deref())
        .unwrap_or("2026-01-01");

    // 1. Delete existing data for this belief (for re-scrapes)
    conn.execute("DELETE FROM beliefs WHERE id = ?1", [&belief.id])?;
    conn.execute("DELETE FROM belief_fts WHERE id = ?1", [&belief.id])?;
    conn.execute(
        "DELETE FROM eventlog WHERE source_id = ?1 AND event_type = 'belief.surface'",
        [&belief.id],
    )?;

    // 2. Insert into eventlog
    let event_data = json!({
        "statement": &belief.statement,
        "persona": &belief.persona,
        "facets": &belief.facets,
        "confidence": belief.confidence,
        "entrenchment": &belief.entrenchment,
        "status": &belief.status,
        "content": &belief.content,
    });

    database::insert_event(
        conn,
        event_type,
        timestamp,
        &belief.id,
        Some(&belief.file_path),
        &event_data.to_string(),
    )?;

    // 3. Insert materialized view
    let facets_str = belief.facets.join(", ");

    conn.execute(
        "INSERT INTO beliefs (id, statement, persona, facets, confidence, entrenchment, status, extracted, revised, file_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            &belief.id,
            &belief.statement,
            &belief.persona,
            &facets_str,
            belief.confidence,
            &belief.entrenchment,
            &belief.status,
            &belief.extracted,
            &belief.revised,
            &belief.file_path,
        ],
    )?;

    // 4. Insert into FTS5 for lexical search
    conn.execute(
        "INSERT INTO belief_fts (id, statement, facets, content)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![&belief.id, &belief.statement, &facets_str, &belief.content,],
    )?;

    Ok(())
}

/// Main entry point for belief scraping
pub fn run(full: bool) -> Result<ScrapeStats> {
    let start = Instant::now();
    let db_path = Path::new(database::PATINA_DB);
    let beliefs_path = Path::new(BELIEFS_DIR);

    // Check if beliefs directory exists
    if !beliefs_path.exists() {
        println!("  No beliefs directory found ({})", BELIEFS_DIR);
        return Ok(ScrapeStats {
            items_processed: 0,
            time_elapsed: start.elapsed(),
            database_size_kb: 0,
        });
    }

    // Initialize unified database with eventlog
    let conn = database::initialize(db_path)?;

    // Create materialized views for belief events
    create_materialized_views(&conn)?;

    // Get list of already processed beliefs for incremental
    let processed: std::collections::HashSet<String> = if full {
        std::collections::HashSet::new()
    } else {
        let mut stmt = conn.prepare("SELECT id FROM beliefs")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.filter_map(|r| r.ok()).collect()
    };

    if full {
        println!("  Full belief scrape...");
    } else {
        println!(
            "  Incremental belief scrape ({} already processed)...",
            processed.len()
        );
    }

    // Collect belief files
    let mut belief_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(beliefs_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map(|ext| ext == "md").unwrap_or(false) {
                belief_files.push(path);
            }
        }
    }
    belief_files.sort();

    let mut processed_count = 0;
    let mut skipped = 0;

    for path in &belief_files {
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

        match parse_belief_file(path) {
            Ok(belief) => {
                if let Err(e) = insert_belief(&conn, &belief) {
                    eprintln!("  Warning: failed to insert belief {}: {}", id, e);
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
        "  Processed {} beliefs ({} skipped)",
        processed_count, skipped
    );

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
    fn test_extract_statement() {
        let content = r#"---
type: belief
id: test-belief
---

# test-belief

This is the one-sentence statement.

## Statement

Expanded explanation here.
"#;
        let statement = extract_statement(content, "test-belief");
        assert_eq!(statement, "This is the one-sentence statement.");
    }

    #[test]
    fn test_parse_belief_frontmatter() {
        let content = r#"---
type: belief
id: sync-first
persona: architect
facets: [rust, architecture]
confidence:
  score: 0.88
entrenchment: high
status: active
extracted: 2025-08-04
revised: 2026-01-16
---

# sync-first

Prefer synchronous code.
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("sync-first.md");
        std::fs::write(&file_path, content).unwrap();

        let belief = parse_belief_file(&file_path).unwrap();
        assert_eq!(belief.id, "sync-first");
        assert_eq!(belief.persona, "architect");
        assert_eq!(belief.facets, vec!["rust", "architecture"]);
        assert!((belief.confidence - 0.88).abs() < 0.01);
        assert_eq!(belief.entrenchment, "high");
        assert_eq!(belief.status, "active");
        assert_eq!(belief.statement, "Prefer synchronous code.");
    }
}
