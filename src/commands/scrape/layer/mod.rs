//! Unified layer scraper — one command owns all of layer/
//!
//! Routes layer/**/*.md to the appropriate sub-scraper based on path:
//! - layer/sessions/*.md        → SessionScraper (sessions.rs)
//! - layer/core/*.md            → PatternScraper (this file)
//! - layer/surface/*.md         → PatternScraper (this file)
//! - layer/dust/*.md            → skipped (gitignored, local-only content)
//!
//! Beliefs (layer/surface/epistemic/beliefs/) are handled by the separate
//! beliefs scraper — they have their own embedding pipeline.
//!
//! File collection uses `ignore::WalkBuilder` to respect .gitignore,
//! matching the pattern in the code scraper (extract_v2.rs).

pub mod sessions;

use anyhow::Result;
use regex::Regex;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use std::time::Instant;

use super::database;
use super::ScrapeStats;

const LAYER_DIR: &str = "layer";

/// Content types within the layer directory
#[derive(Debug, PartialEq)]
enum LayerContent {
    Pattern,
    Session,
    Belief, // handled separately — skip in layer router
}

/// Classify a layer file path to its content type
fn classify_layer_path(path: &Path) -> LayerContent {
    let path_str = path.to_string_lossy();

    if path_str.contains("sessions/") {
        LayerContent::Session
    } else if path_str.contains("epistemic/beliefs/") {
        LayerContent::Belief
    } else {
        LayerContent::Pattern
    }
}

/// Milestone from spec frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub version: String,
    pub name: String,
    pub status: String, // pending, in_progress, complete
}

/// Parsed pattern from markdown file
#[derive(Debug)]
struct ParsedPattern {
    id: String,
    title: String,
    layer: String,          // core, surface
    status: Option<String>, // active, draft, archived, ready, complete
    created: Option<String>,
    tags: Vec<String>,
    references: Vec<String>,
    purpose: Option<String>, // First **Purpose:** line
    content: String,         // Full markdown content (for embedding)
    file_path: String,
    milestones: Vec<Milestone>,        // Version-linked milestones
    current_milestone: Option<String>, // Current milestone version
    // Spec dependency fields (for spec-as-work-item)
    blocked_by: Vec<String>, // Spec IDs this depends on
    blocks: Vec<String>,     // Spec IDs that depend on this
    target: Option<String>,  // Version target (e.g., "v0.12.0")
}

/// Create materialized views for pattern events
fn create_materialized_views(conn: &Connection) -> Result<()> {
    // First, create base tables
    conn.execute_batch(
        r#"
        -- Patterns view (materialized from pattern.* events)
        CREATE TABLE IF NOT EXISTS patterns (
            id TEXT PRIMARY KEY,
            title TEXT,
            layer TEXT,
            status TEXT,
            created TEXT,
            tags TEXT,
            refs TEXT,
            purpose TEXT,
            file_path TEXT
        );"#,
    )?;

    // Migration: add current_milestone column if it doesn't exist
    let has_milestone_col: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('patterns') WHERE name = 'current_milestone'")?
        .exists([])?;

    if !has_milestone_col {
        conn.execute("ALTER TABLE patterns ADD COLUMN current_milestone TEXT", [])?;
    }

    // Migration: add target column if it doesn't exist (spec-as-work-item)
    let has_target_col: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('patterns') WHERE name = 'target'")?
        .exists([])?;

    if !has_target_col {
        conn.execute("ALTER TABLE patterns ADD COLUMN target TEXT", [])?;
    }

    // Continue with rest of schema
    conn.execute_batch(
        r#"

        -- FTS5 for pattern content search
        CREATE VIRTUAL TABLE IF NOT EXISTS pattern_fts USING fts5(
            id,
            title,
            purpose,
            content,
            tags,
            file_path,
            tokenize='porter unicode61'
        );

        -- Milestones table (version-linked spec outcomes)
        CREATE TABLE IF NOT EXISTS milestones (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            spec_id TEXT NOT NULL,
            version TEXT NOT NULL,
            name TEXT NOT NULL,
            status TEXT NOT NULL,
            UNIQUE(spec_id, version)
        );

        -- Spec dependencies table (spec-as-work-item)
        -- Stores blocked_by relationships for ready queue calculation
        CREATE TABLE IF NOT EXISTS spec_deps (
            spec_id TEXT NOT NULL,
            depends_on TEXT NOT NULL,
            UNIQUE(spec_id, depends_on)
        );

        -- Indexes
        CREATE INDEX IF NOT EXISTS idx_patterns_layer ON patterns(layer);
        CREATE INDEX IF NOT EXISTS idx_patterns_status ON patterns(status);
        CREATE INDEX IF NOT EXISTS idx_patterns_target ON patterns(target);
        CREATE INDEX IF NOT EXISTS idx_milestones_spec ON milestones(spec_id);
        CREATE INDEX IF NOT EXISTS idx_milestones_status ON milestones(status);
        CREATE INDEX IF NOT EXISTS idx_milestones_version ON milestones(version);
        CREATE INDEX IF NOT EXISTS idx_spec_deps_spec ON spec_deps(spec_id);
        CREATE INDEX IF NOT EXISTS idx_spec_deps_depends ON spec_deps(depends_on);
        "#,
    )?;

    Ok(())
}

/// Helper struct for parsing frontmatter with serde_yaml
#[derive(Debug, Deserialize, Default)]
struct Frontmatter {
    #[serde(default)]
    milestones: Vec<Milestone>,
    #[serde(default)]
    current_milestone: Option<String>,
    // Spec dependency fields (for spec-as-work-item)
    #[serde(default)]
    blocked_by: Vec<String>,
    #[serde(default)]
    blocks: Vec<String>,
    #[serde(default)]
    target: Option<String>,
}

/// Parse complex frontmatter fields using serde_yaml
/// Returns Frontmatter with milestones, blocked_by, blocks, target
fn parse_frontmatter_yaml(content: &str) -> Frontmatter {
    // Extract frontmatter between --- markers
    if let Some(after_start) = content.strip_prefix("---") {
        if let Some(end) = after_start.find("---") {
            let frontmatter_str = &after_start[..end];

            // Try to parse with serde_yaml
            if let Ok(fm) = serde_yaml::from_str::<Frontmatter>(frontmatter_str) {
                return fm;
            }
        }
    }
    Frontmatter::default()
}

/// Parse a pattern markdown file with YAML frontmatter
fn parse_pattern_file(path: &Path) -> Result<ParsedPattern> {
    let content = std::fs::read_to_string(path)?;
    let file_path = path.to_string_lossy().to_string();

    // Extract frontmatter between --- markers
    let mut id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut layer = "surface".to_string();
    let mut status = None;
    let mut created = None;
    let mut tags = Vec::new();
    let mut references = Vec::new();

    // Parse YAML frontmatter if present
    if let Some(after_start) = content.strip_prefix("---") {
        if let Some(end) = after_start.find("---") {
            let frontmatter = &after_start[..end];

            // Extract id (multiline mode for ^ to match line start)
            if let Some(cap) = regex::RegexBuilder::new(r"^id:\s*(.+)$")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                id = cap[1].trim().to_string();
            }

            // Extract layer
            if let Some(cap) = regex::RegexBuilder::new(r"^layer:\s*(.+)$")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                layer = cap[1].trim().to_string();
            }

            // Extract status
            if let Some(cap) = regex::RegexBuilder::new(r"^status:\s*(.+)$")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                status = Some(cap[1].trim().to_string());
            }

            // Extract created
            if let Some(cap) = regex::RegexBuilder::new(r"^created:\s*(.+)$")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                created = Some(cap[1].trim().to_string());
            }

            // Extract tags (YAML array)
            if let Some(cap) = Regex::new(r"tags:\s*\[([^\]]+)\]")
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                tags = cap[1]
                    .split(',')
                    .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }

            // Extract references (YAML array)
            if let Some(cap) = Regex::new(r"references:\s*\[([^\]]+)\]")
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                references = cap[1]
                    .split(',')
                    .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }

    // Parse complex frontmatter fields using serde_yaml
    let frontmatter_yaml = parse_frontmatter_yaml(&content);
    let milestones = frontmatter_yaml.milestones;
    let current_milestone = frontmatter_yaml.current_milestone;
    let blocked_by = frontmatter_yaml.blocked_by;
    let blocks = frontmatter_yaml.blocks;
    let target = frontmatter_yaml.target;

    // Extract title from first # heading
    let title_re = Regex::new(r"^# (.+)$").unwrap();
    let title = content
        .lines()
        .find_map(|line| title_re.captures(line).map(|c| c[1].to_string()))
        .unwrap_or_else(|| id.clone());

    // Extract purpose from **Purpose:** line
    let purpose_re = Regex::new(r"\*\*Purpose:\*\*\s*(.+)").unwrap();
    let purpose = content
        .lines()
        .find_map(|line| purpose_re.captures(line).map(|c| c[1].trim().to_string()));

    Ok(ParsedPattern {
        id,
        title,
        layer,
        status,
        created,
        tags,
        references,
        purpose,
        content,
        file_path,
        milestones,
        current_milestone,
        blocked_by,
        blocks,
        target,
    })
}

/// Insert a parsed pattern into eventlog and materialized views
fn insert_pattern(conn: &Connection, pattern: &ParsedPattern) -> Result<()> {
    let event_type = format!("pattern.{}", pattern.layer);
    let timestamp = pattern.created.as_deref().unwrap_or("2025-01-01");

    // 1. Delete existing data for this pattern (for re-scrapes)
    conn.execute("DELETE FROM patterns WHERE id = ?1", [&pattern.id])?;
    conn.execute("DELETE FROM pattern_fts WHERE id = ?1", [&pattern.id])?;
    conn.execute("DELETE FROM milestones WHERE spec_id = ?1", [&pattern.id])?;
    conn.execute("DELETE FROM spec_deps WHERE spec_id = ?1", [&pattern.id])?;
    // Delete from eventlog too
    conn.execute(
        "DELETE FROM eventlog WHERE source_id = ?1 AND event_type LIKE 'pattern.%'",
        [&pattern.id],
    )?;

    // 2. Insert into eventlog
    let event_data = json!({
        "title": &pattern.title,
        "layer": &pattern.layer,
        "status": &pattern.status,
        "created": &pattern.created,
        "tags": &pattern.tags,
        "references": &pattern.references,
        "purpose": &pattern.purpose,
        "content": &pattern.content,
        "milestones": &pattern.milestones,
        "current_milestone": &pattern.current_milestone,
        "blocked_by": &pattern.blocked_by,
        "blocks": &pattern.blocks,
        "target": &pattern.target,
    });

    database::insert_event(
        conn,
        &event_type,
        timestamp,
        &pattern.id,
        Some(&pattern.file_path),
        &event_data.to_string(),
    )?;

    // 3. Insert materialized view
    let tags_str = pattern.tags.join(", ");
    let refs_str = pattern.references.join(", ");

    conn.execute(
        "INSERT INTO patterns (id, title, layer, status, created, tags, refs, purpose, file_path, current_milestone, target)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![
            &pattern.id,
            &pattern.title,
            &pattern.layer,
            &pattern.status,
            &pattern.created,
            &tags_str,
            &refs_str,
            &pattern.purpose,
            &pattern.file_path,
            &pattern.current_milestone,
            &pattern.target,
        ],
    )?;

    // 4. Insert into FTS5 for lexical search
    conn.execute(
        "INSERT INTO pattern_fts (id, title, purpose, content, tags, file_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            &pattern.id,
            &pattern.title,
            &pattern.purpose,
            &pattern.content,
            &tags_str,
            &pattern.file_path,
        ],
    )?;

    // 5. Insert milestones (if any)
    for milestone in &pattern.milestones {
        conn.execute(
            "INSERT OR REPLACE INTO milestones (spec_id, version, name, status)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                &pattern.id,
                &milestone.version,
                &milestone.name,
                &milestone.status,
            ],
        )?;
    }

    // 6. Insert spec dependencies (blocked_by relationships)
    for dep in &pattern.blocked_by {
        conn.execute(
            "INSERT OR IGNORE INTO spec_deps (spec_id, depends_on) VALUES (?1, ?2)",
            rusqlite::params![&pattern.id, dep],
        )?;
    }

    Ok(())
}

/// Collect markdown files from a directory, respecting .gitignore.
///
/// Uses `ignore::WalkBuilder` (same as code scraper in extract_v2.rs)
/// to skip gitignored paths like `layer/dust/`.
fn collect_md_files(dir: &Path, recursive: bool) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();

    if !dir.exists() {
        return files;
    }

    if recursive {
        for entry in ignore::WalkBuilder::new(dir)
            .hidden(false)
            .git_ignore(true)
            .max_depth(None)
            .build()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().map(|ext| ext == "md").unwrap_or(false) {
                files.push(path.to_path_buf());
            }
        }
    } else {
        // Just immediate directory
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map(|ext| ext == "md").unwrap_or(false) {
                    files.push(path);
                }
            }
        }
    }

    files.sort();
    files
}

/// Unified entry point: scrape all of layer/ (patterns + sessions)
///
/// Walks layer/**/*.md, classifies each file, and routes to the
/// appropriate sub-scraper. Beliefs are skipped (separate pipeline).
pub fn run(full: bool) -> Result<ScrapeStats> {
    let start = Instant::now();
    let db_path = Path::new(database::PATINA_DB);
    let layer_dir = Path::new(LAYER_DIR);

    if !layer_dir.exists() {
        anyhow::bail!("Layer directory not found: {}", LAYER_DIR);
    }

    // Initialize unified database with eventlog
    let conn = database::initialize(db_path)?;

    // Collect all markdown files from layer/ and classify
    let all_files = collect_md_files(layer_dir, true);

    let mut pattern_files = Vec::new();
    let mut session_files = Vec::new();

    for path in all_files {
        match classify_layer_path(&path) {
            LayerContent::Pattern => pattern_files.push(path),
            LayerContent::Session => session_files.push(path),
            LayerContent::Belief => {} // handled by beliefs scraper
        }
    }

    // --- Patterns ---
    let pattern_count = scrape_patterns(&conn, &pattern_files, full)?;

    // --- Sessions ---
    let (session_count, session_skipped) = sessions::scrape_sessions(&conn, &session_files, full)?;

    println!("  {} sessions ({} skipped)", session_count, session_skipped);

    let total = pattern_count + session_count;

    let elapsed = start.elapsed();
    let db_size = std::fs::metadata(db_path)
        .map(|m| m.len() / 1024)
        .unwrap_or(0);

    // Emit measurement: layer scrape capture metrics
    patina::measure::emit_or_warn(
        "capture",
        "scrape",
        "layer",
        &serde_json::json!({
            "patterns_processed": pattern_count,
            "sessions_processed": session_count,
            "duration_ms": elapsed.as_millis() as u64,
        }),
    );

    Ok(ScrapeStats {
        items_processed: total,
        time_elapsed: elapsed,
        database_size_kb: db_size,
    })
}

/// Scrape pattern files (core, surface, dust). Returns count of processed patterns.
fn scrape_patterns(
    conn: &Connection,
    pattern_files: &[std::path::PathBuf],
    full: bool,
) -> Result<usize> {
    // Create materialized views for pattern events
    create_materialized_views(conn)?;

    // Get list of already processed patterns for incremental
    let processed: std::collections::HashSet<String> = if full {
        std::collections::HashSet::new()
    } else {
        let mut stmt = conn.prepare("SELECT id FROM patterns")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.filter_map(|r| r.ok()).collect()
    };

    if full {
        println!("📜 Full layer scrape...");
    } else {
        println!(
            "📜 Incremental layer scrape ({} already processed)...",
            processed.len()
        );
    }

    let mut processed_count = 0;
    let mut skipped = 0;
    let mut current_file_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for path in pattern_files {
        match parse_pattern_file(path) {
            Ok(pattern) => {
                // Track the frontmatter ID (not file stem) for pruning
                current_file_ids.insert(pattern.id.clone());

                // Skip if already processed (incremental mode)
                if !full && processed.contains(&pattern.id) {
                    skipped += 1;
                    continue;
                }

                if let Err(e) = insert_pattern(conn, &pattern) {
                    eprintln!("  Warning: failed to insert {}: {}", pattern.id, e);
                } else {
                    processed_count += 1;
                }
            }
            Err(e) => {
                eprintln!("  Warning: failed to parse {}: {}", path.display(), e);
            }
        }
    }

    println!("  {} patterns ({} skipped)", processed_count, skipped);

    // Prune stale entries: delete DB entries for IDs that no longer exist on disk
    let file_ids = current_file_ids;

    let mut stmt = conn.prepare("SELECT id FROM patterns")?;
    let db_ids: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    let mut pruned = 0;
    for db_id in &db_ids {
        if !file_ids.contains(db_id) {
            // Delete from all related tables
            conn.execute("DELETE FROM patterns WHERE id = ?1", [db_id])?;
            conn.execute("DELETE FROM pattern_fts WHERE id = ?1", [db_id])?;
            conn.execute("DELETE FROM milestones WHERE spec_id = ?1", [db_id])?;
            conn.execute("DELETE FROM spec_deps WHERE spec_id = ?1", [db_id])?;
            conn.execute(
                "DELETE FROM eventlog WHERE source_id = ?1 AND event_type LIKE 'pattern.%'",
                [db_id],
            )?;
            pruned += 1;
        }
    }

    if pruned > 0 {
        println!("  Pruned {} stale entries", pruned);
    }

    Ok(processed_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
id: test-pattern
layer: core
status: active
created: 2025-01-01
tags: [rust, testing]
references: [other-pattern]
---

# Test Pattern

**Purpose:** A test pattern for unit testing.

## Content

Some content here.
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test-pattern.md");
        std::fs::write(&file_path, content).unwrap();

        let pattern = parse_pattern_file(&file_path).unwrap();
        assert_eq!(pattern.id, "test-pattern");
        assert_eq!(pattern.layer, "core");
        assert_eq!(pattern.status, Some("active".to_string()));
        assert_eq!(pattern.title, "Test Pattern");
        assert_eq!(
            pattern.purpose,
            Some("A test pattern for unit testing.".to_string())
        );
        assert_eq!(pattern.tags, vec!["rust", "testing"]);
        assert_eq!(pattern.references, vec!["other-pattern"]);
    }

    #[test]
    fn test_classify_layer_path() {
        use std::path::PathBuf;

        assert_eq!(
            classify_layer_path(&PathBuf::from("layer/sessions/20260205.md")),
            LayerContent::Session
        );
        assert_eq!(
            classify_layer_path(&PathBuf::from("layer/core/unix-philosophy.md")),
            LayerContent::Pattern
        );
        assert_eq!(
            classify_layer_path(&PathBuf::from("layer/surface/build/feat/foo/SPEC.md")),
            LayerContent::Pattern
        );
        assert_eq!(
            classify_layer_path(&PathBuf::from(
                "layer/surface/epistemic/beliefs/my-belief.md"
            )),
            LayerContent::Belief
        );
        assert_eq!(
            classify_layer_path(&PathBuf::from("layer/dust/old-pattern.md")),
            LayerContent::Pattern
        );
    }

    #[test]
    fn test_parse_spec_dependencies() {
        let content = r#"---
type: feat
id: cli-reorganization
status: ready
target: v0.12.0
blocked_by:
  - system-introspection
  - scrape-layer-unify
blocks:
  - science-commands
related:
  - mother-v2
---

# feat: CLI Reorganization

Some spec content.
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("cli-reorganization.md");
        std::fs::write(&file_path, content).unwrap();

        let pattern = parse_pattern_file(&file_path).unwrap();
        assert_eq!(pattern.id, "cli-reorganization");
        assert_eq!(pattern.status, Some("ready".to_string()));
        assert_eq!(pattern.target, Some("v0.12.0".to_string()));
        assert_eq!(
            pattern.blocked_by,
            vec!["system-introspection", "scrape-layer-unify"]
        );
        assert_eq!(pattern.blocks, vec!["science-commands"]);
    }
}
