//! Evaluation framework for validating retrieval quality
//!
//! Phase 2.5c: Measure dimension value and query interface effectiveness.
//!
//! Key question: "Which query interfaces work for which dimensions?"

use anyhow::Result;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

use crate::commands::scry::{scry, ScryOptions};

/// Evaluation results
#[derive(Debug)]
pub struct EvalResults {
    pub dimension: String,
    pub query_type: String,
    pub num_queries: usize,
    pub precision_at_5: f32,
    pub precision_at_10: f32,
    pub random_baseline: f32,
}

/// Run evaluation
pub fn execute(dimension: Option<String>) -> Result<()> {
    println!("📊 Evaluation Framework\n");
    println!("Testing retrieval quality for each dimension...\n");

    let db_path = ".patina/data/patina.db";
    let conn = Connection::open(db_path)?;

    let mut all_results = Vec::new();

    // Evaluate semantic dimension
    if dimension.is_none() || dimension.as_deref() == Some("semantic") {
        println!("━━━ Semantic Dimension (text → text) ━━━\n");
        let results = eval_semantic(&conn)?;
        print_results(&results);
        all_results.push(results);
    }

    // Evaluate temporal dimension (text queries - expected to be poor)
    if dimension.is_none() || dimension.as_deref() == Some("temporal") {
        println!("\n━━━ Temporal Dimension (text → files) ━━━\n");
        let results = eval_temporal_text(&conn)?;
        print_results(&results);
        all_results.push(results);

        println!("\n━━━ Temporal Dimension (file → files) ━━━\n");
        let results = eval_temporal_file(&conn)?;
        print_results(&results);
        all_results.push(results);
    }

    // Summary
    println!("\n━━━ Summary ━━━\n");
    println!(
        "{:<30} {:>12} {:>12} {:>12}",
        "Dimension/Query", "P@5", "P@10", "vs Random"
    );
    println!("{}", "─".repeat(70));
    for r in &all_results {
        let vs_random = if r.random_baseline > 0.0 {
            r.precision_at_10 / r.random_baseline
        } else {
            0.0
        };
        println!(
            "{:<30} {:>11.1}% {:>11.1}% {:>11.1}x",
            format!("{} ({})", r.dimension, r.query_type),
            r.precision_at_5 * 100.0,
            r.precision_at_10 * 100.0,
            vs_random
        );
    }

    Ok(())
}

/// Evaluate semantic dimension: text → text
///
/// Strategy: For observations from a session, use one as query,
/// check if other observations from same session are retrieved.
fn eval_semantic(conn: &Connection) -> Result<EvalResults> {
    // Get sessions with multiple observations
    let mut sessions: HashMap<String, Vec<(i64, String)>> = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT source_id, seq, json_extract(data, '$.content') as content
         FROM eventlog
         WHERE event_type IN ('session.decision', 'session.observation', 'session.pattern')
           AND content IS NOT NULL AND length(content) > 50
         ORDER BY source_id, seq",
    )?;

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let session_id: String = row.get(0)?;
        let seq: i64 = row.get(1)?;
        let content: String = row.get(2)?;
        sessions.entry(session_id).or_default().push((seq, content));
    }

    // Filter to sessions with 3+ observations (need query + expected results)
    let valid_sessions: Vec<_> = sessions.iter().filter(|(_, obs)| obs.len() >= 3).collect();

    println!(
        "Found {} sessions with 3+ observations",
        valid_sessions.len()
    );

    let mut total_precision_5 = 0.0;
    let mut total_precision_10 = 0.0;
    let mut num_queries = 0;

    // Sample up to 20 sessions for evaluation
    let sample_size = valid_sessions.len().min(20);
    let mut rng = fastrand::Rng::new();

    for i in 0..sample_size {
        let idx = if sample_size < valid_sessions.len() {
            rng.usize(..valid_sessions.len())
        } else {
            i
        };

        let (session_id, observations) = valid_sessions[idx];

        // Use first observation as query
        let query = &observations[0].1;
        let expected_seqs: HashSet<i64> =
            observations.iter().skip(1).map(|(seq, _)| *seq).collect();

        // Run scry
        let options = ScryOptions {
            limit: 10,
            min_score: 0.0,
            dimension: Some("semantic".to_string()),
            file: None,
            repo: None,
            all_repos: false,
            include_issues: false,
            include_persona: false, // Eval doesn't need persona
            hybrid: false,
        };

        if let Ok(results) = scry(query, &options) {
            let retrieved_seqs: Vec<i64> = results.iter().map(|r| r.id).collect();

            // Calculate precision@5 and precision@10
            let hits_5 = retrieved_seqs
                .iter()
                .take(5)
                .filter(|id| expected_seqs.contains(id))
                .count();
            let hits_10 = retrieved_seqs
                .iter()
                .take(10)
                .filter(|id| expected_seqs.contains(id))
                .count();

            let p5 = hits_5 as f32 / 5.0_f32.min(expected_seqs.len() as f32);
            let p10 = hits_10 as f32 / 10.0_f32.min(expected_seqs.len() as f32);

            total_precision_5 += p5;
            total_precision_10 += p10;
            num_queries += 1;

            if num_queries <= 3 {
                println!(
                    "  Query from {}: P@5={:.0}%, P@10={:.0}%",
                    session_id,
                    p5 * 100.0,
                    p10 * 100.0
                );
            }
        }
    }

    if num_queries > 3 {
        println!("  ... and {} more queries", num_queries - 3);
    }

    // Random baseline: chance of hitting same-session observation
    let total_observations: usize = sessions.values().map(|v| v.len()).sum();
    let avg_session_size = total_observations as f32 / sessions.len() as f32;
    let random_baseline = avg_session_size / total_observations as f32;

    Ok(EvalResults {
        dimension: "semantic".to_string(),
        query_type: "text→text".to_string(),
        num_queries,
        precision_at_5: if num_queries > 0 {
            total_precision_5 / num_queries as f32
        } else {
            0.0
        },
        precision_at_10: if num_queries > 0 {
            total_precision_10 / num_queries as f32
        } else {
            0.0
        },
        random_baseline,
    })
}

/// Evaluate temporal dimension with text queries (expected poor)
fn eval_temporal_text(conn: &Connection) -> Result<EvalResults> {
    // Use session observation text as queries against temporal index
    // This should perform poorly since temporal was trained on file relationships

    let mut stmt = conn.prepare(
        "SELECT json_extract(data, '$.content') as content
         FROM eventlog
         WHERE event_type = 'session.observation'
           AND content IS NOT NULL AND length(content) > 50
         LIMIT 20",
    )?;

    let mut queries: Vec<String> = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        queries.push(row.get(0)?);
    }

    println!(
        "Testing {} text queries against temporal index",
        queries.len()
    );

    // For temporal with text queries, there's no "correct" answer
    // We measure if results are even meaningful by checking score distribution
    let mut avg_top_score = 0.0;
    let mut avg_score_variance = 0.0;
    let mut num_queries = 0;

    for query in queries.iter().take(10) {
        let options = ScryOptions {
            limit: 10,
            min_score: 0.0,
            dimension: Some("temporal".to_string()),
            file: None,
            repo: None,
            all_repos: false,
            include_issues: false,
            include_persona: false,
            hybrid: false,
        };

        if let Ok(results) = scry(query, &options) {
            if !results.is_empty() {
                let scores: Vec<f32> = results.iter().map(|r| r.score).collect();
                let top = scores[0];
                let mean = scores.iter().sum::<f32>() / scores.len() as f32;
                let variance =
                    scores.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / scores.len() as f32;

                avg_top_score += top;
                avg_score_variance += variance;
                num_queries += 1;
            }
        }
    }

    if num_queries > 0 {
        avg_top_score /= num_queries as f32;
        avg_score_variance /= num_queries as f32;
    }

    println!("  Avg top score: {:.3}", avg_top_score);
    println!(
        "  Avg score variance: {:.4} (low = results are random-ish)",
        avg_score_variance
    );

    // Without ground truth for text→file, precision is undefined
    // Report 0 to indicate "not applicable"
    Ok(EvalResults {
        dimension: "temporal".to_string(),
        query_type: "text→files".to_string(),
        num_queries,
        precision_at_5: 0.0, // N/A - no ground truth
        precision_at_10: 0.0,
        random_baseline: 0.0,
    })
}

/// Evaluate temporal dimension with file queries (expected good)
fn eval_temporal_file(conn: &Connection) -> Result<EvalResults> {
    // Pick files, find their actual co-change partners, check if retrieved

    // Get files with known co-changes
    let mut stmt = conn.prepare(
        "SELECT file_a, file_b, count
         FROM co_changes
         WHERE count >= 3
         ORDER BY count DESC
         LIMIT 100",
    )?;

    let mut cochanges: HashMap<String, HashSet<String>> = HashMap::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let file_a: String = row.get(0)?;
        let file_b: String = row.get(1)?;
        cochanges
            .entry(file_a.clone())
            .or_default()
            .insert(file_b.clone());
        cochanges.entry(file_b).or_default().insert(file_a);
    }

    // Get files with multiple co-change partners
    let test_files: Vec<_> = cochanges
        .iter()
        .filter(|(_, partners)| partners.len() >= 2)
        .take(20)
        .collect();

    println!(
        "Testing {} files with known co-change partners",
        test_files.len()
    );

    let mut total_precision_5 = 0.0;
    let mut total_precision_10 = 0.0;
    let mut num_queries = 0;

    for (file_path, expected_partners) in &test_files {
        // Query using file description (same format as indexed)
        let query = format!("File: {} ({})", file_path, get_file_type(file_path));

        let options = ScryOptions {
            limit: 10,
            min_score: 0.0,
            dimension: Some("temporal".to_string()),
            file: None,
            repo: None,
            all_repos: false,
            include_issues: false,
            include_persona: false,
            hybrid: false,
        };

        if let Ok(results) = scry(&query, &options) {
            // Extract file paths from results
            let retrieved_files: Vec<String> =
                results.iter().map(|r| r.source_id.clone()).collect();

            let hits_5 = retrieved_files
                .iter()
                .take(5)
                .filter(|f| expected_partners.contains(f.as_str()))
                .count();
            let hits_10 = retrieved_files
                .iter()
                .take(10)
                .filter(|f| expected_partners.contains(f.as_str()))
                .count();

            let max_possible = expected_partners.len().min(10);
            let p5 = hits_5 as f32 / 5.0_f32.min(max_possible as f32);
            let p10 = hits_10 as f32 / max_possible as f32;

            total_precision_5 += p5;
            total_precision_10 += p10;
            num_queries += 1;

            if num_queries <= 3 {
                println!(
                    "  {}: found {}/{} partners in top 10",
                    file_path,
                    hits_10,
                    expected_partners.len().min(10)
                );
            }
        }
    }

    if num_queries > 3 {
        println!("  ... and {} more queries", num_queries - 3);
    }

    // Random baseline
    let total_files = cochanges.len();
    let avg_partners =
        cochanges.values().map(|v| v.len()).sum::<usize>() as f32 / total_files as f32;
    let random_baseline = avg_partners / total_files as f32;

    Ok(EvalResults {
        dimension: "temporal".to_string(),
        query_type: "file→files".to_string(),
        num_queries,
        precision_at_5: if num_queries > 0 {
            total_precision_5 / num_queries as f32
        } else {
            0.0
        },
        precision_at_10: if num_queries > 0 {
            total_precision_10 / num_queries as f32
        } else {
            0.0
        },
        random_baseline,
    })
}

fn get_file_type(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "Rust source",
        "ts" => "TypeScript source",
        "js" => "JavaScript source",
        "py" => "Python source",
        "md" => "Markdown document",
        _ => "file",
    }
}

fn print_results(results: &EvalResults) {
    println!("\nResults ({} queries):", results.num_queries);
    println!("  Precision@5:  {:.1}%", results.precision_at_5 * 100.0);
    println!("  Precision@10: {:.1}%", results.precision_at_10 * 100.0);
    println!("  Random baseline: {:.2}%", results.random_baseline * 100.0);
    if results.random_baseline > 0.0 && results.precision_at_10 > 0.0 {
        println!(
            "  Improvement: {:.1}x over random",
            results.precision_at_10 / results.random_baseline
        );
    }
}

// ============================================================================
// Feedback Loop Evaluation (Phase 3)
// ============================================================================

use crate::commands::scrape::database;

/// Execute feedback loop evaluation - measure real-world precision
///
/// Uses feedback views to correlate scry queries with subsequent commits.
pub fn execute_feedback() -> Result<()> {
    println!("📊 Feedback Loop Evaluation\n");
    println!("Measuring real-world retrieval precision from session data...\n");

    let conn = Connection::open(database::PATINA_DB)?;

    // Ensure feedback views exist
    database::create_feedback_views(&conn)?;

    // Get overall statistics
    let (total_queries, total_retrievals): (i64, i64) = conn.query_row(
        "SELECT COUNT(DISTINCT query), COUNT(*) FROM feedback_query_hits",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    if total_queries == 0 {
        println!("No feedback data available yet.");
        println!("\nTo collect feedback data:");
        println!("  1. Start a session: /session-start");
        println!("  2. Run scry queries during development");
        println!("  3. Commit your changes");
        println!("  4. Run: patina scrape git");
        println!("  5. Then run: patina eval --feedback");
        return Ok(());
    }

    let total_hits: i64 = conn.query_row(
        "SELECT COUNT(*) FROM feedback_query_hits WHERE is_hit = 1",
        [],
        |row| row.get(0),
    )?;

    println!("━━━ Overall Statistics ━━━\n");
    println!("Queries with session data: {}", total_queries);
    println!("Total retrievals: {}", total_retrievals);
    println!("Retrievals that led to commits: {}", total_hits);
    println!(
        "Overall precision: {:.1}%",
        if total_retrievals > 0 {
            total_hits as f64 / total_retrievals as f64 * 100.0
        } else {
            0.0
        }
    );

    // Precision by rank
    println!("\n━━━ Precision by Rank ━━━\n");
    let mut stmt = conn.prepare(
        "SELECT rank, COUNT(*) as total, SUM(is_hit) as hits
         FROM feedback_query_hits
         GROUP BY rank
         ORDER BY rank",
    )?;

    let mut rows = stmt.query([])?;
    println!(
        "{:<8} {:>10} {:>10} {:>12}",
        "Rank", "Total", "Hits", "Precision"
    );
    println!("{}", "─".repeat(44));

    while let Some(row) = rows.next()? {
        let rank: i64 = row.get(0)?;
        let total: i64 = row.get(1)?;
        let hits: i64 = row.get(2)?;
        let precision = if total > 0 {
            hits as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "{:<8} {:>10} {:>10} {:>11.1}%",
            rank, total, hits, precision
        );
    }

    // Sessions with most feedback
    println!("\n━━━ Top Sessions by Queries ━━━\n");
    let mut stmt = conn.prepare(
        "SELECT session_id, COUNT(DISTINCT query) as queries,
                SUM(is_hit) as hits, COUNT(*) as retrievals
         FROM feedback_query_hits
         GROUP BY session_id
         ORDER BY queries DESC
         LIMIT 5",
    )?;

    let mut rows = stmt.query([])?;
    println!(
        "{:<20} {:>8} {:>10} {:>12}",
        "Session", "Queries", "Retrievals", "Precision"
    );
    println!("{}", "─".repeat(54));

    while let Some(row) = rows.next()? {
        let session: String = row.get(0)?;
        let queries: i64 = row.get(1)?;
        let hits: i64 = row.get(2)?;
        let retrievals: i64 = row.get(3)?;
        let precision = if retrievals > 0 {
            hits as f64 / retrievals as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "{:<20} {:>8} {:>10} {:>11.1}%",
            session, queries, retrievals, precision
        );
    }

    // High-value retrievals (files that were retrieved AND committed)
    println!("\n━━━ High-Value Retrievals ━━━\n");
    let mut stmt = conn.prepare(
        "SELECT retrieved_doc_id, COUNT(*) as times_retrieved, SUM(is_hit) as times_committed
         FROM feedback_query_hits
         WHERE is_hit = 1
         GROUP BY retrieved_doc_id
         ORDER BY times_committed DESC
         LIMIT 10",
    )?;

    let mut rows = stmt.query([])?;
    let mut has_hits = false;

    println!("{:<50} {:>12} {:>12}", "Document", "Retrieved", "Committed");
    println!("{}", "─".repeat(76));

    while let Some(row) = rows.next()? {
        has_hits = true;
        let doc_id: String = row.get(0)?;
        let retrieved: i64 = row.get(1)?;
        let committed: i64 = row.get(2)?;
        // Truncate long doc_ids
        let display_id = if doc_id.len() > 48 {
            format!("...{}", &doc_id[doc_id.len() - 45..])
        } else {
            doc_id
        };
        println!("{:<50} {:>12} {:>12}", display_id, retrieved, committed);
    }

    if !has_hits {
        println!("(No retrievals have matched committed files yet)");
        println!("\nNote: Hits occur when retrieved doc_ids match committed file paths.");
        println!("Code queries (not session queries) are more likely to have hits.");
    }

    println!("\n{}", "─".repeat(60));

    Ok(())
}
