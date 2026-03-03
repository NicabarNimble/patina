//! Belief scraper - extracts epistemic beliefs from layer/surface/epistemic/beliefs/
//!
//! Uses unified eventlog pattern:
//! - Inserts belief.surface events into eventlog table
//! - Creates materialized views (beliefs) from eventlog
//! - Runs verification queries (Phase 2.5) and stores results

mod verification;

use anyhow::Result;
use patina::mother::BeliefStatus;
use regex::Regex;
use rusqlite::Connection;
use serde_json::json;
use std::path::Path;
use std::time::Instant;

use super::database;
use super::ScrapeStats;

const BELIEFS_DIR: &str = "layer/surface/epistemic/beliefs";
const VALUES_DIR: &str = "layer/core/values";

/// Parsed belief from markdown file
#[derive(Debug)]
struct ParsedBelief {
    id: String,
    kind: String,         // "belief" or "value"
    statement: String,    // One-sentence statement after # heading
    persona: String,      // architect, etc.
    facets: Vec<String>,  // Domain tags
    confidence: f64,      // 0.0-1.0 (legacy, will be removed)
    entrenchment: String, // low/medium/high/very-high
    status: BeliefStatus, // active/scoped/defeated/archived
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
    // Phase E: imported flag (from imported_from frontmatter)
    imported: bool,
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

    // E4.6a: Semantic grounding — how connected is this belief to code/commits/sessions/forge?
    grounding_score: f32, // Average similarity to grounded neighbors (0.0-1.0)
    grounding_code_count: i32, // Code functions above similarity threshold
    grounding_commit_count: i32, // Commits above similarity threshold
    grounding_session_count: i32, // Sessions above similarity threshold
    grounding_forge_count: i32, // Forge issues/PRs above similarity threshold

    // Temporal: staleness detection (Phase A)
    last_file_touch: Option<String>, // file mtime → YYYY-MM-DD (weakest signal)
    last_frontmatter_revision: Option<String>, // from `revised` frontmatter field
    last_session_citation: Option<String>, // MAX date from citing session filenames
    last_verification_run: Option<String>, // today's date if belief has verification queries
    last_activity: Option<String>,   // computed: strong ?? last_file_touch ?? NULL

    // Health score (Phase B)
    health_score: f64, // 0.0-1.0 composite score

    // Contradiction detection (Phase C)
    attacked_by_ids: Vec<String>, // non-defeated attacker belief IDs from ## Attacked-By
    attacks_ids: Vec<String>,     // non-defeated target belief IDs from ## Attacks
    contested_by: Vec<String>,    // active beliefs that contest this one (bidirectional)

    // Relationship edges (belief-graph Phase B)
    supports_ids: Vec<String>, // belief IDs from ## Supports section
    defeated_attack_targets: Vec<String>, // defeated targets from ## Attacks (for edge rows)
    defeated_attacker_ids: Vec<String>, // defeated attackers from ## Attacked-By (for edge rows)
}

/// Create materialized views for belief events
fn create_materialized_views(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Beliefs view (materialized from belief.* events)
        CREATE TABLE IF NOT EXISTS beliefs (
            id TEXT PRIMARY KEY,
            kind TEXT DEFAULT 'belief',
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
            grounding_session_count INTEGER DEFAULT 0,
            grounding_forge_count INTEGER DEFAULT 0,
            -- Belief truthfulness: temporal signals
            last_file_touch TEXT,
            last_frontmatter_revision TEXT,
            last_session_citation TEXT,
            last_verification_run TEXT,
            last_activity TEXT,
            -- Belief truthfulness: health score
            health_score REAL DEFAULT 0.0,
            -- Belief truthfulness: drift + contradiction
            verification_drifted INTEGER DEFAULT 0,
            contested_by TEXT DEFAULT ''
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

        -- Belief relationship edges (belief-graph Phase B)
        CREATE TABLE IF NOT EXISTS belief_supports (
            from_belief TEXT NOT NULL,
            to_belief TEXT NOT NULL,
            PRIMARY KEY (from_belief, to_belief)
        );

        CREATE TABLE IF NOT EXISTS belief_attacks (
            from_belief TEXT NOT NULL,
            to_belief TEXT NOT NULL,
            defeated INTEGER DEFAULT 0,
            PRIMARY KEY (from_belief, to_belief)
        );

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
        ("kind", "TEXT DEFAULT 'belief'"),
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
        ("grounding_forge_count", "INTEGER DEFAULT 0"),
        // Belief truthfulness: temporal signals
        ("last_file_touch", "TEXT"),
        ("last_frontmatter_revision", "TEXT"),
        ("last_session_citation", "TEXT"),
        ("last_verification_run", "TEXT"),
        ("last_activity", "TEXT"),
        // Belief truthfulness: health score
        ("health_score", "REAL DEFAULT 0.0"),
        // Belief truthfulness: drift + contradiction
        ("verification_drifted", "INTEGER DEFAULT 0"),
        ("contested_by", "TEXT DEFAULT ''"),
        // Belief-graph Phase E: import detection
        ("imported", "INTEGER DEFAULT 0"),
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

    // Extract last_file_touch from file mtime (free — already reading file)
    let last_file_touch = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.format("%Y-%m-%d").to_string()
        });

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
    let mut status = BeliefStatus::Active;
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
                status = BeliefStatus::from_str_or_default(cap[1].trim());
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

    // Detect imported_from frontmatter (Phase E: sole authoritative import signal)
    let mut imported = false;
    if let Some(after_start) = content.strip_prefix("---") {
        if let Some(end) = after_start.find("---") {
            let frontmatter = &after_start[..end];
            if let Ok(re) = regex::RegexBuilder::new(r"^imported_from:\s*\S+")
                .multi_line(true)
                .build()
            {
                imported = re.is_match(frontmatter);
            }
        }
    }

    // Extract one-sentence statement (line after # id heading)
    let statement = extract_statement(&content, &id);

    // Compute per-file metrics from markdown sections
    let mut metrics = extract_file_metrics(&content);

    // Check for endorsed field in frontmatter (default: true for existing beliefs)
    metrics.endorsed = true; // All beliefs created via skill are user-initiated

    // Set temporal signals from file-level data
    metrics.last_file_touch = last_file_touch;
    metrics.last_frontmatter_revision = revised.clone(); // already YYYY-MM-DD from frontmatter

    // Parse verification queries from ## Verification section
    let verification_queries = verification::parse_verification_blocks(&content);

    Ok(ParsedBelief {
        id,
        kind: "belief".to_string(),
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
        imported,
    })
}

/// Parse a value file from layer/core/values/.
/// Values are simpler than beliefs: type: value, fixed entrenchment, no verification.
fn parse_value_file(path: &Path) -> Result<ParsedBelief> {
    let content = std::fs::read_to_string(path)?;
    let file_path = path.to_string_lossy().to_string();
    let file_size = content.len();

    // Warn if value exceeds 1KB (advisory, not blocking)
    if file_size > 1024 {
        let id_hint = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        println!(
            "  warning: value '{}' exceeds 1KB ({} bytes)",
            id_hint, file_size
        );
    }

    let last_file_touch = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.format("%Y-%m-%d").to_string()
        });

    // Defaults
    let mut id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut facets = Vec::new();
    let mut status = BeliefStatus::Active;

    // Parse YAML frontmatter
    if let Some(after_start) = content.strip_prefix("---") {
        if let Some(end) = after_start.find("---") {
            let frontmatter = &after_start[..end];

            // Verify type: value
            let is_value = regex::RegexBuilder::new(r"^type:\s*value\s*$")
                .multi_line(true)
                .build()
                .ok()
                .map(|re| re.is_match(frontmatter))
                .unwrap_or(false);

            if !is_value {
                anyhow::bail!("not a value file (missing type: value)");
            }

            if let Some(cap) = regex::RegexBuilder::new(r"^id:\s*(.+)$")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                id = cap[1].trim().to_string();
            }

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

            if let Some(cap) = regex::RegexBuilder::new(r"^status:\s*(.+)$")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| re.captures(frontmatter))
            {
                status = BeliefStatus::from_str_or_default(cap[1].trim());
            }
        } else {
            anyhow::bail!("unclosed frontmatter");
        }
    } else {
        anyhow::bail!("no frontmatter");
    }

    let statement = extract_statement(&content, &id);

    let metrics = BeliefMetrics {
        endorsed: true,
        last_file_touch,
        ..Default::default()
    };

    Ok(ParsedBelief {
        id,
        kind: "value".to_string(),
        statement,
        persona: "core".to_string(),
        facets,
        confidence: 1.0,
        entrenchment: "very-high".to_string(),
        status,
        extracted: None,
        revised: None,
        content,
        file_path,
        metrics,
        verification_queries: Vec::new(),
        verification: verification::VerificationAggregates::default(),
        imported: false,
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
    let wikilink_re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();

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
                    // Capture defeated attacker IDs for edge rows (defeated=1)
                    for cap in wikilink_re.captures_iter(trimmed) {
                        metrics.defeated_attacker_ids.push(cap[1].to_string());
                    }
                } else {
                    // Phase C: collect non-defeated attacker IDs from [[wikilinks]]
                    for cap in wikilink_re.captures_iter(trimmed) {
                        metrics.attacked_by_ids.push(cap[1].to_string());
                    }
                }
            }
            s if s.starts_with("## Attacks") => {
                if trimmed.contains("status: defeated") {
                    // Capture defeated target IDs for edge rows (defeated=1)
                    for cap in wikilink_re.captures_iter(trimmed) {
                        metrics.defeated_attack_targets.push(cap[1].to_string());
                    }
                } else {
                    // Phase C: collect non-defeated target IDs from [[wikilinks]]
                    for cap in wikilink_re.captures_iter(trimmed) {
                        metrics.attacks_ids.push(cap[1].to_string());
                    }
                }
            }
            s if s.starts_with("## Supports") => {
                // Parse supported belief IDs from [[wikilinks]]
                let mut found_link = false;
                for cap in wikilink_re.captures_iter(trimmed) {
                    metrics.supports_ids.push(cap[1].to_string());
                    found_link = true;
                }
                // Warn on non-link entries
                if !found_link {
                    // Extract bare belief ID: first token before ':' or space
                    let entry = trimmed.trim_start_matches("- ").trim_start_matches("* ");
                    let bare_id = entry.split(&[':', ' '][..]).next().unwrap_or("").trim();
                    if !bare_id.is_empty() {
                        eprintln!(
                            "  warning: ## Supports entry without [[wikilink]]: {}",
                            bare_id
                        );
                        metrics.supports_ids.push(bare_id.to_string());
                    }
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
/// Computes cited_by_beliefs, cited_by_sessions, last_session_citation,
/// and contested_by for each belief.
fn cross_reference_beliefs(beliefs: &mut [ParsedBelief], project_root: &Path) {
    let sessions_dir = project_root.join("layer/sessions");

    // Regex for extracting date from session filenames: YYYYMMDD-HHMMSS.md
    let session_date_re = Regex::new(r"^(\d{4})(\d{2})(\d{2})-\d{6}").unwrap();

    // Collect all belief IDs for reference
    let belief_ids: Vec<String> = beliefs.iter().map(|b| b.id.clone()).collect();

    // Collect all belief file contents (for cross-referencing)
    let belief_contents: Vec<(String, String)> = beliefs
        .iter()
        .map(|b| (b.id.clone(), b.content.clone()))
        .collect();

    // Read session files once, build reverse index: belief_id → (citation_count, max_date)
    let mut session_citations: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();
    let mut session_max_dates: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    if sessions_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&sessions_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map(|ext| ext == "md").unwrap_or(false) {
                    // Extract date from session filename (skip non-matching filenames)
                    let session_date = path.file_stem().and_then(|s| s.to_str()).and_then(|name| {
                        session_date_re
                            .captures(name)
                            .map(|cap| format!("{}-{}-{}", &cap[1], &cap[2], &cap[3]))
                    });

                    if let Ok(session_content) = std::fs::read_to_string(&path) {
                        for bid in &belief_ids {
                            if session_content.contains(bid.as_str()) {
                                *session_citations.entry(bid.clone()).or_insert(0) += 1;

                                // Track MAX session date per belief
                                if let Some(ref date) = session_date {
                                    let entry = session_max_dates.entry(bid.clone()).or_default();
                                    if entry.is_empty() || date.as_str() > entry.as_str() {
                                        *entry = date.clone();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Build status map for contest detection (Phase C)
    let status_map: std::collections::HashMap<String, BeliefStatus> =
        beliefs.iter().map(|b| (b.id.clone(), b.status)).collect();

    // Build bidirectional contest map (Phase C)
    // A contests B if: A's ## Attacks lists B (non-defeated) AND B is active
    // B is contested by A if: B's ## Attacked-By lists A (non-defeated) AND A is active
    let mut contest_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for belief in beliefs.iter() {
        // From ## Attacks: belief attacks these targets
        for target_id in &belief.metrics.attacks_ids {
            if status_map.get(target_id) == Some(&BeliefStatus::Active) {
                contest_map
                    .entry(target_id.clone())
                    .or_default()
                    .push(belief.id.clone());
            }
        }
        // From ## Attacked-By: belief is attacked by these
        for attacker_id in &belief.metrics.attacked_by_ids {
            if status_map.get(attacker_id) == Some(&BeliefStatus::Active) {
                contest_map
                    .entry(belief.id.clone())
                    .or_default()
                    .push(attacker_id.clone());
            }
        }
    }

    // Deduplicate contest entries
    for ids in contest_map.values_mut() {
        ids.sort();
        ids.dedup();
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

        // Set last_session_citation from MAX date
        belief.metrics.last_session_citation = session_max_dates.get(bid).cloned();
        // Filter out empty strings
        if belief.metrics.last_session_citation.as_deref() == Some("") {
            belief.metrics.last_session_citation = None;
        }

        // Set contested_by from bidirectional map (Phase C)
        belief.metrics.contested_by = contest_map.get(bid).cloned().unwrap_or_default();
    }
}

/// Extract searchable keywords from a belief ID for lexical fallback.
/// Splits on `-`, removes stop words and short tokens, returns unique keywords.
fn extract_belief_keywords(belief_id: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "is", "the", "a", "an", "in", "on", "of", "for", "to", "and", "or", "not", "no", "over",
        "vs", "with", "from", "by", "be", "at", "as", "do", "if", "its", "own", "first", "early",
        "often", "before", "after", "always", "never", "every", "many", "requires", "needs",
    ];
    belief_id
        .split('-')
        .filter(|w| w.len() >= 3)
        .filter(|w| !STOP_WORDS.contains(w))
        .map(|w| w.to_lowercase())
        .collect()
}

/// Check if the usearch index has been rebuilt since the last grounding computation.
///
/// Compares the usearch index file mtime with a stored watermark. If the index
/// is newer, grounding needs to be recomputed for all beliefs.
///
/// Returns Some(current_mtime) if grounding is needed (index changed or no watermark),
/// None if the index hasn't changed. The caller passes the mtime to
/// `update_grounding_watermark()` to avoid a TOCTOU race between read and write.
fn grounding_index_changed(conn: &Connection) -> Option<String> {
    let model = crate::commands::scry::internal::search::get_embedding_model();
    let index_path = format!(
        ".patina/local/data/embeddings/{}/projections/semantic.usearch",
        model
    );

    let current_mtime = match std::fs::metadata(&index_path).and_then(|m| m.modified()) {
        Ok(t) => t
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string(),
        Err(_) => return None, // No index = no grounding possible
    };

    // Compare with stored watermark
    match database::get_last_processed(conn, "grounding_index_mtime") {
        Ok(Some(stored)) if stored == current_mtime => None, // Unchanged
        _ => Some(current_mtime),                            // Changed or first grounding
    }
}

/// Store the usearch index mtime as a grounding watermark.
///
/// Accepts the mtime string from `grounding_index_changed()` to avoid reading
/// the filesystem again (eliminates TOCTOU race if oxidize runs concurrently).
fn update_grounding_watermark(conn: &Connection, mtime: &str) {
    let _ = database::set_last_processed(conn, "grounding_index_mtime", mtime);
}

fn is_source_code(path: &str) -> bool {
    const SOURCE_EXTENSIONS: &[&str] = &[
        ".rs", ".py", ".ts", ".tsx", ".js", ".jsx", ".go", ".c", ".cpp", ".h", ".java", ".rb",
        ".sh", ".sql", ".swift", ".kt", ".zig",
    ];
    SOURCE_EXTENSIONS.iter().any(|ext| path.ends_with(ext))
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

    use patina::embeddings::offsets::*;
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
    let mut total_source_files = 0u32;
    let mut total_lexical_fallbacks = 0u32;

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
        let mut forge_count = 0i32;
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
            if (BELIEF_ID_OFFSET..FORGE_ID_OFFSET).contains(&key) && key != BELIEF_ID_OFFSET + rowid
            {
                continue;
            }

            if score < MIN_SCORE {
                continue;
            }

            if key >= FORGE_ID_OFFSET {
                // Forge event (issue or PR)
                forge_count += 1;
                total_score += score;
                total_count += 1;
            } else if (COMMIT_ID_OFFSET..BELIEF_ID_OFFSET).contains(&key) {
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
                total_score += score;
                total_count += 1;
            } else if key < CODE_ID_OFFSET {
                session_count += 1;
                total_score += score;
                total_count += 1;
            } else {
                // Code matches — counted in total_score but NOT in grounding_code_count
                total_score += score;
                total_count += 1;
            }
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
                // Filter at the hop: only source code files enter reach
                if !is_source_code(&file_path) {
                    continue;
                }
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

        // Lexical fallback: if structural hop found no source files but belief has
        // commit neighbors, search function_facts.file for belief ID keywords.
        // The semantic hop confirmed topic relevance; this finds the code for that topic.
        if file_reach.is_empty() && !commit_neighbors.is_empty() {
            let keywords = extract_belief_keywords(belief_id);
            if !keywords.is_empty() {
                let best_commit_score = commit_neighbors
                    .iter()
                    .map(|(_, s)| *s)
                    .fold(0.0_f32, f32::max);
                let hop_tag = "lexical-fallback";

                for keyword in &keywords {
                    let pattern = format!("%{}%", keyword);
                    let mut kw_stmt = conn.prepare_cached(
                        "SELECT DISTINCT file FROM function_facts WHERE file LIKE ?1",
                    )?;
                    let files: Vec<String> = kw_stmt
                        .query_map([&pattern], |row| row.get::<_, String>(0))?
                        .filter_map(|r| r.ok())
                        .filter(|f| is_source_code(f))
                        .collect();

                    for file_path in files {
                        let entry = file_reach
                            .entry(file_path)
                            .or_insert_with(|| (0.0_f32, Vec::new()));
                        if best_commit_score > entry.0 {
                            entry.0 = best_commit_score;
                        }
                        if !entry.1.contains(&hop_tag.to_string()) {
                            entry.1.push(hop_tag.to_string());
                        }
                    }
                }
                if !file_reach.is_empty() {
                    total_lexical_fallbacks += 1;
                }
            }
        }

        // Insert reach entries — all entries are source code (filtered at the hop)
        let source_file_count = file_reach.len() as i32;
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
        total_source_files += source_file_count as u32;

        let grounding_score = if total_count > 0 {
            total_score / total_count as f32
        } else {
            0.0
        };

        // grounding_code_count = source code files only (not docs/configs)
        conn.execute(
            "UPDATE beliefs SET grounding_score = ?1, grounding_code_count = ?2, grounding_commit_count = ?3, grounding_session_count = ?4, grounding_forge_count = ?5 WHERE id = ?6",
            rusqlite::params![grounding_score, source_file_count, commit_count, session_count, forge_count, belief_id],
        )?;

        if total_count > 0 {
            grounded += 1;
        }
    }

    let precision = if total_reach_files > 0 {
        (total_source_files as f64 / total_reach_files as f64 * 100.0) as u32
    } else {
        0
    };
    println!(
        "  Computed grounding for {} beliefs ({} grounded, {} reach files, {} source, {}% precision, {} lexical fallbacks)",
        total, grounded, total_reach_files, total_source_files, precision, total_lexical_fallbacks
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
        "status": belief.status.as_str(),
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
                "forge": belief.metrics.grounding_forge_count,
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

    let contested_by_str = belief.metrics.contested_by.join(",");

    conn.execute(
        "INSERT INTO beliefs (id, kind, statement, persona, facets, confidence, entrenchment, status, extracted, revised, file_path,
         cited_by_beliefs, cited_by_sessions, applied_in, evidence_count, evidence_verified, defeated_attacks, external_sources, endorsed,
         verification_total, verification_passed, verification_failed, verification_errored,
         grounding_score, grounding_code_count, grounding_commit_count, grounding_session_count, grounding_forge_count,
         last_file_touch, last_frontmatter_revision, last_session_citation, last_verification_run, last_activity,
         health_score, contested_by, imported)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28,
                 ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36)",
        rusqlite::params![
            &belief.id,
            &belief.kind,
            &belief.statement,
            &belief.persona,
            &facets_str,
            belief.confidence,
            &belief.entrenchment,
            belief.status.as_str(),
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
            belief.metrics.grounding_forge_count,
            // 7 new belief-truthfulness params (verification_drifted is DEFAULT 0, not passed)
            &belief.metrics.last_file_touch,
            &belief.metrics.last_frontmatter_revision,
            &belief.metrics.last_session_citation,
            &belief.metrics.last_verification_run,
            &belief.metrics.last_activity,
            belief.metrics.health_score,
            &contested_by_str,
            belief.imported as i32,
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

/// Compute last_activity using the fallback algorithm from the spec:
/// strong = MAX(last_frontmatter_revision, last_session_citation, last_verification_run)
/// last_activity = strong ?? last_file_touch ?? NULL
fn compute_last_activity(metrics: &BeliefMetrics) -> Option<String> {
    // Collect the three strong signals
    let strong_signals: Vec<&str> = [
        metrics.last_frontmatter_revision.as_deref(),
        metrics.last_session_citation.as_deref(),
        metrics.last_verification_run.as_deref(),
    ]
    .iter()
    .filter_map(|s| *s)
    .collect();

    // MAX of strong signals (string comparison works for YYYY-MM-DD)
    let strong = strong_signals.iter().max().map(|s| s.to_string());

    // Fallback: last_file_touch participates ONLY when ALL strong signals are NULL
    if strong.is_some() {
        strong
    } else {
        metrics.last_file_touch.clone()
    }
}

/// Compute health score per spec Phase B formula:
/// health = w_use * use_score + w_truth * truth_score + w_fresh * freshness_score
fn compute_health_score(metrics: &BeliefMetrics, stale_days: u32) -> f64 {
    const W_USE: f64 = 0.3;
    const W_TRUTH: f64 = 0.4;
    const W_FRESH: f64 = 0.3;

    let use_score = ((metrics.cited_by_beliefs + metrics.cited_by_sessions) as f64 / 3.0).min(1.0);
    let truth_score = metrics.evidence_verified as f64 / (metrics.evidence_count.max(1)) as f64;

    let freshness_score = match &metrics.last_activity {
        Some(date_str) => {
            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
            if let Ok(activity_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                if let Ok(today_date) = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d") {
                    let days = (today_date - activity_date).num_days().max(0) as f64;
                    (1.0 - (days / stale_days as f64).min(1.0)).max(0.0)
                } else {
                    0.0
                }
            } else {
                0.0
            }
        }
        None => 0.0, // NULL last_activity → maximally stale
    };

    W_USE * use_score + W_TRUTH * truth_score + W_FRESH * freshness_score
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

    // Load config for stale_days (each command loads independently per spec)
    let config = patina::project::load(Path::new(".")).unwrap_or_default();
    let stale_days = config.beliefs.stale_days;

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

    let mut processed_count: usize = 0;
    let mut skipped: usize = 0;
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

    // Phase 1b: Parse value files from layer/core/values/
    // Values are axioms — they skip verification and cross-referencing.
    let values_path = Path::new(VALUES_DIR);
    let mut values_count = 0;
    if values_path.exists() {
        let mut value_files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(values_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map(|ext| ext == "md").unwrap_or(false) {
                    value_files.push(path);
                }
            }
        }
        value_files.sort();

        for path in &value_files {
            match parse_value_file(path) {
                Ok(value) => {
                    current_file_ids.insert(value.id.clone());
                    values_count += 1;
                    all_beliefs.push(value);
                }
                Err(e) => {
                    eprintln!("  Warning: failed to parse value {}: {}", path.display(), e);
                }
            }
        }

        if values_count > 0 {
            println!("  Parsed {} values from {}", values_count, VALUES_DIR);
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
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
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
            // Set last_verification_run to today for beliefs with verification queries
            belief.metrics.last_verification_run = Some(today.clone());
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

    // Compute last_activity and health_score BEFORE Phase 3
    // (both insert and Phase 3a update paths use the same values)
    for belief in &mut all_beliefs {
        belief.metrics.last_activity = compute_last_activity(&belief.metrics);
        belief.metrics.health_score = compute_health_score(&belief.metrics, stale_days);
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

    // Phase 3a: Push temporal updates for skipped beliefs (incremental only)
    // Temporal signals change even when the belief file doesn't —
    // a new session may cite it, or verification results may differ.
    if !full {
        for belief in &all_beliefs {
            if processed.contains(&belief.id) {
                let contested_by_str = belief.metrics.contested_by.join(",");
                let _ = conn.execute(
                    "UPDATE beliefs SET last_file_touch = ?1, last_frontmatter_revision = ?2, last_session_citation = ?3, last_verification_run = ?4, last_activity = ?5, health_score = ?6, contested_by = ?7 WHERE id = ?8",
                    rusqlite::params![
                        &belief.metrics.last_file_touch,
                        &belief.metrics.last_frontmatter_revision,
                        &belief.metrics.last_session_citation,
                        &belief.metrics.last_verification_run,
                        &belief.metrics.last_activity,
                        belief.metrics.health_score,
                        &contested_by_str,
                        belief.id,
                    ],
                );
            }
        }
    }

    // Phase 3b: Apply verification drift flags
    // Compares _prev (snapshot from start of scrape) with current results.
    // Reset all → flag only pass→non-pass transitions.
    let _ = conn.execute("UPDATE beliefs SET verification_drifted = 0", []);
    let drift_result = conn.execute_batch(
        r#"
        UPDATE beliefs SET verification_drifted = 1
          WHERE id IN (SELECT DISTINCT p.belief_id
            FROM belief_verifications_prev p
            JOIN belief_verifications c ON p.belief_id = c.belief_id AND p.label = c.label
            WHERE p.last_status = 'pass' AND c.last_status != 'pass');
        "#,
    );
    // Ignore error (no _prev table on first scrape — expected)
    if drift_result.is_ok() {
        let drifted: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM beliefs WHERE verification_drifted = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if drifted > 0 {
            println!("  Verification drift detected: {} beliefs", drifted);
        }
    }
    // Clean up _prev table
    let _ = conn.execute_batch("DROP TABLE IF EXISTS belief_verifications_prev;");

    if values_count > 0 {
        println!(
            "  Processed {} beliefs + {} values ({} skipped)",
            processed_count.saturating_sub(values_count),
            values_count,
            skipped
        );
    } else {
        println!(
            "  Processed {} beliefs ({} skipped)",
            processed_count, skipped
        );
    }

    // Phase 3.5: Compute semantic grounding (E4.6a step 5)
    // Must run AFTER insert so beliefs have rowids in the DB.
    // Uses usearch index from a previous `patina oxidize` run.
    // After rebuild, rowids change and won't match the index → grounding = 0
    // (expected; next oxidize+scrape cycle will fix this).
    //
    // Incremental optimization ([[scrape-diff-driven]] Phase 4):
    // Grounding only changes when (a) new beliefs are added, or (b) the usearch
    // index is rebuilt by oxidize. Code changes alone don't affect grounding.
    // Skip grounding when neither condition is true.
    let new_beliefs_added = processed_count > 0;
    let index_mtime = grounding_index_changed(&conn);
    let grounding_needed = full || new_beliefs_added || index_mtime.is_some();
    if grounding_needed {
        if let Err(e) = compute_belief_grounding(&conn) {
            eprintln!("  Warning: grounding computation failed: {}", e);
        }
        // Update the grounding watermark using the mtime we already read
        // (read-once eliminates TOCTOU race if oxidize runs concurrently)
        if let Some(mtime) = &index_mtime {
            update_grounding_watermark(&conn, mtime);
        }
    } else {
        println!("  Grounding: skipped (no new beliefs, index unchanged)");
    }

    // Phase 4: Write belief relationship edges (belief-graph Phase B)
    // Separate pass after all beliefs are inserted. Iterates ALL beliefs
    // because cross_reference_beliefs() always computes fresh data for all.
    conn.execute("DELETE FROM belief_supports", [])?;
    conn.execute("DELETE FROM belief_attacks", [])?;

    let mut supports_written = 0;
    let mut attacks_written = 0;

    for belief in &all_beliefs {
        // Supports edges (from this belief to supported beliefs)
        for target_id in &belief.metrics.supports_ids {
            conn.execute(
                "INSERT OR IGNORE INTO belief_supports (from_belief, to_belief) VALUES (?1, ?2)",
                rusqlite::params![&belief.id, target_id],
            )?;
            supports_written += 1;
        }

        // Attacks edges — non-defeated (from this belief to targets, defeated=0)
        for target_id in &belief.metrics.attacks_ids {
            conn.execute(
                "INSERT OR IGNORE INTO belief_attacks (from_belief, to_belief, defeated) VALUES (?1, ?2, 0)",
                rusqlite::params![&belief.id, target_id],
            )?;
            // Defeated=1 wins: if already inserted as defeated, don't downgrade
            attacks_written += 1;
        }

        // Attacks edges — defeated targets from ## Attacks (defeated=1)
        for target_id in &belief.metrics.defeated_attack_targets {
            conn.execute(
                "INSERT OR IGNORE INTO belief_attacks (from_belief, to_belief, defeated) VALUES (?1, ?2, 1)",
                rusqlite::params![&belief.id, target_id],
            )?;
            // Upgrade to defeated if already exists as non-defeated
            conn.execute(
                "UPDATE belief_attacks SET defeated = 1 WHERE from_belief = ?1 AND to_belief = ?2 AND defeated = 0",
                rusqlite::params![&belief.id, target_id],
            )?;
            attacks_written += 1;
        }

        // Attacked-By edges — non-defeated (reverse: from attacker to this belief, defeated=0)
        for attacker_id in &belief.metrics.attacked_by_ids {
            conn.execute(
                "INSERT OR IGNORE INTO belief_attacks (from_belief, to_belief, defeated) VALUES (?1, ?2, 0)",
                rusqlite::params![attacker_id, &belief.id],
            )?;
            attacks_written += 1;
        }

        // Attacked-By edges — defeated (reverse: from attacker to this belief, defeated=1)
        for attacker_id in &belief.metrics.defeated_attacker_ids {
            conn.execute(
                "INSERT OR IGNORE INTO belief_attacks (from_belief, to_belief, defeated) VALUES (?1, ?2, 1)",
                rusqlite::params![attacker_id, &belief.id],
            )?;
            conn.execute(
                "UPDATE belief_attacks SET defeated = 1 WHERE from_belief = ?1 AND to_belief = ?2 AND defeated = 0",
                rusqlite::params![attacker_id, &belief.id],
            )?;
            attacks_written += 1;
        }
    }

    if supports_written > 0 || attacks_written > 0 {
        println!(
            "  Wrote {} supports + {} attacks edges",
            supports_written, attacks_written
        );
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

    // Emit measurement: beliefs scrape capture metrics
    patina::measure::emit_or_warn(
        "capture",
        "scrape",
        "beliefs",
        &serde_json::json!({
            "beliefs_processed": processed_count,
            "values_processed": values_count,
            "beliefs_skipped": skipped,
            "beliefs_verified": verified_count,
            "supports_edges": supports_written,
            "attacks_edges": attacks_written,
            "duration_ms": elapsed.as_millis() as u64,
        }),
    );

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
        assert_eq!(belief.status, BeliefStatus::Active);
        assert_eq!(belief.statement, "Prefer synchronous code.");
    }

    // =========================================================================
    // Belief Truthfulness Tests (per spec Verification Plan)
    // =========================================================================

    #[test]
    fn test_last_activity_max() {
        // All four signals present → MAX of strong signals wins
        let mut m = BeliefMetrics::default();
        m.last_frontmatter_revision = Some("2026-01-10".to_string());
        m.last_session_citation = Some("2026-02-05".to_string());
        m.last_verification_run = Some("2026-01-20".to_string());
        m.last_file_touch = Some("2026-02-16".to_string());
        assert_eq!(
            compute_last_activity(&m),
            Some("2026-02-05".to_string()) // session citation is MAX of strong
        );

        // Only file touch → fallback
        let mut m2 = BeliefMetrics::default();
        m2.last_file_touch = Some("2026-02-16".to_string());
        assert_eq!(compute_last_activity(&m2), Some("2026-02-16".to_string()));

        // All NULL → NULL
        let m3 = BeliefMetrics::default();
        assert_eq!(compute_last_activity(&m3), None);

        // Strong signal present → file touch ignored
        let mut m4 = BeliefMetrics::default();
        m4.last_frontmatter_revision = Some("2025-06-01".to_string());
        m4.last_file_touch = Some("2026-02-16".to_string()); // much more recent but weak
        assert_eq!(
            compute_last_activity(&m4),
            Some("2025-06-01".to_string()) // strong signal wins, not file touch
        );
    }

    #[test]
    fn test_session_filename_parsing() {
        let re = Regex::new(r"^(\d{4})(\d{2})(\d{2})-\d{6}").unwrap();

        // Standard format
        let name = "20260215-230959";
        let cap = re.captures(name).unwrap();
        let date = format!("{}-{}-{}", &cap[1], &cap[2], &cap[3]);
        assert_eq!(date, "2026-02-15");

        // With suffix (e.g., -init)
        let name2 = "20250812-123323-init";
        let cap2 = re.captures(name2).unwrap();
        let date2 = format!("{}-{}-{}", &cap2[1], &cap2[2], &cap2[3]);
        assert_eq!(date2, "2025-08-12");

        // Non-matching filename → skip
        assert!(re.captures("session_summary_july27").is_none());
        assert!(re.captures("notes").is_none());
    }

    #[test]
    fn test_health_score_computation() {
        // Zero-evidence: max possible score is 0.6 (use + freshness, no truth)
        let mut m = BeliefMetrics::default();
        m.cited_by_beliefs = 3;
        m.cited_by_sessions = 3; // use_score = min(1.0, 6/3) = 1.0
        m.evidence_count = 0;
        m.evidence_verified = 0;
        m.last_activity = Some("2026-02-16".to_string()); // fresh
        let score = compute_health_score(&m, 90);
        assert!(
            score <= 0.61,
            "zero-evidence max should be ~0.6, got {}",
            score
        );
        assert!(score >= 0.5, "zero-evidence should be ~0.5+, got {}", score);

        // All-healthy: near 1.0
        let mut m2 = BeliefMetrics::default();
        m2.cited_by_beliefs = 3;
        m2.cited_by_sessions = 3;
        m2.evidence_count = 3;
        m2.evidence_verified = 3;
        m2.last_activity = Some("2026-02-16".to_string());
        let score2 = compute_health_score(&m2, 90);
        assert!(
            score2 > 0.95,
            "all-healthy should be near 1.0, got {}",
            score2
        );

        // All-stale: freshness = 0.0
        let mut m3 = BeliefMetrics::default();
        m3.cited_by_beliefs = 3;
        m3.cited_by_sessions = 3;
        m3.evidence_count = 3;
        m3.evidence_verified = 3;
        m3.last_activity = Some("2025-01-01".to_string()); // very old
        let score3 = compute_health_score(&m3, 90);
        // freshness = 0.0, use = 1.0, truth = 1.0 → 0.3 + 0.4 + 0.0 = 0.7
        assert!(
            (score3 - 0.7).abs() < 0.01,
            "stale should be 0.7, got {}",
            score3
        );

        // NULL last_activity → freshness = 0.0
        let mut m4 = BeliefMetrics::default();
        m4.cited_by_beliefs = 3;
        m4.cited_by_sessions = 3;
        m4.evidence_count = 3;
        m4.evidence_verified = 3;
        m4.last_activity = None;
        let score4 = compute_health_score(&m4, 90);
        assert!(
            (score4 - 0.7).abs() < 0.01,
            "null activity should be 0.7, got {}",
            score4
        );
    }

    #[test]
    fn test_verification_drift_detection() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();

        // Create _prev table with a passing result
        conn.execute_batch(
            "CREATE TABLE belief_verifications_prev (
                belief_id TEXT NOT NULL, label TEXT NOT NULL,
                query_type TEXT, query_text TEXT, expectation TEXT,
                last_status TEXT NOT NULL, last_result TEXT, last_error TEXT,
                last_run_at TEXT, data_freshness TEXT,
                PRIMARY KEY (belief_id, label)
            );
            INSERT INTO belief_verifications_prev VALUES
                ('belief-a', 'check-1', 'sql', 'SELECT 1', '= 1', 'pass', '1', NULL, '2026-01-01', 'full'),
                ('belief-b', 'check-2', 'sql', 'SELECT 1', '= 1', 'pass', '1', NULL, '2026-01-01', 'full'),
                ('belief-c', 'check-3', 'sql', 'SELECT 1', '= 1', 'error', '0', 'err', '2026-01-01', 'full');
            ",
        ).unwrap();

        // Create current table: belief-a drifted (pass→contested), belief-b stayed pass, belief-c error→pass (not drift)
        conn.execute_batch(
            "CREATE TABLE belief_verifications (
                belief_id TEXT NOT NULL, label TEXT NOT NULL,
                query_type TEXT, query_text TEXT, expectation TEXT,
                last_status TEXT NOT NULL, last_result TEXT, last_error TEXT,
                last_run_at TEXT, data_freshness TEXT,
                PRIMARY KEY (belief_id, label)
            );
            INSERT INTO belief_verifications VALUES
                ('belief-a', 'check-1', 'sql', 'SELECT 0', '= 1', 'contested', '0', NULL, '2026-02-01', 'full'),
                ('belief-b', 'check-2', 'sql', 'SELECT 1', '= 1', 'pass', '1', NULL, '2026-02-01', 'full'),
                ('belief-c', 'check-3', 'sql', 'SELECT 1', '= 1', 'pass', '1', NULL, '2026-02-01', 'full');
            ",
        ).unwrap();

        // Query drifted belief IDs
        let drifted: Vec<String> = conn
            .prepare(
                "SELECT DISTINCT p.belief_id
                 FROM belief_verifications_prev p
                 JOIN belief_verifications c ON p.belief_id = c.belief_id AND p.label = c.label
                 WHERE p.last_status = 'pass' AND c.last_status != 'pass'",
            )
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(drifted, vec!["belief-a"]); // only pass→contested counts
    }

    #[test]
    fn test_attacked_by_parsing() {
        let content = r#"---
id: test-belief
---

# test-belief

Test statement.

## Attacked-By

- [[attacker-one]] (status: active, confidence: 0.3, scope: "narrow")
- [[attacker-two]] (status: defeated, confidence: 0.1)
- plain text without wikilink (should be skipped)
- [[attacker-three]]

## Attacks

- [[target-one]] (status: active)
- [[target-two]] (status: defeated)
- [[target-three]]
"#;
        let metrics = extract_file_metrics(content);

        // Attacked-By: attacker-two is defeated → excluded
        assert_eq!(
            metrics.attacked_by_ids,
            vec!["attacker-one", "attacker-three"]
        );
        assert_eq!(metrics.defeated_attacks, 1); // attacker-two

        // Attacks: target-two is defeated → excluded
        assert_eq!(metrics.attacks_ids, vec!["target-one", "target-three"]);
    }

    #[test]
    fn test_contested_bidirectional() {
        // A attacks B (both active) → B should be flagged as contested by A
        // B attacked-by A (both active) → same result, bidirectional merge
        let content_a = r#"---
id: belief-a
status: active
---

# belief-a

Statement A.

## Attacks

- [[belief-b]] (status: active)
"#;
        let content_b = r#"---
id: belief-b
status: active
---

# belief-b

Statement B.

## Attacked-By

- [[belief-a]] (status: active)
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let path_a = temp_dir.path().join("belief-a.md");
        let path_b = temp_dir.path().join("belief-b.md");
        std::fs::write(&path_a, content_a).unwrap();
        std::fs::write(&path_b, content_b).unwrap();

        let mut beliefs = vec![
            parse_belief_file(&path_a).unwrap(),
            parse_belief_file(&path_b).unwrap(),
        ];

        // Cross-reference (no sessions dir needed for this test)
        cross_reference_beliefs(&mut beliefs, temp_dir.path());

        // B should be contested by A (from both A's ## Attacks and B's ## Attacked-By)
        let b = &beliefs[1];
        assert!(
            b.metrics.contested_by.contains(&"belief-a".to_string()),
            "belief-b should be contested by belief-a, got {:?}",
            b.metrics.contested_by
        );

        // A is NOT contested by B (B attacks nothing, B's Attacked-By lists A not the reverse)
        let a = &beliefs[0];
        assert!(
            a.metrics.contested_by.is_empty(),
            "belief-a should not be contested, got {:?}",
            a.metrics.contested_by
        );
    }
}
