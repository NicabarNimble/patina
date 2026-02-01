//! Belief audit command — show computed use/truth metrics for all beliefs
//!
//! Reads from the `beliefs` table (computed by `patina scrape`) and displays
//! real metrics instead of fabricated confidence scores.

use anyhow::Result;
use clap::Subcommand;
use rusqlite::Connection;
use std::path::Path;

use super::scrape::database;

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
    },
}

pub fn execute(command: Option<BeliefCommands>) -> Result<()> {
    let cmd = command.unwrap_or(BeliefCommands::Audit {
        sort: "use".to_string(),
        warnings_only: false,
    });

    match cmd {
        BeliefCommands::Audit {
            sort,
            warnings_only,
        } => run_audit(&sort, warnings_only),
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
        warnings
    }
}

fn run_audit(sort_by: &str, warnings_only: bool) -> Result<()> {
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
        _ => "(cited_by_beliefs + cited_by_sessions) DESC, evidence_count DESC", // "use" default
    };

    // Check if verification columns exist (migration may not have run yet)
    let has_verification = conn
        .prepare("SELECT verification_total FROM beliefs LIMIT 1")
        .is_ok();

    let sql = if has_verification {
        format!(
            "SELECT id, entrenchment, cited_by_beliefs, cited_by_sessions, applied_in,
                    evidence_count, evidence_verified, defeated_attacks,
                    verification_total, verification_passed, verification_failed, verification_errored
             FROM beliefs
             ORDER BY {}",
            order_clause
        )
    } else {
        format!(
            "SELECT id, entrenchment, cited_by_beliefs, cited_by_sessions, applied_in,
                    evidence_count, evidence_verified, defeated_attacks
             FROM beliefs
             ORDER BY {}",
            order_clause
        )
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<BeliefRow> = stmt
        .query_map([], |row| {
            Ok(BeliefRow {
                id: row.get(0)?,
                entrenchment: row.get(1)?,
                cited_by_beliefs: row.get(2)?,
                cited_by_sessions: row.get(3)?,
                applied_in: row.get(4)?,
                evidence_count: row.get(5)?,
                evidence_verified: row.get(6)?,
                defeated_attacks: row.get(7)?,
                verification_total: if has_verification { row.get(8)? } else { 0 },
                verification_passed: if has_verification { row.get(9)? } else { 0 },
                verification_failed: if has_verification { row.get(10)? } else { 0 },
                verification_errored: if has_verification { row.get(11)? } else { 0 },
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
        "  {:<36} {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>5} {:>9} {}",
        "BELIEF", "B-USE", "S-USE", "EVID", "VERI", "DEFT", "APPL", "V-OK", "ENTRENCH", "WARNINGS"
    );
    println!(
        "  {:<36} {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>5} {:>9} {}",
        "──────",
        "─────",
        "─────",
        "────",
        "────",
        "────",
        "────",
        "─────",
        "─────────",
        "────────"
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
            "  {:<36} {:>5} {:>5} {:>4} {:>4} {:>4} {:>4} {:>5} {:>9} {}",
            display_id,
            row.cited_by_beliefs,
            row.cited_by_sessions,
            row.evidence_count,
            row.evidence_verified,
            row.defeated_attacks,
            row.applied_in,
            row.v_ok_display(),
            row.entrenchment,
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
        if total_failed > 0 {
            println!("    {} beliefs with contested verification", total_failed);
        }
        if total_errored > 0 {
            println!("    {} beliefs with verification errors", total_errored);
        }
    }
    println!();

    Ok(())
}
