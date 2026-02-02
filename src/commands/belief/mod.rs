//! Belief audit command — show computed use/truth metrics for all beliefs
//!
//! Reads from the `beliefs` table (computed by `patina scrape`) and displays
//! real metrics instead of fabricated confidence scores.
//!
//! E4.6a: --grounding flag computes semantic grounding from usearch embeddings.

use anyhow::{Context, Result};
use clap::Subcommand;
use rusqlite::Connection;
use std::path::Path;
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

use super::scrape::database;
use super::scry::internal::enrichment::{enrich_results, SearchResults};

#[derive(Subcommand, Debug)]
pub enum BeliefCommands {
    /// Show all beliefs ranked by use/truth metrics (default)
    Audit {
        /// Sort by: "use" (default), "truth", "weak"
        #[arg(long, default_value = "use")]
        sort: String,

        /// Show only beliefs with warnings
        #[arg(long)]
        warnings_only: bool,

        /// Show semantic grounding — nearest code/commits/sessions for each belief (E4.6a)
        #[arg(long)]
        grounding: bool,
    },
}

pub fn execute(command: Option<BeliefCommands>) -> Result<()> {
    let cmd = command.unwrap_or(BeliefCommands::Audit {
        sort: "use".to_string(),
        warnings_only: false,
        grounding: false,
    });

    match cmd {
        BeliefCommands::Audit {
            sort,
            warnings_only,
            grounding,
        } => run_audit(&sort, warnings_only, grounding),
    }
}

struct BeliefRow {
    id: String,
    entrenchment: String,
    cited_by_beliefs: i32,
    cited_by_sessions: i32,
    applied_in: i32,
    evidence_count: i32,
    evidence_verified: i32,
    defeated_attacks: i32,
    verification_total: i32,
    verification_passed: i32,
    verification_failed: i32,
    verification_errored: i32,
    // E4.6a: Semantic grounding
    grounding_score: f32,
    grounding_code_count: i32,
    grounding_commit_count: i32,
    grounding_session_count: i32,
}

impl BeliefRow {
    fn total_use(&self) -> i32 {
        self.cited_by_beliefs + self.cited_by_sessions
    }

    fn v_ok_display(&self) -> String {
        if self.verification_total == 0 {
            "\u{2014}".to_string() // em dash
        } else {
            format!("{}/{}", self.verification_passed, self.verification_total)
        }
    }

    fn grounding_total(&self) -> i32 {
        self.grounding_code_count + self.grounding_commit_count + self.grounding_session_count
    }

    fn grounding_display(&self) -> String {
        if self.grounding_total() == 0 {
            "\u{2014}".to_string() // em dash
        } else {
            format!(
                "{}c{}m{}s",
                self.grounding_code_count,
                self.grounding_commit_count,
                self.grounding_session_count
            )
        }
    }

    fn health_warnings(&self) -> Vec<&'static str> {
        let mut warnings = Vec::new();
        if self.evidence_count == 0 {
            warnings.push("no-evidence");
        }
        if self.evidence_verified == 0 && self.evidence_count > 0 {
            warnings.push("unverified");
        }
        if self.total_use() == 0 {
            warnings.push("unused");
        }
        if self.applied_in == 0 {
            warnings.push("no-applications");
        }
        if self.verification_failed > 0 {
            warnings.push("verify-contested");
        }
        if self.verification_errored > 0 {
            warnings.push("verify-error");
        }
        if self.grounding_total() == 0 && self.grounding_score == 0.0 {
            warnings.push("floating");
        }
        warnings
    }
}

fn run_audit(sort_by: &str, warnings_only: bool, show_grounding: bool) -> Result<()> {
    let db_path = Path::new(database::PATINA_DB);
    if !db_path.exists() {
        anyhow::bail!("No database found. Run `patina scrape` first.");
    }

    let conn = Connection::open(db_path)?;

    // Check if metric columns exist
    let has_metrics = conn
        .prepare("SELECT cited_by_beliefs FROM beliefs LIMIT 1")
        .is_ok();

    if !has_metrics {
        anyhow::bail!(
            "Belief metrics not computed yet. Run `patina scrape --rebuild` to compute use/truth metrics."
        );
    }

    let order_clause = match sort_by {
        "truth" => "evidence_count DESC, evidence_verified DESC",
        "weak" => "(cited_by_beliefs + cited_by_sessions) ASC, evidence_count ASC",
        "grounding" => "grounding_score DESC, (grounding_code_count + grounding_commit_count + grounding_session_count) DESC",
        _ => "(cited_by_beliefs + cited_by_sessions) DESC, evidence_count DESC", // "use" default
    };

    // Check if verification columns exist (migration may not have run yet)
    let has_verification = conn
        .prepare("SELECT verification_total FROM beliefs LIMIT 1")
        .is_ok();

    // Check if grounding columns exist
    let has_grounding = conn
        .prepare("SELECT grounding_score FROM beliefs LIMIT 1")
        .is_ok();

    let sql = format!(
        "SELECT id, entrenchment, cited_by_beliefs, cited_by_sessions, applied_in,
                evidence_count, evidence_verified, defeated_attacks{}{}
         FROM beliefs
         ORDER BY {}",
        if has_verification {
            ", verification_total, verification_passed, verification_failed, verification_errored"
        } else {
            ""
        },
        if has_grounding {
            ", grounding_score, grounding_code_count, grounding_commit_count, grounding_session_count"
        } else {
            ""
        },
        order_clause
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<BeliefRow> = stmt
        .query_map([], |row| {
            let base_idx = 8; // 0-7 are always present
            let v_offset = base_idx;
            let g_offset = if has_verification { v_offset + 4 } else { v_offset };

            Ok(BeliefRow {
                id: row.get(0)?,
                entrenchment: row.get(1)?,
                cited_by_beliefs: row.get(2)?,
                cited_by_sessions: row.get(3)?,
                applied_in: row.get(4)?,
                evidence_count: row.get(5)?,
                evidence_verified: row.get(6)?,
                defeated_attacks: row.get(7)?,
                verification_total: if has_verification { row.get(v_offset)? } else { 0 },
                verification_passed: if has_verification { row.get(v_offset + 1)? } else { 0 },
                verification_failed: if has_verification { row.get(v_offset + 2)? } else { 0 },
                verification_errored: if has_verification { row.get(v_offset + 3)? } else { 0 },
                grounding_score: if has_grounding { row.get(g_offset)? } else { 0.0 },
                grounding_code_count: if has_grounding { row.get(g_offset + 1)? } else { 0 },
                grounding_commit_count: if has_grounding { row.get(g_offset + 2)? } else { 0 },
                grounding_session_count: if has_grounding { row.get(g_offset + 3)? } else { 0 },
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    if rows.is_empty() {
        println!("No beliefs found. Create beliefs in layer/surface/epistemic/beliefs/");
        return Ok(());
    }

    // Filter if warnings_only
    let display_rows: Vec<&BeliefRow> = if warnings_only {
        rows.iter()
            .filter(|r| !r.health_warnings().is_empty())
            .collect()
    } else {
        rows.iter().collect()
    };

    // Print header
    println!(
        "\n  Belief Audit — {} beliefs (sorted by {})\n",
        rows.len(),
        sort_by
    );
    println!(
        "  {:<36} {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>5} {:>9} {:>7} WARNINGS",
        "BELIEF", "B-USE", "S-USE", "EVID", "VERI", "DEFT", "APPL", "V-OK", "ENTRENCH", "GROUND"
    );
    println!(
        "  {:<36} {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>5} {:>9} {:>7} ────────",
        "──────", "─────", "─────", "────", "────", "────", "────", "─────", "─────────", "───────"
    );

    let mut warning_count = 0;
    for row in &display_rows {
        let warnings = row.health_warnings();
        if !warnings.is_empty() {
            warning_count += 1;
        }
        let warning_str = if warnings.is_empty() {
            String::new()
        } else {
            warnings.join(", ")
        };

        // Truncate ID for display
        let display_id = if row.id.len() > 35 {
            format!("{}…", &row.id[..34])
        } else {
            row.id.clone()
        };

        println!(
            "  {:<36} {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>5} {:>9} {:>7} {}",
            display_id,
            row.cited_by_beliefs,
            row.cited_by_sessions,
            row.evidence_count,
            row.evidence_verified,
            row.defeated_attacks,
            row.applied_in,
            row.v_ok_display(),
            row.entrenchment,
            row.grounding_display(),
            warning_str,
        );
    }

    // Summary
    let total_use: i32 = rows.iter().map(|r| r.total_use()).sum();
    let total_evidence: i32 = rows.iter().map(|r| r.evidence_count).sum();
    let total_verified: i32 = rows.iter().map(|r| r.evidence_verified).sum();
    let with_no_evidence: usize = rows.iter().filter(|r| r.evidence_count == 0).count();
    let with_unverified: usize = rows
        .iter()
        .filter(|r| r.evidence_verified == 0 && r.evidence_count > 0)
        .count();
    let unused: usize = rows.iter().filter(|r| r.total_use() == 0).count();

    // Verification stats
    let beliefs_with_queries: usize = rows.iter().filter(|r| r.verification_total > 0).count();
    let total_queries: i32 = rows.iter().map(|r| r.verification_total).sum();
    let total_passed: i32 = rows.iter().map(|r| r.verification_passed).sum();
    let total_failed: i32 = rows.iter().map(|r| r.verification_failed).sum();
    let total_errored: i32 = rows.iter().map(|r| r.verification_errored).sum();

    // Grounding stats
    let grounded: usize = rows.iter().filter(|r| r.grounding_total() > 0).count();
    let floating: usize = rows.len() - grounded;

    println!("\n  ── Summary ──");
    println!("  Total beliefs: {}", rows.len());
    println!(
        "  Total citations: {} ({} by beliefs, {} by sessions)",
        total_use,
        rows.iter().map(|r| r.cited_by_beliefs).sum::<i32>(),
        rows.iter().map(|r| r.cited_by_sessions).sum::<i32>()
    );
    println!(
        "  Evidence: {} total, {} verified ({:.0}%)",
        total_evidence,
        total_verified,
        if total_evidence > 0 {
            total_verified as f64 / total_evidence as f64 * 100.0
        } else {
            0.0
        }
    );
    if total_queries > 0 {
        println!(
            "  Verification: {} queries across {} beliefs ({} passed, {} contested, {} errors)",
            total_queries, beliefs_with_queries, total_passed, total_failed, total_errored
        );
    }
    if grounded > 0 || floating > 0 {
        println!(
            "  Grounding: {} grounded, {} floating",
            grounded, floating
        );
    }
    if warning_count > 0 {
        println!("\n  Warnings: {}", warning_count);
        if with_no_evidence > 0 {
            println!("    {} beliefs with no evidence", with_no_evidence);
        }
        if with_unverified > 0 {
            println!("    {} beliefs with unverified evidence", with_unverified);
        }
        if unused > 0 {
            println!("    {} beliefs with no citations", unused);
        }
        if floating > 0 {
            println!("    {} beliefs floating (no code/commit/session grounding)", floating);
        }
        if total_failed > 0 {
            println!("    {} beliefs with contested verification", total_failed);
        }
        if total_errored > 0 {
            println!("    {} beliefs with verification errors", total_errored);
        }
    }
    println!();

    // E4.6a: Semantic grounding report
    if show_grounding {
        run_grounding_report(&conn, &rows)?;
    }

    Ok(())
}

/// Compute and display semantic grounding for each belief (E4.6a)
///
/// Uses the usearch semantic index to find each belief's nearest neighbors
/// across all content types. Shows what code, commits, and sessions each
/// belief is semantically connected to.
fn run_grounding_report(conn: &Connection, rows: &[BeliefRow]) -> Result<()> {
    // Get embeddings path
    let model = crate::commands::scry::internal::search::get_embedding_model();
    let index_path = format!(
        ".patina/local/data/embeddings/{}/projections/semantic.usearch",
        model
    );

    if !Path::new(&index_path).exists() {
        println!("  Grounding: semantic index not found. Run `patina oxidize` first.\n");
        return Ok(());
    }

    // Load usearch index
    let index_options = IndexOptions {
        dimensions: 256,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };

    let index = Index::new(&index_options).context("Failed to create index")?;
    index
        .load(&index_path)
        .context("Failed to load semantic index")?;

    const BELIEF_ID_OFFSET: i64 = 4_000_000_000;
    const CODE_ID_OFFSET: i64 = 1_000_000_000;
    const PATTERN_ID_OFFSET: i64 = 2_000_000_000;
    const COMMIT_ID_OFFSET: i64 = 3_000_000_000;
    const GROUNDING_LIMIT: usize = 20; // Search this many neighbors
    const DISPLAY_LIMIT: usize = 3; // Show top 3 per type

    println!("  ── Semantic Grounding (E4.6a) ──\n");

    let mut grounded_count = 0;
    let mut floating_count = 0;

    for row in rows {
        // Look up belief's rowid
        let rowid: Result<i64, _> = conn.query_row(
            "SELECT rowid FROM beliefs WHERE id = ?",
            [&row.id],
            |r| r.get(0),
        );

        let rowid = match rowid {
            Ok(r) => r,
            Err(_) => continue,
        };

        let belief_key = (BELIEF_ID_OFFSET + rowid) as u64;

        // Get belief's vector
        let mut vector = vec![0.0_f32; 256];
        if index.get(belief_key, &mut vector).is_err() {
            continue;
        }

        // Check for zero vector (not in index)
        let magnitude: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if magnitude < 0.001 {
            continue;
        }

        // Search for neighbors
        let matches = match index.search(&vector, GROUNDING_LIMIT + 2) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let results = SearchResults {
            keys: matches.keys,
            distances: matches.distances,
        };

        let enriched = match enrich_results(conn, &results, "semantic", 0.0) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Filter out self entries and categorize
        let mut code_results = Vec::new();
        let mut commit_results = Vec::new();
        let mut session_results = Vec::new();

        for r in &enriched {
            if r.source_id == row.id
                && (r.event_type == "belief.surface" || r.event_type.starts_with("pattern."))
            {
                continue; // Skip self
            }

            let key = r.id;
            if key >= CODE_ID_OFFSET && key < PATTERN_ID_OFFSET {
                code_results.push(r);
            } else if key >= COMMIT_ID_OFFSET && key < BELIEF_ID_OFFSET {
                commit_results.push(r);
            } else if key < CODE_ID_OFFSET {
                session_results.push(r);
            }
        }

        let has_grounding =
            !code_results.is_empty() || !commit_results.is_empty() || !session_results.is_empty();

        if has_grounding {
            grounded_count += 1;
        } else {
            floating_count += 1;
        }

        // Display
        let display_id = if row.id.len() > 35 {
            format!("{}…", &row.id[..34])
        } else {
            row.id.clone()
        };

        println!(
            "  {} ({}c {}m {}s)",
            display_id,
            code_results.len(),
            commit_results.len(),
            session_results.len()
        );

        // Show top code neighbors
        for r in code_results.iter().take(DISPLAY_LIMIT) {
            println!("    code  {:.3}  {}", r.score, truncate(&r.source_id, 60));
        }
        for r in commit_results.iter().take(DISPLAY_LIMIT) {
            println!(
                "    commit {:.3}  {}",
                r.score,
                truncate(&r.content, 60)
            );
        }
        for r in session_results.iter().take(DISPLAY_LIMIT) {
            println!(
                "    session {:.3} {}",
                r.score,
                truncate(&r.content, 55)
            );
        }

        if has_grounding {
            println!();
        } else {
            println!("    (floating — no code/commit/session neighbors)\n");
        }
    }

    println!(
        "  ── Grounding Summary: {} grounded, {} floating ──\n",
        grounded_count, floating_count
    );

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max - 1).collect();
    format!("{}…", truncated)
}
