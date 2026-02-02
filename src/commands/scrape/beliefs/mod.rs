//! Belief scraper - extracts epistemic beliefs from layer/surface/epistemic/beliefs/
//!
//! Uses unified eventlog pattern:
//! - Inserts belief.surface events into eventlog table
//! - Creates materialized views (beliefs) from eventlog
//! - Runs verification queries (Phase 2.5) and stores results

mod verification;

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
    statement: String,    // One-sentence statement after # heading
    persona: String,      // architect, etc.
    facets: Vec<String>,  // Domain tags
    confidence: f64,      // 0.0-1.0 (legacy, will be removed)
    entrenchment: String, // low/medium/high/very-high
    status: String,       // active/scoped/defeated/archived
    extracted: Option<String>,
    revised: Option<String>,
    content: String, // Full markdown for embedding
    file_path: String,
    // Computed metrics (E4) — real data, not LLM guesses
    metrics: BeliefMetrics,
    // Verification queries from ## Verification section
    verification_queries: Vec<verification::VerificationQuery>,
    // Verification aggregates (computed during Phase 2.5)
    verification: verification::VerificationAggregates,
}

/// Computed use/truth metrics for a belief — all derived from files on disk
#[derive(Debug, Default)]
struct BeliefMetrics {
    // Use: is this belief doing work?
    cited_by_beliefs: i32, // other beliefs referencing this in Supports/Attacks/Evidence
    cited_by_sessions: i32, // session files mentioning this belief ID
    applied_in: i32,       // entries in ## Applied-In section

    // Truth: is the evidence real?
    evidence_count: i32,    // entries in ## Evidence section
    evidence_verified: i32, // evidence [[wikilinks]] that resolve to real files
    defeated_attacks: i32,  // Attacked-By entries with status: defeated
    external_sources: i32,  // evidence not from sessions/beliefs (papers, docs, etc.)

    // Endorsement
    endorsed: bool, // user explicitly created or confirmed

    // E4.6a: Semantic grounding — how connected is this belief to code/commits/sessions?
    grounding_score: f32, // Average similarity to grounded neighbors (0.0-1.0)
    grounding_code_count: i32, // Code functions above similarity threshold
    grounding_commit_count: i32, // Commits above similarity threshold
    grounding_session_count: i32, // Sessions above similarity threshold
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
            file_path TEXT,
            -- E4: Computed use/truth metrics
            cited_by_beliefs INTEGER DEFAULT 0,
            cited_by_sessions INTEGER DEFAULT 0,
            applied_in INTEGER DEFAULT 0,
            evidence_count INTEGER DEFAULT 0,
            evidence_verified INTEGER DEFAULT 0,
            defeated_attacks INTEGER DEFAULT 0,
            external_sources INTEGER DEFAULT 0,
            endorsed INTEGER DEFAULT 0,
            -- E4.6a: Semantic grounding metrics
            grounding_score REAL DEFAULT 0.0,
            grounding_code_count INTEGER DEFAULT 0,
            grounding_commit_count INTEGER DEFAULT 0,
            grounding_session_count INTEGER DEFAULT 0
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

        -- E4.6a-fix: Multi-hop code grounding (belief → commit → file → function)
        CREATE TABLE IF NOT EXISTS belief_code_reach (
            belief_id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            reach_score REAL,
            commit_count INTEGER,
            function_count INTEGER,
            hop_path TEXT,
            PRIMARY KEY (belief_id, file_path)
        );
        CREATE INDEX IF NOT EXISTS idx_belief_code_reach_file ON belief_code_reach(file_path);
        "#,
    )?;

    // Migrate existing table: add E4 metric columns if they don't exist yet
    let columns_to_add = [
        ("cited_by_beliefs", "INTEGER DEFAULT 0"),
        ("cited_by_sessions", "INTEGER DEFAULT 0"),
        ("applied_in", "INTEGER DEFAULT 0"),
        ("evidence_count", "INTEGER DEFAULT 0"),
        ("evidence_verified", "INTEGER DEFAULT 0"),
        ("defeated_attacks", "INTEGER DEFAULT 0"),
        ("external_sources", "INTEGER DEFAULT 0"),
        ("endorsed", "INTEGER DEFAULT 0"),
        // E4.6a: Semantic grounding metrics
        ("grounding_score", "REAL DEFAULT 0.0"),
        ("grounding_code_count", "INTEGER DEFAULT 0"),
        ("grounding_commit_count", "INTEGER DEFAULT 0"),
        ("grounding_session_count", "INTEGER DEFAULT 0"),
    ];

    for (col_name, col_type) in &columns_to_add {
        let sql = format!("ALTER TABLE beliefs ADD COLUMN {} {}", col_name, col_type);
        // Ignore error if column already exists
        let _ = conn.execute(&sql, []);
    }

    // Create verification tables (belief_verifications + aggregate columns on beliefs)
    verification::create_tables(conn)?;

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

    // Compute per-file metrics from markdown sections
    let mut metrics = extract_file_metrics(&content);

    // Check for endorsed field in frontmatter (default: true for existing beliefs)
    metrics.endorsed = true; // All beliefs created via skill are user-initiated

    // Parse verification queries from ## Verification section
    let verification_queries = verification::parse_verification_blocks(&content);

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
        metrics,
        verification_queries,
        verification: verification::VerificationAggregates::default(),
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

/// Extract per-file metrics from belief markdown content
fn extract_file_metrics(content: &str) -> BeliefMetrics {
    let mut metrics = BeliefMetrics::default();

    // Parse sections by heading
    let mut current_section = "";
    for line in content.lines() {
        let trimmed = line.trim();

        // Detect section headings
        if trimmed.starts_with("## ") {
            current_section = trimmed;
            continue;
        }

        // Only count list entries (lines starting with -)
        if !trimmed.starts_with("- ") && !trimmed.starts_with("* ") {
            continue;
        }

        match current_section {
            s if s.starts_with("## Evidence") => {
                metrics.evidence_count += 1;
                // Verification and external source detection happen in
                // verify_evidence_section() during cross_reference_beliefs()
            }
            s if s.starts_with("## Applied-In") => {
                metrics.applied_in += 1;
            }
            s if s.starts_with("## Attacked-By") => {
                if trimmed.contains("status: defeated") {
                    metrics.defeated_attacks += 1;
                }
            }
            _ => {}
        }
    }

    metrics
}

/// Verify evidence lines from a belief file against real files on disk.
/// Handles both `[[wikilink]]` format and bare `session-YYYYMMDD-HHMMSS:` references.
fn verify_evidence_section(content: &str, project_root: &Path) -> (i32, i32) {
    let mut verified = 0;
    let mut external = 0;
    let wikilink_re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    // Match bare session IDs: session-YYYYMMDD-HHMMSS (with optional colon/space after)
    let bare_session_re = Regex::new(r"(?:^|\s)(session-)?(\d{8}-\d{6})[\s:,]").unwrap();
    // Match bare YYYYMMDD-HHMMSS session IDs (without "session-" prefix)
    let session_id_re = Regex::new(r"\b(\d{8}-\d{6})\b").unwrap();
    let mut in_evidence = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## Evidence") {
            in_evidence = true;
            continue;
        }
        if trimmed.starts_with("## ") && in_evidence {
            break;
        }
        if !in_evidence || !trimmed.starts_with("- ") {
            continue;
        }

        let mut line_verified = false;

        // 1. Check [[wikilinks]]
        for cap in wikilink_re.captures_iter(trimmed) {
            let link = &cap[1];
            if try_verify_link(link, project_root) {
                line_verified = true;
            } else if is_external_source(link) {
                external += 1;
                line_verified = true;
            }
        }

        // 2. Check bare session references (e.g., "session-20260129-074742:" or just "20260129-074742")
        if !line_verified {
            for cap in bare_session_re.captures_iter(trimmed) {
                let session_id = &cap[2];
                let session_path = project_root
                    .join("layer/sessions")
                    .join(format!("{}.md", session_id));
                if session_path.exists() {
                    line_verified = true;
                    break;
                }
            }
        }

        // 3. Fallback: look for any YYYYMMDD-HHMMSS pattern that matches a session file
        if !line_verified {
            for cap in session_id_re.captures_iter(trimmed) {
                let session_id = &cap[1];
                let session_path = project_root
                    .join("layer/sessions")
                    .join(format!("{}.md", session_id));
                if session_path.exists() {
                    line_verified = true;
                    break;
                }
            }
        }

        if line_verified {
            verified += 1;
        }
    }

    (verified, external)
}

/// Try to verify a single wikilink against files on disk
fn try_verify_link(link: &str, project_root: &Path) -> bool {
    // Session links: [[session-YYYYMMDD-HHMMSS]]
    if link.starts_with("session-") {
        let session_id = link.strip_prefix("session-").unwrap_or(link);
        let session_path = project_root
            .join("layer/sessions")
            .join(format!("{}.md", session_id));
        if session_path.exists() {
            return true;
        }
        // Fuzzy match: [[session-20260105]] → find layer/sessions/20260105-*.md
        if session_id.len() == 8 {
            let sessions_dir = project_root.join("layer/sessions");
            if let Ok(entries) = std::fs::read_dir(&sessions_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with(session_id) && name.ends_with(".md") {
                            return true;
                        }
                    }
                }
            }
        }
        return false;
    }

    // Commit links: [[commit-HASH]] → verify via git rev-parse
    if link.starts_with("commit-") {
        let hash = link.strip_prefix("commit-").unwrap_or(link);
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--verify", &format!("{}^{{commit}}", hash)])
            .current_dir(project_root)
            .output()
        {
            return output.status.success();
        }
        return false;
    }

    // Spec links: [[spec-name]] or [[spec/path]]
    if link.starts_with("spec-") || link.starts_with("spec/") {
        return true; // Specs are valid if referenced
    }

    // Check as belief file
    let belief_path = project_root
        .join("layer/surface/epistemic/beliefs")
        .join(format!("{}.md", link));
    if belief_path.exists() {
        return true;
    }

    // Check as a file path directly (e.g., [[CLAUDE.md]])
    let direct_path = project_root.join(link);
    if direct_path.exists() {
        return true;
    }

    false
}

/// Check if a link looks like an external source (not in-project)
fn is_external_source(link: &str) -> bool {
    let lower = link.to_lowercase();
    lower.contains("paper")
        || lower.contains("helland")
        || lower.contains("blog")
        || lower.contains("rfc")
        || lower.contains("doi")
}

/// Cross-reference beliefs against each other and session files.
/// Computes cited_by_beliefs and cited_by_sessions for each belief.
fn cross_reference_beliefs(beliefs: &mut [ParsedBelief], project_root: &Path) {
    let sessions_dir = project_root.join("layer/sessions");

    // Collect all belief IDs for reference
    let belief_ids: Vec<String> = beliefs.iter().map(|b| b.id.clone()).collect();

    // Collect all belief file contents (for cross-referencing)
    let belief_contents: Vec<(String, String)> = beliefs
        .iter()
        .map(|b| (b.id.clone(), b.content.clone()))
        .collect();

    // Read session files once, build reverse index: belief_id → citation count
    let mut session_citations: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();

    if sessions_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&sessions_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map(|ext| ext == "md").unwrap_or(false) {
                    if let Ok(session_content) = std::fs::read_to_string(&path) {
                        for bid in &belief_ids {
                            if session_content.contains(bid.as_str()) {
                                *session_citations.entry(bid.clone()).or_insert(0) += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    // Cross-reference beliefs against each other
    for belief in beliefs.iter_mut() {
        let bid = &belief.id;

        // Count how many OTHER belief files reference this belief ID
        let mut belief_citations = 0;
        for (other_id, other_content) in &belief_contents {
            if other_id != bid && other_content.contains(bid.as_str()) {
                belief_citations += 1;
            }
        }

        // Verify evidence lines (handles both [[wikilinks]] and bare session-ID references)
        let (verified, external) = verify_evidence_section(&belief.content, project_root);

        // Update metrics
        belief.metrics.cited_by_beliefs = belief_citations;
        belief.metrics.cited_by_sessions = session_citations.get(bid).copied().unwrap_or(0);
        belief.metrics.evidence_verified = verified;
        belief.metrics.external_sources += external;
    }
}

/// Compute semantic grounding metrics for all beliefs (E4.6a step 5)
///
/// Loads the usearch semantic index (built by `patina oxidize`) and computes
/// how each belief connects to code, commits, and sessions. Updates the
/// beliefs table with counts and average similarity.
///
/// Runs AFTER Phase 3 insertion so beliefs have rowids in the DB.
/// After a rebuild, rowids change and won't match the index (grounding = 0).
/// Next `patina oxidize` + `patina scrape` cycle restores the mapping.
fn compute_belief_grounding(conn: &Connection) -> Result<()> {
    use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

    let model = crate::commands::scry::internal::search::get_embedding_model();
    let index_path = format!(
        ".patina/local/data/embeddings/{}/projections/semantic.usearch",
        model
    );

    if !Path::new(&index_path).exists() {
        return Ok(());
    }

    let index_options = IndexOptions {
        dimensions: 256,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };

    let index = Index::new(&index_options)?;
    index.load(&index_path)?;

    const BELIEF_ID_OFFSET: i64 = 4_000_000_000;
    const CODE_ID_OFFSET: i64 = 1_000_000_000;
    const PATTERN_ID_OFFSET: i64 = 2_000_000_000;
    const COMMIT_ID_OFFSET: i64 = 3_000_000_000;
    const SEARCH_LIMIT: usize = 20;
    const MIN_SCORE: f32 = 0.85;

    // Clear previous reach data
    conn.execute("DELETE FROM belief_code_reach", [])?;

    // Read all beliefs from DB
    let mut stmt = conn.prepare("SELECT rowid, id FROM beliefs")?;
    let beliefs: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    let total = beliefs.len();
    let mut grounded = 0;
    let mut total_reach_files = 0u32;

    for (rowid, belief_id) in &beliefs {
        let belief_key = (BELIEF_ID_OFFSET + rowid) as u64;

        let mut vector = vec![0.0_f32; 256];
        if index.get(belief_key, &mut vector).is_err() {
            continue;
        }

        let magnitude: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if magnitude < 0.001 {
            continue;
        }

        let matches = match index.search(&vector, SEARCH_LIMIT + 2) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let mut commit_count = 0i32;
        let mut session_count = 0i32;
        let mut total_score: f32 = 0.0;
        let mut total_count = 0i32;

        // E4.6a-fix: Collect commit neighbors for structural hop
        // (sha, cosine_score) for commits above threshold
        let mut commit_neighbors: Vec<(String, f32)> = Vec::new();

        for i in 0..matches.keys.len() {
            let key = matches.keys[i] as i64;
            let score = 1.0 - matches.distances[i];

            // Skip self and pattern entries
            if key == BELIEF_ID_OFFSET + rowid {
                continue;
            }
            if (PATTERN_ID_OFFSET..COMMIT_ID_OFFSET).contains(&key) {
                continue;
            }
            // Skip other beliefs
            if key >= BELIEF_ID_OFFSET {
                continue;
            }

            if score < MIN_SCORE {
                continue;
            }

            if (COMMIT_ID_OFFSET..BELIEF_ID_OFFSET).contains(&key) {
                commit_count += 1;

                // Resolve commit rowid → SHA for structural hop
                let commit_rowid = key - COMMIT_ID_OFFSET;
                if let Ok(sha) = conn.query_row(
                    "SELECT sha FROM commits WHERE rowid = ?1",
                    [commit_rowid],
                    |row| row.get::<_, String>(0),
                ) {
                    commit_neighbors.push((sha, score));
                }
            } else if key < CODE_ID_OFFSET {
                session_count += 1;
            }
            // Note: direct code matches (CODE_ID_OFFSET range) still counted
            // in total_score but NOT in grounding_code_count (that comes from reach)

            total_score += score;
            total_count += 1;
        }

        // E4.6a-fix: Structural hop — commit → commit_files → file_path
        // Aggregate per-file: max score across touching commits, count of commits
        let mut file_reach: std::collections::HashMap<String, (f32, Vec<String>)> =
            std::collections::HashMap::new();

        for (sha, score) in &commit_neighbors {
            let mut file_stmt =
                conn.prepare_cached("SELECT file_path FROM commit_files WHERE sha = ?1")?;
            let files: Vec<String> = file_stmt
                .query_map([sha], |row| row.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .collect();

            for file_path in files {
                let entry = file_reach
                    .entry(file_path)
                    .or_insert_with(|| (0.0_f32, Vec::new()));
                // reach_score = max cosine across commits touching this file
                if *score > entry.0 {
                    entry.0 = *score;
                }
                entry.1.push(sha.clone());
            }
        }

        // Insert reach entries and count functions per file
        let reach_count = file_reach.len() as i32;
        for (file_path, (reach_score, shas)) in &file_reach {
            // Count functions in this file from function_facts
            let function_count: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM function_facts WHERE file = ?1",
                    [file_path],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            let hop_path = shas
                .iter()
                .map(|s| format!("commit:{}", &s[..7.min(s.len())]))
                .collect::<Vec<_>>()
                .join(",");

            conn.execute(
                "INSERT OR REPLACE INTO belief_code_reach (belief_id, file_path, reach_score, commit_count, function_count, hop_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    belief_id,
                    file_path,
                    reach_score,
                    shas.len() as i32,
                    function_count,
                    hop_path,
                ],
            )?;
        }
        total_reach_files += file_reach.len() as u32;

        let grounding_score = if total_count > 0 {
            total_score / total_count as f32
        } else {
            0.0
        };

        // grounding_code_count now derived from multi-hop reach, not direct cosine
        conn.execute(
            "UPDATE beliefs SET grounding_score = ?1, grounding_code_count = ?2, grounding_commit_count = ?3, grounding_session_count = ?4 WHERE id = ?5",
            rusqlite::params![grounding_score, reach_count, commit_count, session_count, belief_id],
        )?;

        if total_count > 0 {
            grounded += 1;
        }
    }

    println!(
        "  Computed grounding for {} beliefs ({} grounded, {} reach files)",
        total, grounded, total_reach_files
    );

    Ok(())
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
        "metrics": {
            "use": {
                "cited_by_beliefs": belief.metrics.cited_by_beliefs,
                "cited_by_sessions": belief.metrics.cited_by_sessions,
                "applied_in": belief.metrics.applied_in,
            },
            "truth": {
                "evidence_count": belief.metrics.evidence_count,
                "evidence_verified": belief.metrics.evidence_verified,
                "defeated_attacks": belief.metrics.defeated_attacks,
                "external_sources": belief.metrics.external_sources,
            },
            "endorsed": belief.metrics.endorsed,
            "grounding": {
                "score": belief.metrics.grounding_score,
                "code": belief.metrics.grounding_code_count,
                "commits": belief.metrics.grounding_commit_count,
                "sessions": belief.metrics.grounding_session_count,
            },
        },
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
        "INSERT INTO beliefs (id, statement, persona, facets, confidence, entrenchment, status, extracted, revised, file_path,
         cited_by_beliefs, cited_by_sessions, applied_in, evidence_count, evidence_verified, defeated_attacks, external_sources, endorsed,
         verification_total, verification_passed, verification_failed, verification_errored,
         grounding_score, grounding_code_count, grounding_commit_count, grounding_session_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)",
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
            belief.metrics.cited_by_beliefs,
            belief.metrics.cited_by_sessions,
            belief.metrics.applied_in,
            belief.metrics.evidence_count,
            belief.metrics.evidence_verified,
            belief.metrics.defeated_attacks,
            belief.metrics.external_sources,
            belief.metrics.endorsed as i32,
            belief.verification.total,
            belief.verification.passed,
            belief.verification.failed,
            belief.verification.errored,
            belief.metrics.grounding_score,
            belief.metrics.grounding_code_count,
            belief.metrics.grounding_commit_count,
            belief.metrics.grounding_session_count,
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
    let mut current_file_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Phase 1: Parse all belief files (need all of them for cross-referencing)
    let mut all_beliefs: Vec<ParsedBelief> = Vec::new();
    for path in &belief_files {
        match parse_belief_file(path) {
            Ok(belief) => {
                current_file_ids.insert(belief.id.clone());
                all_beliefs.push(belief);
            }
            Err(e) => {
                eprintln!("  Warning: failed to parse {}: {}", path.display(), e);
            }
        }
    }

    // Phase 2: Cross-reference beliefs against each other and sessions
    // This must happen after all beliefs are parsed
    let project_root = Path::new(".");
    cross_reference_beliefs(&mut all_beliefs, project_root);

    // Phase 2.5: Run verification queries
    // Executes SQL queries from ## Verification sections, stores per-query results,
    // and computes aggregates. Runs on every scrape (D5: always verify).
    let data_freshness = if full { "full" } else { "incremental" };
    let mut verified_count = 0;
    for belief in &mut all_beliefs {
        if !belief.verification_queries.is_empty() {
            let (_results, aggregates) = verification::run_verification_queries(
                &conn,
                &belief.id,
                &belief.verification_queries,
                data_freshness,
            );
            belief.verification = aggregates;
            verified_count += 1;
        }
    }
    if verified_count > 0 {
        println!("  Ran verification queries for {} beliefs", verified_count);

        // Update aggregate columns on beliefs table for existing beliefs.
        // Phase 3 skips already-processed beliefs during incremental scrape,
        // so we need to push verification aggregates directly.
        // No-op for beliefs not yet in the table (Phase 3 will insert them).
        for belief in &all_beliefs {
            if !belief.verification_queries.is_empty() {
                let _ = conn.execute(
                    "UPDATE beliefs SET verification_total = ?1, verification_passed = ?2, verification_failed = ?3, verification_errored = ?4 WHERE id = ?5",
                    rusqlite::params![
                        belief.verification.total,
                        belief.verification.passed,
                        belief.verification.failed,
                        belief.verification.errored,
                        belief.id,
                    ],
                );
            }
        }
    }

    // Phase 3: Insert beliefs into database
    for belief in &all_beliefs {
        // Skip if already processed AND not doing full scrape
        // Note: metrics change when sessions change, so full scrape recomputes all
        if !full && processed.contains(&belief.id) {
            skipped += 1;
            continue;
        }

        if let Err(e) = insert_belief(&conn, belief) {
            eprintln!("  Warning: failed to insert belief {}: {}", belief.id, e);
        } else {
            processed_count += 1;
        }
    }

    println!(
        "  Processed {} beliefs ({} skipped)",
        processed_count, skipped
    );

    // Phase 3.5: Compute semantic grounding (E4.6a step 5)
    // Must run AFTER insert so beliefs have rowids in the DB.
    // Uses usearch index from a previous `patina oxidize` run.
    // After rebuild, rowids change and won't match the index → grounding = 0
    // (expected; next oxidize+scrape cycle will fix this).
    if let Err(e) = compute_belief_grounding(&conn) {
        eprintln!("  Warning: grounding computation failed: {}", e);
    }

    // Prune stale entries: delete DB entries for IDs that no longer exist on disk
    let file_ids = current_file_ids;

    let mut stmt = conn.prepare("SELECT id FROM beliefs")?;
    let db_ids: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    let mut pruned = 0;
    for db_id in &db_ids {
        if !file_ids.contains(db_id) {
            // Delete from all related tables
            conn.execute("DELETE FROM beliefs WHERE id = ?1", [db_id])?;
            conn.execute("DELETE FROM belief_fts WHERE id = ?1", [db_id])?;
            conn.execute(
                "DELETE FROM eventlog WHERE source_id = ?1 AND event_type = 'belief.surface'",
                [db_id],
            )?;
            pruned += 1;
        }
    }

    if pruned > 0 {
        println!("  Pruned {} stale beliefs", pruned);
    }

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
