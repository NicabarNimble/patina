//! Evaluation framework for validating retrieval quality
//!
//! Post-split eval modes:
//! - `--assay`: independent factual retrieval eval (FTS5)
//! - `--scry`: independent semantic retrieval eval (vectors) + scry-vs-assay comparison
//! - `--combined`: full pipeline eval (assay + scry together)
//!
//! Legacy eval modes (pre-split, kept for historical comparison):
//! - `--nl`: NL query eval against QueryEngine (52 queries from mixed-oracle era)
//! - `--feedback`: feedback loop eval from session data
//! - `--dimension`: structural/temporal/belief co-retrieval tests

mod internal;

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

use crate::retrieval::{FusedResult, QueryEngine, RetrievalConfig};

/// Evaluation results for one engine + test combination
#[derive(Debug)]
pub struct EvalResults {
    pub engine: String,
    pub test_name: String,
    pub num_queries: usize,
    pub precision_at_5: f32,
    pub precision_at_10: f32,
    pub random_baseline: f32,
}

/// Belief self-retrieval results (MRR-based)
#[derive(Debug)]
pub struct BeliefSelfResults {
    pub engine: String,
    pub num_queries: usize,
    pub mrr: f32,
    pub hit_rate: f32,
}

/// Belief-code co-retrieval results (split metrics per reviewer feedback)
#[derive(Debug)]
pub struct BeliefCoResults {
    pub engine: String,
    pub num_queries: usize,
    /// Fraction of queries where belief:<id> appeared in top-K
    pub belief_present_rate: f32,
    /// Avg(reached files in top-K / min(K, reach_count))
    pub reach_recall: f32,
    /// Fraction where belief present AND ≥1 reached file
    pub co_retrieval_rate: f32,
}

/// Run evaluation
pub fn execute(dimension: Option<String>) -> Result<()> {
    println!("📊 Evaluation Framework\n");
    println!("Testing retrieval quality: unified pipeline + per-oracle ablation\n");

    let db_path = ".patina/local/data/patina.db";
    let conn = Connection::open(db_path)?;

    // Create engines: unified (all oracles), ablation per-oracle, and belief delta
    let unified = QueryEngine::new();
    let semantic_only = QueryEngine::with_config(RetrievalConfig {
        oracle_filter: Some(vec!["semantic".to_string()]),
        ..Default::default()
    });
    let temporal_only = QueryEngine::with_config(RetrievalConfig {
        oracle_filter: Some(vec!["temporal".to_string()]),
        ..Default::default()
    });
    // D1 measurement: all oracles EXCEPT belief — delta measures belief impact
    let no_belief = QueryEngine::with_config(RetrievalConfig {
        oracle_filter: Some(vec![
            "semantic".to_string(),
            "lexical".to_string(),
            "temporal".to_string(),
            "persona".to_string(),
        ]),
        ..Default::default()
    });

    let mut all_results = Vec::new();

    // Semantic tests: --dimension narrows which tests run, not which engines
    if dimension.is_none() || dimension.as_deref() == Some("semantic") {
        println!("━━━ Unified Pipeline (code → same-file) ━━━\n");
        let results = eval_semantic_co_retrieval(&conn, &unified, "unified")?;
        print_results(&results);
        all_results.push(results);

        println!("\n━━━ Ablation: no-belief (code → same-file) ━━━\n");
        let results = eval_semantic_co_retrieval(&conn, &no_belief, "no-belief")?;
        print_results(&results);
        all_results.push(results);

        println!("\n━━━ Ablation: semantic-only (code → same-file) ━━━\n");
        let results = eval_semantic_co_retrieval(&conn, &semantic_only, "semantic-only")?;
        print_results(&results);
        all_results.push(results);
    }

    // Temporal tests
    if dimension.is_none() || dimension.as_deref() == Some("temporal") {
        // Score distribution (unified only, no ground truth)
        println!("\n━━━ Unified Pipeline (text → score distribution) ━━━\n");
        eval_temporal_text(&conn, &unified)?;

        // File co-change (unified + temporal-only)
        println!("\n━━━ Unified Pipeline (file → co-change) ━━━\n");
        let results = eval_temporal_file(&conn, &unified, "unified")?;
        print_results(&results);
        all_results.push(results);

        println!("\n━━━ Ablation: no-belief (file → co-change) ━━━\n");
        let results = eval_temporal_file(&conn, &no_belief, "no-belief")?;
        print_results(&results);
        all_results.push(results);

        println!("\n━━━ Ablation: temporal-only (file → co-change) ━━━\n");
        let results = eval_temporal_file(&conn, &temporal_only, "temporal-only")?;
        print_results(&results);
        all_results.push(results);
    }

    // Belief tests: knowledge-query ground truth
    let mut self_results = Vec::new();
    let mut co_results = Vec::new();

    if dimension.is_none() || dimension.as_deref() == Some("belief") {
        println!("\n━━━ Unified Pipeline (belief self-retrieval) ━━━\n");
        let results = eval_belief_self_retrieval(&conn, &unified, "unified")?;
        print_belief_self_results(&results);
        self_results.push(results);

        println!("\n━━━ Ablation: no-belief (belief self-retrieval) ━━━\n");
        let results = eval_belief_self_retrieval(&conn, &no_belief, "no-belief")?;
        print_belief_self_results(&results);
        self_results.push(results);

        println!("\n━━━ Unified Pipeline (belief→code co-retrieval) ━━━\n");
        let results = eval_belief_code_co_retrieval(&conn, &unified, "unified")?;
        print_belief_co_results(&results);
        co_results.push(results);

        println!("\n━━━ Ablation: no-belief (belief→code co-retrieval) ━━━\n");
        let results = eval_belief_code_co_retrieval(&conn, &no_belief, "no-belief")?;
        print_belief_co_results(&results);
        co_results.push(results);
    }

    // Summary table: structural tests
    println!("\n━━━ Summary ━━━\n");
    println!(
        "{:<35} {:>12} {:>12} {:>12}",
        "Pipeline", "P@5", "P@10", "vs Random"
    );
    println!("{}", "─".repeat(75));
    for r in &all_results {
        let vs_random = if r.random_baseline > 0.0 {
            r.precision_at_10 / r.random_baseline
        } else {
            0.0
        };
        println!(
            "{:<35} {:>11.1}% {:>11.1}% {:>11.1}x",
            format!("{} ({})", r.engine, r.test_name),
            r.precision_at_5 * 100.0,
            r.precision_at_10 * 100.0,
            vs_random
        );
    }

    // Summary table: belief tests
    if !self_results.is_empty() {
        println!(
            "\n{:<35} {:>12} {:>12}",
            "Pipeline (self-retrieval)", "MRR", "Hit Rate"
        );
        println!("{}", "─".repeat(63));
        for r in &self_results {
            println!(
                "{:<35} {:>12.3} {:>11.1}%",
                r.engine,
                r.mrr,
                r.hit_rate * 100.0,
            );
        }
    }

    if !co_results.is_empty() {
        println!(
            "\n{:<35} {:>10} {:>10} {:>10}",
            "Pipeline (co-retrieval)", "B.Pres", "ReachR", "Co-Retr"
        );
        println!("{}", "─".repeat(69));
        for r in &co_results {
            println!(
                "{:<35} {:>9.1}% {:>9.1}% {:>9.1}%",
                r.engine,
                r.belief_present_rate * 100.0,
                r.reach_recall * 100.0,
                r.co_retrieval_rate * 100.0,
            );
        }
    }

    // D1 belief delta: full picture
    const STRUCTURAL_BUDGET_PP: f32 = 5.0; // max acceptable regression in percentage points
    let mut d1_pass = true;

    println!("\n━━━ D1 Belief Delta (unified vs no-belief) ━━━\n");
    println!(
        "{:<25} {:>12} {:>12} {:>8} {:>8}",
        "Test", "Unified", "No-Belief", "Delta", "Verdict"
    );
    println!("{}", "─".repeat(69));

    // Self-retrieval delta (MRR)
    if let (Some(u), Some(nb)) = (
        self_results.iter().find(|r| r.engine == "unified"),
        self_results.iter().find(|r| r.engine == "no-belief"),
    ) {
        let delta = u.mrr - nb.mrr;
        let verdict = if delta >= 0.0 { "PASS" } else { "FAIL" };
        if delta < 0.0 {
            d1_pass = false;
        }
        println!(
            "{:<25} {:>8.3}MRR {:>8.3}MRR {:>+7.3} {:>8}",
            "self-retrieval", u.mrr, nb.mrr, delta, verdict
        );
    }

    // Co-retrieval delta (co_retrieval_rate)
    if let (Some(u), Some(nb)) = (
        co_results.iter().find(|r| r.engine == "unified"),
        co_results.iter().find(|r| r.engine == "no-belief"),
    ) {
        let delta = u.co_retrieval_rate - nb.co_retrieval_rate;
        let verdict = if delta >= 0.0 { "PASS" } else { "FAIL" };
        if delta < 0.0 {
            d1_pass = false;
        }
        println!(
            "{:<25} {:>9.1}%   {:>9.1}%   {:>+6.1}% {:>8}",
            "belief→code",
            u.co_retrieval_rate * 100.0,
            nb.co_retrieval_rate * 100.0,
            delta * 100.0,
            verdict
        );
    }

    // Structural test deltas (P@10, budget-enforced)
    let test_names: Vec<String> = all_results.iter().map(|r| r.test_name.clone()).collect();
    for test in test_names.iter().collect::<HashSet<_>>() {
        let unified_r = all_results
            .iter()
            .find(|r| r.engine == "unified" && &r.test_name == test);
        let no_belief_r = all_results
            .iter()
            .find(|r| r.engine == "no-belief" && &r.test_name == test);
        if let (Some(u), Some(nb)) = (unified_r, no_belief_r) {
            let delta_pp = (u.precision_at_10 - nb.precision_at_10) * 100.0;
            let within_budget = delta_pp >= -STRUCTURAL_BUDGET_PP;
            let verdict = if within_budget {
                "PASS"
            } else {
                d1_pass = false;
                "FAIL"
            };
            println!(
                "{:<25} {:>11.1}% {:>11.1}% {:>+6.1}pp {:>5} (budget: {}pp)",
                test,
                u.precision_at_10 * 100.0,
                nb.precision_at_10 * 100.0,
                delta_pp,
                verdict,
                STRUCTURAL_BUDGET_PP,
            );
        }
    }

    println!(
        "\n{}",
        if d1_pass {
            "D1 VERDICT: PASS — knowledge gains positive, structural regression within budget"
        } else {
            "D1 VERDICT: FAIL — see failing tests above"
        }
    );

    // Emit measurement: unified engine results for the dimension eval
    if let Some(unified_r) = all_results.iter().find(|r| r.engine == "unified") {
        let unified_mrr = self_results
            .iter()
            .find(|r| r.engine == "unified")
            .map(|r| r.mrr as f64)
            .unwrap_or(0.0);
        patina::measure::emit_or_warn(
            "search",
            "eval",
            "dimension",
            &serde_json::json!({
                "p_at_5": unified_r.precision_at_5 as f64,
                "p_at_10": unified_r.precision_at_10 as f64,
                "mrr": unified_mrr,
                "query_count": unified_r.num_queries,
            }),
        );
    }

    Ok(())
}

// ============================================================================
// Semantic evaluation: function_facts co-retrieval
// ============================================================================

/// Evaluate semantic retrieval: functions in same file should co-retrieve
///
/// Ground truth: function_facts table. Files with 3+ functions provide
/// query (one function description) and expected results (other functions
/// from same file). doc_ids are file::function format — unique per function,
/// no RRF dedup issue.
fn eval_semantic_co_retrieval(
    conn: &Connection,
    engine: &QueryEngine,
    engine_name: &str,
) -> Result<EvalResults> {
    // Load function_facts grouped by file
    let mut files: HashMap<String, Vec<(String, String)>> = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT file, name, parameters, return_type, is_public, is_async
         FROM function_facts
         ORDER BY file, name",
    )?;

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let file: String = row.get(0)?;
        let name: String = row.get(1)?;
        let params: Option<String> = row.get(2)?;
        let return_type: Option<String> = row.get(3)?;
        let is_public: bool = row.get(4)?;
        let is_async: bool = row.get(5)?;

        // Build description matching what's embedded in the semantic index
        let mut desc = format!("Function `{}` in `{}`", name, file);
        if is_public {
            desc.push_str(", public");
        }
        if is_async {
            desc.push_str(", async");
        }
        if let Some(ref p) = params {
            if !p.is_empty() {
                desc.push_str(&format!(", params: {}", p));
            }
        }
        if let Some(ref rt) = return_type {
            if !rt.is_empty() {
                desc.push_str(&format!(", returns: {}", rt));
            }
        }

        files.entry(file).or_default().push((name, desc));
    }

    // Files with 3+ functions have enough for query + expected results
    let valid_files: Vec<_> = files.iter().filter(|(_, funcs)| funcs.len() >= 3).collect();

    println!(
        "Found {} files with 3+ functions ({} total functions)",
        valid_files.len(),
        files.values().map(|v| v.len()).sum::<usize>()
    );

    if valid_files.is_empty() {
        return Ok(EvalResults {
            engine: engine_name.to_string(),
            test_name: "code→same-file".to_string(),
            num_queries: 0,
            precision_at_5: 0.0,
            precision_at_10: 0.0,
            random_baseline: 0.0,
        });
    }

    let mut total_precision_5 = 0.0;
    let mut total_precision_10 = 0.0;
    let mut num_queries = 0;

    // Sample up to 20 files, seeded for deterministic eval
    let sample_size = valid_files.len().min(20);
    let mut rng = fastrand::Rng::with_seed(42);

    for i in 0..sample_size {
        let idx = if sample_size < valid_files.len() {
            rng.usize(..valid_files.len())
        } else {
            i
        };

        let (file_path, functions) = valid_files[idx];

        // Use first function's description as query
        let query = &functions[0].1;
        let expected_file = normalize_path(file_path);
        let expected_count = functions.len() - 1; // exclude query function itself

        if let Ok(results) = engine.query(query, 10) {
            let hits_5 = count_file_hits(&results, &expected_file, 5);
            let hits_10 = count_file_hits(&results, &expected_file, 10);

            let p5 = hits_5 as f32 / 5.0_f32.min(expected_count as f32);
            let p10 = hits_10 as f32 / 10.0_f32.min(expected_count as f32);

            total_precision_5 += p5;
            total_precision_10 += p10;
            num_queries += 1;

            if num_queries <= 3 {
                println!(
                    "  {} ({} funcs): P@5={:.0}%, P@10={:.0}%",
                    file_path,
                    functions.len(),
                    p5 * 100.0,
                    p10 * 100.0
                );
            }
        }
    }

    if num_queries > 3 {
        println!("  ... and {} more queries", num_queries - 3);
    }

    // Random baseline: chance of hitting same-file function
    let total_functions: usize = files.values().map(|v| v.len()).sum();
    let avg_file_size = total_functions as f32 / files.len() as f32;
    let random_baseline = avg_file_size / total_functions as f32;

    Ok(EvalResults {
        engine: engine_name.to_string(),
        test_name: "code→same-file".to_string(),
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

// ============================================================================
// Temporal evaluation: co-change partners + score distribution
// ============================================================================

/// Evaluate temporal with text queries (score distribution, no ground truth)
///
/// Measures whether the unified pipeline returns meaningful score distributions
/// for text queries. No precision — just diagnostic.
fn eval_temporal_text(conn: &Connection, engine: &QueryEngine) -> Result<()> {
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
        "Testing {} text queries (score distribution)",
        queries.len()
    );

    let mut avg_top_score = 0.0;
    let mut avg_score_variance = 0.0;
    let mut num_queries = 0;

    for query in queries.iter().take(10) {
        if let Ok(results) = engine.query(query, 10) {
            if !results.is_empty() {
                let scores: Vec<f32> = results.iter().map(|r| r.fused_score).collect();
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

    println!("  Avg top fused score: {:.4}", avg_top_score);
    println!(
        "  Avg score variance: {:.6} (low = results are random-ish)",
        avg_score_variance
    );
    println!("  Queries evaluated: {}", num_queries);

    Ok(())
}

/// Evaluate temporal with file queries: file → co-change partners
///
/// Ground truth: co_changes table. Files that frequently change together
/// should co-retrieve. Extract file path from FusedResult doc_id (strip
/// "./" prefix and "::suffix") before matching.
fn eval_temporal_file(
    conn: &Connection,
    engine: &QueryEngine,
    engine_name: &str,
) -> Result<EvalResults> {
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

    // Files with 2+ co-change partners, sorted for deterministic eval
    let mut test_files: Vec<_> = cochanges
        .iter()
        .filter(|(_, partners)| partners.len() >= 2)
        .collect();
    test_files.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    test_files.truncate(20);

    println!(
        "Testing {} files with known co-change partners",
        test_files.len()
    );

    if test_files.is_empty() {
        return Ok(EvalResults {
            engine: engine_name.to_string(),
            test_name: "file→co-change".to_string(),
            num_queries: 0,
            precision_at_5: 0.0,
            precision_at_10: 0.0,
            random_baseline: 0.0,
        });
    }

    let mut total_precision_5 = 0.0;
    let mut total_precision_10 = 0.0;
    let mut num_queries = 0;

    for (file_path, expected_partners) in &test_files {
        let query = format!("File: {} ({})", file_path, get_file_type(file_path));

        if let Ok(results) = engine.query(&query, 10) {
            // Extract and normalize file paths from FusedResult doc_ids
            let retrieved_files: Vec<String> = results
                .iter()
                .map(|r| extract_file_from_doc_id(&r.doc_id))
                .collect();

            // Normalize expected partners for comparison
            let normalized_partners: HashSet<String> = expected_partners
                .iter()
                .map(|p| normalize_path(p))
                .collect();

            let hits_5 = retrieved_files
                .iter()
                .take(5)
                .filter(|f| normalized_partners.contains(f.as_str()))
                .count();
            let hits_10 = retrieved_files
                .iter()
                .take(10)
                .filter(|f| normalized_partners.contains(f.as_str()))
                .count();

            let max_possible = normalized_partners.len().min(10);
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
        engine: engine_name.to_string(),
        test_name: "file→co-change".to_string(),
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

// ============================================================================
// Belief evaluation: self-retrieval + code co-retrieval
// ============================================================================

/// Belief self-retrieval: query with belief statement, check if belief appears in results
///
/// Ground truth: beliefs table. MRR = average of 1/rank for each belief found.
/// Hit rate = fraction of beliefs found in top-K at all.
fn eval_belief_self_retrieval(
    conn: &Connection,
    engine: &QueryEngine,
    engine_name: &str,
) -> Result<BeliefSelfResults> {
    let mut stmt = conn.prepare("SELECT id, statement FROM beliefs ORDER BY id")?;
    let mut beliefs: Vec<(String, String)> = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let statement: String = row.get(1)?;
        beliefs.push((id, statement));
    }

    println!("Testing {} beliefs (self-retrieval)", beliefs.len());

    if beliefs.is_empty() {
        return Ok(BeliefSelfResults {
            engine: engine_name.to_string(),
            num_queries: 0,
            mrr: 0.0,
            hit_rate: 0.0,
        });
    }

    let k = 10;
    let mut total_rr = 0.0;
    let mut hits = 0;
    let mut num_queries = 0;

    for (id, statement) in &beliefs {
        let expected_doc_id = format!("belief:{}", id);

        if let Ok(results) = engine.query(statement, k) {
            let rank = results
                .iter()
                .position(|r| r.doc_id == expected_doc_id)
                .map(|pos| pos + 1); // 1-indexed

            if let Some(r) = rank {
                total_rr += 1.0 / r as f32;
                hits += 1;
            }

            num_queries += 1;

            if num_queries <= 5 {
                let rank_str = rank
                    .map(|r| format!("@{}", r))
                    .unwrap_or("miss".to_string());
                println!("  {} — {}", id, rank_str);
            }
        }
    }

    if num_queries > 5 {
        println!("  ... and {} more beliefs", num_queries - 5);
    }

    let mrr = if num_queries > 0 {
        total_rr / num_queries as f32
    } else {
        0.0
    };
    let hit_rate = if num_queries > 0 {
        hits as f32 / num_queries as f32
    } else {
        0.0
    };

    Ok(BeliefSelfResults {
        engine: engine_name.to_string(),
        num_queries,
        mrr,
        hit_rate,
    })
}

/// Belief-code co-retrieval: query with belief statement, check for belief AND reached code
///
/// Split metrics:
/// - belief_present_rate: is belief:<id> in top-K?
/// - reach_recall@K: reached files in top-K / min(K, reach_count)
/// - co_retrieval_rate: belief present AND ≥1 reached file (the product claim)
fn eval_belief_code_co_retrieval(
    conn: &Connection,
    engine: &QueryEngine,
    engine_name: &str,
) -> Result<BeliefCoResults> {
    // Load beliefs that have code reach entries
    let mut stmt = conn.prepare(
        "SELECT b.id, b.statement, GROUP_CONCAT(bcr.file_path, '|') as files
         FROM beliefs b
         JOIN belief_code_reach bcr ON b.id = bcr.belief_id
         GROUP BY b.id
         HAVING COUNT(bcr.file_path) >= 1
         ORDER BY b.id",
    )?;

    let mut beliefs_with_reach: Vec<(String, String, Vec<String>)> = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let statement: String = row.get(1)?;
        let files_str: String = row.get(2)?;
        let files: Vec<String> = files_str.split('|').map(normalize_path).collect();
        beliefs_with_reach.push((id, statement, files));
    }

    println!(
        "Testing {} beliefs with code reach",
        beliefs_with_reach.len()
    );

    if beliefs_with_reach.is_empty() {
        return Ok(BeliefCoResults {
            engine: engine_name.to_string(),
            num_queries: 0,
            belief_present_rate: 0.0,
            reach_recall: 0.0,
            co_retrieval_rate: 0.0,
        });
    }

    let k = 10;
    let mut belief_present_count = 0;
    let mut total_reach_recall = 0.0;
    let mut co_retrieval_count = 0;
    let mut num_queries = 0;

    for (id, statement, reached_files) in &beliefs_with_reach {
        let expected_belief = format!("belief:{}", id);

        if let Ok(results) = engine.query(statement, k) {
            // Belief-present@K
            let belief_present = results.iter().any(|r| r.doc_id == expected_belief);
            if belief_present {
                belief_present_count += 1;
            }

            // Reach-hit@K: normalize by min(K, reach_count)
            let reached_set: HashSet<&str> = reached_files.iter().map(|f| f.as_str()).collect();
            let reach_hits = results
                .iter()
                .take(k)
                .filter(|r| {
                    let file = extract_file_from_doc_id(&r.doc_id);
                    reached_set.contains(file.as_str())
                })
                .count();
            let max_possible = k.min(reached_files.len());
            let recall = if max_possible > 0 {
                reach_hits as f32 / max_possible as f32
            } else {
                0.0
            };
            total_reach_recall += recall;

            // Co-retrieval: belief present AND ≥1 reached file
            if belief_present && reach_hits >= 1 {
                co_retrieval_count += 1;
            }

            num_queries += 1;

            if num_queries <= 5 {
                let bp = if belief_present { "✓" } else { "✗" };
                println!(
                    "  {} — belief:{} reach:{}/{} files",
                    id,
                    bp,
                    reach_hits,
                    reached_files.len()
                );
            }
        }
    }

    if num_queries > 5 {
        println!("  ... and {} more beliefs", num_queries - 5);
    }

    let belief_present_rate = if num_queries > 0 {
        belief_present_count as f32 / num_queries as f32
    } else {
        0.0
    };
    let reach_recall = if num_queries > 0 {
        total_reach_recall / num_queries as f32
    } else {
        0.0
    };
    let co_retrieval_rate = if num_queries > 0 {
        co_retrieval_count as f32 / num_queries as f32
    } else {
        0.0
    };

    Ok(BeliefCoResults {
        engine: engine_name.to_string(),
        num_queries,
        belief_present_rate,
        reach_recall,
        co_retrieval_rate,
    })
}

fn print_belief_self_results(results: &BeliefSelfResults) {
    println!("\nResults ({} beliefs):", results.num_queries);
    println!("  MRR:       {:.3}", results.mrr);
    println!("  Hit rate:  {:.1}%", results.hit_rate * 100.0);
}

fn print_belief_co_results(results: &BeliefCoResults) {
    println!(
        "\nResults ({} beliefs with code reach):",
        results.num_queries
    );
    println!(
        "  Belief present: {:.1}%",
        results.belief_present_rate * 100.0
    );
    println!("  Reach recall:   {:.1}%", results.reach_recall * 100.0);
    println!(
        "  Co-retrieval:   {:.1}% (belief + ≥1 code)",
        results.co_retrieval_rate * 100.0
    );
}

// ============================================================================
// Helpers
// ============================================================================

/// Extract file path from a FusedResult doc_id, normalized for comparison
///
/// Handles different oracle doc_id formats:
/// - "./src/main.rs::fn:main" → "src/main.rs" (SemanticOracle code facts)
/// - "src/main.rs" → "src/main.rs" (TemporalOracle co-changes)
/// - "persona:direct:..." → "persona:direct:..." (no file, won't match)
fn extract_file_from_doc_id(doc_id: &str) -> String {
    let path = if let Some(idx) = doc_id.find("::") {
        &doc_id[..idx]
    } else {
        doc_id
    };
    normalize_path(path)
}

/// Normalize path by stripping "./" prefix
fn normalize_path(path: &str) -> String {
    path.strip_prefix("./").unwrap_or(path).to_string()
}

/// Count results in top-K whose doc_id resolves to the expected file
fn count_file_hits(results: &[FusedResult], expected_file: &str, k: usize) -> usize {
    results
        .iter()
        .take(k)
        .filter(|r| extract_file_from_doc_id(&r.doc_id) == expected_file)
        .count()
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

use patina::eventlog;

/// Execute feedback loop evaluation - measure real-world precision
///
/// Cross-database query: opens events.db (scry.query events), ATTACHes patina.db
/// (git.commit → commit_files for structured file paths). The ATTACH direction is
/// opposite from measure — eval owns the scry.query data.
pub fn execute_feedback() -> Result<()> {
    println!("📊 Feedback Loop Evaluation\n");
    println!("Measuring real-world retrieval precision from session data...\n");

    // Open events.db as primary — scry.query events live here
    let conn = eventlog::open_events_db()?;

    // ATTACH patina.db for git.commit data and structured commit_files table
    let patina_path = std::path::Path::new(eventlog::PATINA_DB);
    if !patina_path.exists() {
        println!("No patina.db found — cannot correlate queries with commits.");
        return Ok(());
    }
    conn.execute(
        "ATTACH DATABASE ?1 AS patina",
        [patina_path.to_str().unwrap_or(eventlog::PATINA_DB)],
    )?;

    // Step 1: Session → commit SHA mapping from patina.db git.commit events.
    // Then join with structured commit_files for file paths (no JSON file parsing).
    conn.execute_batch(
        r#"
        DROP TABLE IF EXISTS _fb_commit_files;
        CREATE TEMP TABLE _fb_commit_files AS
        SELECT DISTINCT sc.session_id, cf.file_path
        FROM (
            SELECT
                json_extract(data, '$.session_id') as session_id,
                json_extract(data, '$.sha') as sha
            FROM patina.eventlog
            WHERE event_type = 'git.commit'
              AND json_extract(data, '$.session_id') IS NOT NULL
        ) sc
        JOIN patina.commit_files cf ON cf.sha = sc.sha;
        CREATE INDEX _fb_cf_session ON _fb_commit_files(session_id);
        CREATE INDEX _fb_cf_path ON _fb_commit_files(file_path);
        "#,
    )?;

    // Step 2: scry.query results from events.db, joined with commit files.
    // Normalize doc_id: strip '::...' suffix and './' prefix before matching.
    conn.execute_batch(
        r#"
        DROP TABLE IF EXISTS _fb_query_hits;
        CREATE TEMP TABLE _fb_query_hits AS
        SELECT
            q_session_id as session_id,
            query,
            mode,
            query_time,
            doc_id as retrieved_doc_id,
            rank,
            score,
            CASE WHEN cf.file_path IS NOT NULL THEN 1 ELSE 0 END as is_hit
        FROM (
            SELECT
                json_extract(data, '$.session_id') as q_session_id,
                json_extract(data, '$.query') as query,
                json_extract(data, '$.mode') as mode,
                timestamp as query_time,
                json_extract(r.value, '$.doc_id') as doc_id,
                -- Normalize: strip '::...' suffix and './' prefix
                REPLACE(
                    CASE
                        WHEN INSTR(json_extract(r.value, '$.doc_id'), '::') > 0
                        THEN SUBSTR(json_extract(r.value, '$.doc_id'), 1,
                             INSTR(json_extract(r.value, '$.doc_id'), '::') - 1)
                        ELSE json_extract(r.value, '$.doc_id')
                    END,
                    './', '') as norm_doc_id,
                json_extract(r.value, '$.rank') as rank,
                json_extract(r.value, '$.score') as score
            FROM eventlog, json_each(json_extract(data, '$.results')) as r
            WHERE event_type = 'scry.query'
              AND json_extract(data, '$.session_id') IS NOT NULL
        ) q
        LEFT JOIN _fb_commit_files cf
            ON cf.session_id = q.q_session_id
            AND cf.file_path = q.norm_doc_id;
        "#,
    )?;

    // Get overall statistics
    let (total_queries, total_retrievals): (i64, i64) = conn.query_row(
        "SELECT COUNT(DISTINCT query), COUNT(*) FROM _fb_query_hits",
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
        "SELECT COUNT(*) FROM _fb_query_hits WHERE is_hit = 1",
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
         FROM _fb_query_hits
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
         FROM _fb_query_hits
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
         FROM _fb_query_hits
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

    // Emit measurement: feedback loop precision
    let precision = if total_retrievals > 0 {
        total_hits as f64 / total_retrievals as f64
    } else {
        0.0
    };
    patina::measure::emit_or_warn(
        "search",
        "eval",
        "feedback",
        &serde_json::json!({
            "precision": precision,
            "query_count": total_queries,
            "total_retrievals": total_retrievals,
            "total_hits": total_hits,
        }),
    );

    Ok(())
}

// ============================================================================
// Natural-Language Query Evaluation (Phase 2)
// ============================================================================

/// NL query test case loaded from JSON
#[derive(serde::Deserialize, Debug)]
struct NlQueryCase {
    query: String,
    expected: Vec<String>,
    category: String,
    split: String,
}

/// Aggregated NL eval metrics for one engine configuration
#[derive(Debug, Clone)]
struct NlMetrics {
    name: String,
    p5: f32,
    p10: f32,
    mrr: f32,
}

/// Score one engine against the NL test set, return aggregate metrics
fn score_nl_engine(engine: &QueryEngine, name: &str, cases: &[NlQueryCase]) -> Result<NlMetrics> {
    let mut total_p5 = 0.0f32;
    let mut total_p10 = 0.0f32;
    let mut total_rr = 0.0f32;

    for case in cases {
        let results = engine.query(&case.query, 10)?;
        let expected: HashSet<String> = case.expected.iter().map(|p| normalize_path(p)).collect();

        // Deduplicate by file — multiple doc_ids from the same file count once
        let unique_files_5: HashSet<String> = results
            .iter()
            .take(5)
            .map(|r| extract_file_from_doc_id(&r.doc_id))
            .filter(|f| expected.contains(f))
            .collect();
        let unique_files_10: HashSet<String> = results
            .iter()
            .take(10)
            .map(|r| extract_file_from_doc_id(&r.doc_id))
            .filter(|f| expected.contains(f))
            .collect();

        let denom_5 = expected.len().clamp(1, 5) as f32;
        let denom_10 = expected.len().clamp(1, 10) as f32;
        total_p5 += unique_files_5.len() as f32 / denom_5;
        total_p10 += unique_files_10.len() as f32 / denom_10;

        // MRR: rank of first hit (by file, not by doc_id — same result)
        total_rr += results
            .iter()
            .enumerate()
            .find(|(_, r)| expected.contains(&extract_file_from_doc_id(&r.doc_id)))
            .map(|(i, _)| 1.0 / (i as f32 + 1.0))
            .unwrap_or(0.0);
    }

    let n = cases.len();
    Ok(NlMetrics {
        name: name.to_string(),
        p5: total_p5 / n as f32,
        p10: total_p10 / n as f32,
        mrr: total_rr / n as f32,
    })
}

/// Score one engine against a subset of NL test cases (by reference)
fn score_nl_engine_refs(
    engine: &QueryEngine,
    name: &str,
    cases: &[&NlQueryCase],
) -> Result<NlMetrics> {
    let mut total_p5 = 0.0f32;
    let mut total_p10 = 0.0f32;
    let mut total_rr = 0.0f32;

    for case in cases {
        let results = engine.query(&case.query, 10)?;
        let expected: HashSet<String> = case.expected.iter().map(|p| normalize_path(p)).collect();

        let unique_files_5: HashSet<String> = results
            .iter()
            .take(5)
            .map(|r| extract_file_from_doc_id(&r.doc_id))
            .filter(|f| expected.contains(f))
            .collect();
        let unique_files_10: HashSet<String> = results
            .iter()
            .take(10)
            .map(|r| extract_file_from_doc_id(&r.doc_id))
            .filter(|f| expected.contains(f))
            .collect();

        let denom_5 = expected.len().clamp(1, 5) as f32;
        let denom_10 = expected.len().clamp(1, 10) as f32;
        total_p5 += unique_files_5.len() as f32 / denom_5;
        total_p10 += unique_files_10.len() as f32 / denom_10;

        total_rr += results
            .iter()
            .enumerate()
            .find(|(_, r)| expected.contains(&extract_file_from_doc_id(&r.doc_id)))
            .map(|(i, _)| 1.0 / (i as f32 + 1.0))
            .unwrap_or(0.0);
    }

    let n = cases.len();
    Ok(NlMetrics {
        name: name.to_string(),
        p5: total_p5 / n as f32,
        p10: total_p10 / n as f32,
        mrr: total_rr / n as f32,
    })
}

/// Execute NL query eval from curated test set
///
/// Loads queries from resources/eval/nl-queries.json, runs each through the
/// unified QueryEngine, and measures P@5, P@10, MRR against expected results.
/// Includes per-oracle ablation to measure each oracle's contribution.
pub fn execute_nl() -> Result<()> {
    println!("📊 Natural-Language Query Evaluation\n");
    println!("Testing retrieval quality with curated real-world queries...\n");

    // Load test set
    let test_path = "resources/eval/nl-queries.json";
    let content = std::fs::read_to_string(test_path).context(format!("Cannot read {test_path}"))?;
    let cases: Vec<NlQueryCase> =
        serde_json::from_str(&content).context("Failed to parse nl-queries.json")?;

    // --- Per-query detail for unified engine ---
    let unified = QueryEngine::new();

    let mut category_stats: HashMap<String, (f32, f32, f32, usize)> = HashMap::new();
    let mut split_stats: HashMap<String, (f32, f32, f32, usize)> = HashMap::new();

    let train_count = cases.iter().filter(|c| c.split == "train").count();
    let test_count = cases.iter().filter(|c| c.split == "test").count();
    println!(
        "Loaded {} test queries ({} train, {} test)\n",
        cases.len(),
        train_count,
        test_count
    );

    println!("{:<55} {:>6} {:>6} {:>6}", "Query", "P@5", "P@10", "RR");
    println!("{}", "─".repeat(77));

    for case in &cases {
        let results = unified.query(&case.query, 10)?;
        let expected: HashSet<String> = case.expected.iter().map(|p| normalize_path(p)).collect();

        // Deduplicate by file — multiple doc_ids from the same file count once
        let unique_files_5: HashSet<String> = results
            .iter()
            .take(5)
            .map(|r| extract_file_from_doc_id(&r.doc_id))
            .filter(|f| expected.contains(f))
            .collect();
        let unique_files_10: HashSet<String> = results
            .iter()
            .take(10)
            .map(|r| extract_file_from_doc_id(&r.doc_id))
            .filter(|f| expected.contains(f))
            .collect();

        let denom_5 = expected.len().clamp(1, 5) as f32;
        let denom_10 = expected.len().clamp(1, 10) as f32;
        let p5 = unique_files_5.len() as f32 / denom_5;
        let p10 = unique_files_10.len() as f32 / denom_10;

        let rr = results
            .iter()
            .enumerate()
            .find(|(_, r)| expected.contains(&extract_file_from_doc_id(&r.doc_id)))
            .map(|(i, _)| 1.0 / (i as f32 + 1.0))
            .unwrap_or(0.0);

        let entry = category_stats
            .entry(case.category.clone())
            .or_insert((0.0, 0.0, 0.0, 0));
        entry.0 += p5;
        entry.1 += p10;
        entry.2 += rr;
        entry.3 += 1;

        let split_entry = split_stats
            .entry(case.split.clone())
            .or_insert((0.0, 0.0, 0.0, 0));
        split_entry.0 += p5;
        split_entry.1 += p10;
        split_entry.2 += rr;
        split_entry.3 += 1;

        let display_q = if case.query.len() > 53 {
            format!("{}...", &case.query[..50])
        } else {
            case.query.clone()
        };
        println!(
            "{:<55} {:>5.0}% {:>5.0}% {:>.3}",
            display_q,
            p5 * 100.0,
            p10 * 100.0,
            rr
        );
    }

    // Category breakdown
    println!("\n━━━ By Category ━━━\n");
    println!(
        "{:<20} {:>6} {:>8} {:>8} {:>8}",
        "Category", "N", "P@5", "P@10", "MRR"
    );
    println!("{}", "─".repeat(54));

    let mut cats: Vec<_> = category_stats.iter().collect();
    cats.sort_by_key(|(k, _)| (*k).clone());
    for (cat, (p5, p10, rr, count)) in &cats {
        let n = *count as f32;
        println!(
            "{:<20} {:>6} {:>7.1}% {:>7.1}% {:>8.3}",
            cat,
            count,
            p5 / n * 100.0,
            p10 / n * 100.0,
            rr / n
        );
    }

    // Split breakdown (train vs test)
    println!("\n━━━ By Split (unified) ━━━\n");
    println!(
        "{:<20} {:>6} {:>8} {:>8} {:>8}",
        "Split", "N", "P@5", "P@10", "MRR"
    );
    println!("{}", "─".repeat(54));

    for split_name in &["train", "test"] {
        if let Some((p5, p10, rr, count)) = split_stats.get(*split_name) {
            let n = *count as f32;
            println!(
                "{:<20} {:>6} {:>7.1}% {:>7.1}% {:>8.3}",
                split_name,
                count,
                p5 / n * 100.0,
                p10 / n * 100.0,
                rr / n
            );
        }
    }

    // --- Ablation: per-oracle contribution ---
    println!("\n━━━ Ablation: Per-Oracle Contribution ━━━\n");

    let oracles = ["semantic", "lexical", "temporal", "persona", "belief"];
    let mut ablation_results: Vec<NlMetrics> = Vec::new();

    // Unified baseline
    let unified_metrics = score_nl_engine(&unified, "unified (all)", &cases)?;
    ablation_results.push(unified_metrics.clone());

    // Each oracle in isolation
    for oracle_name in &oracles {
        let engine = QueryEngine::with_config(RetrievalConfig {
            oracle_filter: Some(vec![oracle_name.to_string()]),
            ..Default::default()
        });
        let metrics = score_nl_engine(&engine, &format!("{}-only", oracle_name), &cases)?;
        ablation_results.push(metrics);
    }

    // No-belief (all except belief)
    let no_belief = QueryEngine::with_config(RetrievalConfig {
        oracle_filter: Some(vec![
            "semantic".to_string(),
            "lexical".to_string(),
            "temporal".to_string(),
            "persona".to_string(),
        ]),
        ..Default::default()
    });
    let no_belief_metrics = score_nl_engine(&no_belief, "no-belief", &cases)?;
    ablation_results.push(no_belief_metrics);

    // Print ablation table
    println!(
        "{:<25} {:>8} {:>8} {:>8} {:>10}",
        "Pipeline", "P@5", "P@10", "MRR", "vs Unified"
    );
    println!("{}", "─".repeat(63));

    let baseline_p10 = unified_metrics.p10;
    for m in &ablation_results {
        let delta = if m.name == "unified (all)" {
            "—".to_string()
        } else {
            let d = (m.p10 - baseline_p10) * 100.0;
            format!("{:+.1}pp", d)
        };
        println!(
            "{:<25} {:>7.1}% {:>7.1}% {:>8.3} {:>10}",
            m.name,
            m.p5 * 100.0,
            m.p10 * 100.0,
            m.mrr,
            delta
        );
    }

    // Per-split ablation (unified only — quick view for tuning validation)
    let train_cases: Vec<&NlQueryCase> = cases.iter().filter(|c| c.split == "train").collect();
    let test_cases: Vec<&NlQueryCase> = cases.iter().filter(|c| c.split == "test").collect();

    if !train_cases.is_empty() && !test_cases.is_empty() {
        println!("\n━━━ Train vs Test (unified engine) ━━━\n");
        println!("{:<25} {:>8} {:>8} {:>8}", "Pipeline", "P@5", "P@10", "MRR");
        println!("{}", "─".repeat(53));

        let train_m = score_nl_engine_refs(&unified, "unified (train)", &train_cases)?;
        let test_m = score_nl_engine_refs(&unified, "unified (test)", &test_cases)?;

        for m in &[&train_m, &test_m] {
            println!(
                "{:<25} {:>7.1}% {:>7.1}% {:>8.3}",
                m.name,
                m.p5 * 100.0,
                m.p10 * 100.0,
                m.mrr,
            );
        }

        let delta_p10 = (test_m.p10 - train_m.p10) * 100.0;
        println!(
            "\n  Train-test gap: {:+.1}pp P@10 (negative = potential overfit)",
            delta_p10
        );
    }

    // Summary
    println!("\n━━━ Summary ━━━\n");
    println!(
        "  Queries:     {} ({} train, {} test)",
        cases.len(),
        train_count,
        test_count
    );
    println!("  Mean P@5:    {:.1}%", unified_metrics.p5 * 100.0);
    println!("  Mean P@10:   {:.1}%", unified_metrics.p10 * 100.0);
    println!("  MRR:         {:.3}", unified_metrics.mrr);

    // Emit measurement: NL eval unified metrics
    patina::measure::emit_or_warn(
        "search",
        "eval",
        "nl",
        &serde_json::json!({
            "p_at_5": unified_metrics.p5 as f64,
            "p_at_10": unified_metrics.p10 as f64,
            "mrr": unified_metrics.mrr as f64,
            "query_count": cases.len(),
        }),
    );

    Ok(())
}

// ============================================================================
// Post-split eval modes (Phase 4: Eval Redesign)
// ============================================================================

/// Independent assay eval — tests factual retrieval (FTS5) in isolation
pub fn execute_assay() -> Result<()> {
    internal::assay_eval::execute()
}

/// Independent scry eval — tests semantic retrieval (vectors) + scry-vs-assay comparison
pub fn execute_scry() -> Result<()> {
    internal::scry_eval::execute()
}

/// Raw E5 diagnostic — brute-force cosine without projection (Phase 5d)
pub fn execute_scry_raw() -> Result<()> {
    internal::scry_eval::execute_raw()
}

/// Combined eval — tests the full retrieval pipeline (assay + scry together)
pub fn execute_combined() -> Result<()> {
    internal::combined_eval::execute()
}
