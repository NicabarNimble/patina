//! Scry subcommands (orient, recent, why, open, copy, feedback)
//!
//! Additional query modes and feedback loop actions.

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::retrieval::{QueryEngine, QueryOptions};
use patina::eventlog;

use super::enrichment::truncate_content;
use super::logging::{get_query_results, log_scry_feedback, log_scry_use};

// ============================================================================
// Scry Orient - Structural-first file ranking
// ============================================================================

/// Result from orient query
#[derive(Debug, Serialize)]
pub struct OrientEntry {
    pub path: String,
    pub composite_score: f64,
    pub importer_count: i64,
    pub activity_level: String,
    pub is_entry_point: bool,
    pub is_test_file: bool,
    pub commit_count: i64,
}

/// Full orient result with directory context
#[derive(Debug, Serialize)]
pub struct OrientOutput {
    pub directory: String,
    pub entries: Vec<OrientEntry>,
}

/// Query orient entries from database — shared between CLI and MCP
fn orient_entries(conn: &Connection, dir_path: &str, limit: usize) -> Result<Vec<OrientEntry>> {
    // Check if module_signals table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='module_signals'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !table_exists {
        anyhow::bail!("module_signals table not found. Run 'patina assay derive' first.");
    }

    // Normalize path for matching (strip trailing slash, ensure ./ prefix)
    let normalized_path = dir_path.trim_end_matches('/');
    let normalized_path = if normalized_path.starts_with("./") {
        normalized_path.to_string()
    } else {
        format!("./{}", normalized_path)
    };

    let sql = "
        SELECT
            path,
            COALESCE(is_entry_point, 0) * 20 +
            MIN(COALESCE(importer_count, 0) * 2, 20) +
            CASE COALESCE(activity_level, 'dormant')
                WHEN 'high' THEN 10
                WHEN 'medium' THEN 5
                WHEN 'low' THEN 2
                ELSE 0
            END +
            CASE
                WHEN COALESCE(commit_count, 0) > 50 THEN 10
                WHEN COALESCE(commit_count, 0) > 20 THEN 8
                WHEN COALESCE(commit_count, 0) > 5 THEN 5
                WHEN COALESCE(commit_count, 0) > 0 THEN 2
                ELSE 0
            END -
            COALESCE(is_test_file, 0) * 5
            AS composite_score,
            COALESCE(importer_count, 0),
            COALESCE(activity_level, 'unknown'),
            COALESCE(is_entry_point, 0),
            COALESCE(is_test_file, 0),
            COALESCE(commit_count, 0)
        FROM module_signals
        WHERE path LIKE ?
        ORDER BY composite_score DESC
        LIMIT ?
    ";

    let pattern = format!("{}%", normalized_path);
    let mut stmt = conn.prepare(sql)?;
    let results: Vec<OrientEntry> = stmt
        .query_map(rusqlite::params![pattern, limit as i64], |row| {
            Ok(OrientEntry {
                path: row.get(0)?,
                composite_score: row.get(1)?,
                importer_count: row.get(2)?,
                activity_level: row.get(3)?,
                is_entry_point: row.get::<_, i64>(4)? != 0,
                is_test_file: row.get::<_, i64>(5)? != 0,
                commit_count: row.get(6)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(results)
}

/// Query orient and return JSON string — called by MCP handler
pub fn orient_json(conn: &Connection, dir_path: &str, limit: usize) -> Result<String> {
    let entries = orient_entries(conn, dir_path, limit)?;
    let result = OrientOutput {
        directory: dir_path.to_string(),
        entries,
    };
    Ok(serde_json::to_string_pretty(&result)?)
}

/// Execute orient subcommand - rank files by structural importance
pub fn execute_orient(dir_path: &str, limit: usize) -> Result<()> {
    println!("🔮 Scry Orient - What's important in {}\n", dir_path);

    let conn = Connection::open(eventlog::PATINA_DB)
        .with_context(|| "Failed to open database. Run 'patina scrape' first.")?;

    let results = orient_entries(&conn, dir_path, limit)?;

    if results.is_empty() {
        println!("No files found in '{}' with structural signals.", dir_path);
        println!("\nHint: Run 'patina assay derive' to compute signals for all files.");
        return Ok(());
    }

    println!("Mode: Structural (file-level importance)\n");
    println!("Found {} files:\n", results.len());
    println!("{}", "─".repeat(70));

    for (i, result) in results.iter().enumerate() {
        let mut flags = Vec::new();
        if result.is_entry_point {
            flags.push("entry_point");
        }
        if result.is_test_file {
            flags.push("test");
        }
        let flags_str = if flags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", flags.join(", "))
        };

        println!(
            "\n[{}] {} (score: {:.0})",
            i + 1,
            result.path,
            result.composite_score
        );
        println!(
            "    {} importers | {} activity | {} commits{}",
            result.importer_count, result.activity_level, result.commit_count, flags_str
        );
    }

    println!("\n{}", "─".repeat(70));

    Ok(())
}

// ============================================================================
// Scry Recent - Temporal-first ranking
// ============================================================================

/// Recent change entry
#[derive(Debug, Serialize)]
pub struct RecentEntry {
    pub path: String,
    pub date: String,
    pub author: String,
    pub message_preview: String,
}

/// Full recent result with query context
#[derive(Debug, Serialize)]
pub struct RecentOutput {
    pub query: Option<String>,
    pub days: u32,
    pub entries: Vec<RecentEntry>,
}

/// Query recent entries from database — shared between CLI and MCP
fn recent_entries(
    conn: &Connection,
    query: Option<&str>,
    days: u32,
    limit: usize,
) -> Result<Vec<RecentEntry>> {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
    let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

    let sql = if query.is_some() {
        "SELECT cf.file_path, c.timestamp, c.message, c.author_name
         FROM commits c
         JOIN commit_files cf ON c.sha = cf.sha
         WHERE c.timestamp >= ? AND cf.file_path LIKE ?
         ORDER BY c.timestamp DESC
         LIMIT ?"
    } else {
        "SELECT cf.file_path, c.timestamp, c.message, c.author_name
         FROM commits c
         JOIN commit_files cf ON c.sha = cf.sha
         WHERE c.timestamp >= ?
         ORDER BY c.timestamp DESC
         LIMIT ?"
    };

    let mut stmt = conn.prepare(sql)?;
    let results: Vec<(String, String, String, String)> = if let Some(q) = query {
        let pattern = format!("%{}%", q);
        stmt.query_map(
            rusqlite::params![cutoff_str, pattern, limit as i64 * 3],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )?
        .filter_map(|r| r.ok())
        .collect()
    } else {
        stmt.query_map(rusqlite::params![cutoff_str, limit as i64 * 3], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect()
    };

    // Deduplicate by file path, keeping most recent
    let mut seen = std::collections::HashSet::new();
    let entries: Vec<RecentEntry> = results
        .into_iter()
        .filter(|(path, _, _, _)| seen.insert(path.clone()))
        .take(limit)
        .map(|(path, timestamp, message, author)| {
            let date = timestamp
                .split('T')
                .next()
                .unwrap_or(&timestamp)
                .to_string();
            let short_msg: String = message.chars().take(50).collect();
            let message_preview = if message.len() > 50 {
                format!("{}...", short_msg)
            } else {
                short_msg
            };
            RecentEntry {
                path,
                date,
                author,
                message_preview,
            }
        })
        .collect();

    Ok(entries)
}

/// Query recent changes and return JSON string — called by MCP handler
pub fn recent_json(
    conn: &Connection,
    query: Option<&str>,
    days: u32,
    limit: usize,
) -> Result<String> {
    let entries = recent_entries(conn, query, days, limit)?;
    let result = RecentOutput {
        query: query.map(|s| s.to_string()),
        days,
        entries,
    };
    Ok(serde_json::to_string_pretty(&result)?)
}

/// Execute recent subcommand - show recently changed files
pub fn execute_recent(query: Option<&str>, days: u32, limit: usize) -> Result<()> {
    println!(
        "🔮 Scry Recent - What changed{}\n",
        query
            .map(|q| format!(" related to '{}'", q))
            .unwrap_or_default()
    );

    let conn = Connection::open(eventlog::PATINA_DB)
        .with_context(|| "Failed to open database. Run 'patina scrape' first.")?;

    let entries = recent_entries(&conn, query, days, limit)?;

    if entries.is_empty() {
        println!("No changes found in the last {} days.", days);
        return Ok(());
    }

    println!("Mode: Temporal (last {} days)\n", days);
    println!("Found {} files with recent changes:\n", entries.len());
    println!("{}", "─".repeat(70));

    for (i, entry) in entries.iter().enumerate() {
        println!("\n[{}] {} ({})", i + 1, entry.path, entry.date);
        println!("    {} - {}", entry.author, entry.message_preview);
    }

    println!("\n{}", "─".repeat(70));

    Ok(())
}

// ============================================================================
// Scry Why - Explain single result
// ============================================================================

/// Why contribution entry
#[derive(Debug, Serialize)]
pub struct WhyContribution {
    pub oracle: String,
    pub rank: usize,
    pub raw_score: f64,
    pub score_type: String,
}

/// Why structural signals
#[derive(Debug, Serialize)]
pub struct WhySignals {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importer_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_level: Option<String>,
}

/// Why output
#[derive(Debug, Serialize)]
pub struct WhyOutput {
    pub doc_id: String,
    pub query: String,
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fused_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributions: Option<Vec<WhyContribution>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structural_signals: Option<WhySignals>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_results: Option<Vec<String>>,
}

/// Query why and return JSON string — called by MCP handler
pub fn why_json(engine: &QueryEngine, doc_id: &str, query: &str) -> Result<String> {
    let options = QueryOptions::default();
    let results = engine.query_with_options(query, 50, &options)?;

    let matching = results
        .iter()
        .find(|r| r.doc_id == doc_id || r.doc_id.ends_with(doc_id) || doc_id.ends_with(&r.doc_id));

    let output = match matching {
        Some(result) => {
            let rank = results
                .iter()
                .position(|r| r.doc_id == result.doc_id)
                .unwrap_or(0)
                + 1;

            let contributions: Vec<WhyContribution> = result
                .contributions
                .iter()
                .map(|(oracle_name, contrib)| WhyContribution {
                    oracle: oracle_name.to_string(),
                    rank: contrib.rank,
                    raw_score: contrib.raw_score as f64,
                    score_type: contrib.score_type.to_string(),
                })
                .collect();

            let ann = &result.annotations;
            let structural_signals = if ann.importer_count.is_some() || ann.activity_level.is_some()
            {
                Some(WhySignals {
                    importer_count: ann.importer_count,
                    activity_level: ann.activity_level.clone(),
                })
            } else {
                None
            };

            WhyOutput {
                doc_id: result.doc_id.clone(),
                query: query.to_string(),
                found: true,
                rank: Some(rank),
                fused_score: Some(result.fused_score as f64),
                contributions: Some(contributions),
                structural_signals,
                top_results: None,
            }
        }
        None => {
            let top: Vec<String> = results.iter().take(5).map(|r| r.doc_id.clone()).collect();
            WhyOutput {
                doc_id: doc_id.to_string(),
                query: query.to_string(),
                found: false,
                rank: None,
                fused_score: None,
                contributions: None,
                structural_signals: None,
                top_results: Some(top),
            }
        }
    };

    Ok(serde_json::to_string_pretty(&output)?)
}

/// Execute why subcommand - explain why a result was returned
pub fn execute_why(doc_id: &str, query: &str) -> Result<()> {
    println!("🔮 Scry Why - Explaining '{}'\n", doc_id);
    println!("Query: \"{}\"\n", query);

    let engine = QueryEngine::new();
    let options = QueryOptions::default();

    // Run the query to get full results with contributions
    let results = engine.query_with_options(query, 50, &options)?;

    // Find the specific doc_id in results
    let matching = results
        .iter()
        .find(|r| r.doc_id == doc_id || r.doc_id.ends_with(doc_id) || doc_id.ends_with(&r.doc_id));

    match matching {
        Some(result) => {
            println!(
                "Found in results at rank {}:\n",
                results
                    .iter()
                    .position(|r| r.doc_id == result.doc_id)
                    .unwrap_or(0)
                    + 1
            );
            println!("{}", "─".repeat(60));

            println!("\n**Document:** {}", result.doc_id);
            println!("**Fused Score:** {:.4}", result.fused_score);
            println!(
                "**Type:** {}",
                result.metadata.event_type.as_deref().unwrap_or("unknown")
            );

            println!("\n## Oracle Contributions\n");

            for (oracle_name, contrib) in &result.contributions {
                let score_display = match contrib.score_type {
                    "co_change_count" => format!("{} co-changes", contrib.raw_score as i32),
                    "bm25" => format!("{:.2} BM25", contrib.raw_score),
                    "cosine" => format!("{:.3} cosine", contrib.raw_score),
                    _ => format!("{:.3} {}", contrib.raw_score, contrib.score_type),
                };

                println!(
                    "- **{}**: rank #{} ({})",
                    oracle_name, contrib.rank, score_display
                );

                if let Some(ref matches) = contrib.matches {
                    if !matches.is_empty() {
                        println!("  - Matched terms: {}", matches.join(", "));
                    }
                }
            }

            // Show structural annotations if available
            let ann = &result.annotations;
            if ann.importer_count.is_some() || ann.activity_level.is_some() {
                println!("\n## Structural Signals\n");
                if let Some(count) = ann.importer_count {
                    println!("- Importers: {}", count);
                }
                if let Some(ref level) = ann.activity_level {
                    println!("- Activity: {}", level);
                }
                if let Some(true) = ann.is_entry_point {
                    println!("- Entry point: yes");
                }
                if let Some(true) = ann.is_test_file {
                    println!("- Test file: yes");
                }
            }

            println!("\n## Content Preview\n");
            println!("{}", truncate_content(&result.content, 300));

            println!("\n{}", "─".repeat(60));
        }
        None => {
            println!("'{}' not found in top 50 results for this query.", doc_id);
            println!("\nTop 5 results were:");
            for (i, r) in results.iter().take(5).enumerate() {
                println!("  {}. {}", i + 1, r.doc_id);
            }
        }
    }

    Ok(())
}

// ============================================================================
// Scry Open - Open file and log usage (Phase 3)
// ============================================================================

/// Execute open subcommand - open a file from query results and log usage
///
/// From spec-observable-scry.md:
/// - Opens file/location, logs usage automatically
/// - Automatic capture (no user effort required)
pub fn execute_open(query_id: &str, rank: usize) -> Result<()> {
    println!(
        "🔮 Scry Open - Opening result #{} from {}\n",
        rank, query_id
    );

    // Get the results from the query
    let results = get_query_results(query_id)
        .with_context(|| format!("Query '{}' not found in eventlog", query_id))?;

    if rank == 0 || rank > results.len() {
        anyhow::bail!(
            "Invalid rank {}. Query had {} results (1-{})",
            rank,
            results.len(),
            results.len()
        );
    }

    let (doc_id, _score) = &results[rank - 1];

    // Log usage before opening
    log_scry_use(query_id, doc_id, rank);

    // Extract file path from doc_id (may be "file:function" or just "file")
    let file_path = if doc_id.contains(':') {
        doc_id.split(':').next().unwrap_or(doc_id)
    } else {
        doc_id.as_str()
    };

    // Check if file exists
    if !std::path::Path::new(file_path).exists() {
        println!("File not found: {}", file_path);
        println!("(Usage logged for feedback analysis)");
        return Ok(());
    }

    // Open the file using the system's default handler
    println!("Opening: {}", file_path);
    println!("Usage logged: {} rank #{}", query_id, rank);

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(file_path)
            .spawn()
            .with_context(|| format!("Failed to open {}", file_path))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(file_path)
            .spawn()
            .with_context(|| format!("Failed to open {}", file_path))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", file_path])
            .spawn()
            .with_context(|| format!("Failed to open {}", file_path))?;
    }

    Ok(())
}

// ============================================================================
// Scry Copy - Copy to clipboard and log usage (Phase 3)
// ============================================================================

/// Execute copy subcommand - copy doc_id to clipboard and log usage
///
/// From spec-observable-scry.md:
/// - Copies to clipboard, logs usage automatically
/// - Automatic capture (no user effort required)
pub fn execute_copy(query_id: &str, rank: usize) -> Result<()> {
    println!(
        "🔮 Scry Copy - Copying result #{} from {}\n",
        rank, query_id
    );

    // Get the results from the query
    let results = get_query_results(query_id)
        .with_context(|| format!("Query '{}' not found in eventlog", query_id))?;

    if rank == 0 || rank > results.len() {
        anyhow::bail!(
            "Invalid rank {}. Query had {} results (1-{})",
            rank,
            results.len(),
            results.len()
        );
    }

    let (doc_id, _score) = &results[rank - 1];

    // Log usage before copying
    log_scry_use(query_id, doc_id, rank);

    // Copy to clipboard using platform-specific command
    let copy_result = copy_to_clipboard(doc_id);

    match copy_result {
        Ok(()) => {
            println!("Copied to clipboard: {}", doc_id);
            println!("Usage logged: {} rank #{}", query_id, rank);
        }
        Err(e) => {
            println!("Failed to copy to clipboard: {}", e);
            println!("Document ID: {}", doc_id);
            println!("(Usage still logged for feedback analysis)");
        }
    }

    Ok(())
}

/// Copy text to system clipboard
fn copy_to_clipboard(text: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        let mut child = std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }
        child.wait()?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use std::io::Write;
        // Try xclip first, then xsel
        let result = std::process::Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn();

        let mut child = match result {
            Ok(c) => c,
            Err(_) => std::process::Command::new("xsel")
                .args(["--clipboard", "--input"])
                .stdin(std::process::Stdio::piped())
                .spawn()?,
        };

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }
        child.wait()?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        use std::io::Write;
        let mut child = std::process::Command::new("cmd")
            .args(["/C", "clip"])
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }
        child.wait()?;
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        anyhow::bail!("Clipboard not supported on this platform");
    }
}

// ============================================================================
// Scry Feedback - Explicit rating (Phase 3)
// ============================================================================

/// Execute feedback subcommand - record explicit good/bad rating
///
/// From spec-observable-scry.md:
/// - Optional explicit feedback (rare, but valuable)
/// - Manual feedback supplements automatic usage capture
pub fn execute_feedback(query_id: &str, signal: &str, comment: Option<&str>) -> Result<()> {
    println!(
        "🔮 Scry Feedback - Recording '{}' for {}\n",
        signal, query_id
    );

    // Validate signal
    if signal != "good" && signal != "bad" {
        anyhow::bail!("Signal must be 'good' or 'bad', got '{}'", signal);
    }

    // Verify query exists
    let _ = get_query_results(query_id)
        .with_context(|| format!("Query '{}' not found in eventlog", query_id))?;

    // Log the feedback
    log_scry_feedback(query_id, signal, comment);

    println!("Feedback recorded: {} = {}", query_id, signal);
    if let Some(c) = comment {
        println!("Comment: {}", c);
    }

    Ok(())
}
