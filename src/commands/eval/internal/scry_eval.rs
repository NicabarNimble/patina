//! Independent scry eval — tests semantic retrieval (vector search) in isolation
//!
//! Calls QueryEngine::query() directly, measures P@5, P@10, MRR against expected
//! beliefs and patterns. Includes scry-vs-assay comparison to prove semantic adds
//! value beyond keyword search (Phase 4 exit criterion: ≥5/20 scry-only hits).
//!
//! Phase 5d: `execute_raw()` — brute-force cosine over raw E5-base-v2 (768-dim)
//! embeddings without projection. Diagnostic to determine if projection helps,
//! hurts, or is neutral compared to the base model.

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::commands::assay::{assay_search, SearchOptions};
use crate::commands::oxidize;
use crate::retrieval::QueryEngine;
use patina::embeddings::create_embedder;

use super::helpers::{
    compute_metrics, extract_file_from_doc_id, normalize_path, print_metrics,
    print_per_query_detail, QueryCase,
};

/// Execute independent scry eval + scry-vs-assay comparison
pub fn execute() -> Result<()> {
    println!("📊 Scry Eval — Independent Semantic Retrieval\n");
    println!("Testing vector search quality (scry only, no FTS5)...\n");

    let test_path = "resources/eval/scry-queries.json";
    let content = std::fs::read_to_string(test_path).context(format!("Cannot read {test_path}"))?;
    let cases: Vec<QueryCase> =
        serde_json::from_str(&content).context("Failed to parse scry-queries.json")?;

    let train_count = cases.iter().filter(|c| c.split == "train").count();
    let test_count = cases.iter().filter(|c| c.split == "test").count();
    println!(
        "Loaded {} queries ({} train, {} test)\n",
        cases.len(),
        train_count,
        test_count
    );

    let engine = QueryEngine::new();

    // Query function for scry: return doc_ids from semantic search
    let scry_fn = |q: &str| -> Vec<String> {
        match engine.query(q, 10) {
            Ok(results) => results.into_iter().map(|r| r.doc_id).collect(),
            Err(_) => Vec::new(),
        }
    };

    // Per-query detail
    println!("━━━ Per-Query Detail (Scry) ━━━\n");
    print_per_query_detail(&cases, &scry_fn);

    // Overall scry metrics
    let scry_metrics = compute_metrics(&cases, &scry_fn, "scry (all)");
    println!("\n━━━ Scry Overall ━━━\n");
    print_metrics(&scry_metrics);

    // Train/test split
    let train_cases: Vec<QueryCase> = cases
        .iter()
        .filter(|c| c.split == "train")
        .map(|c| QueryCase {
            query: c.query.clone(),
            expected: c.expected.clone(),
            category: c.category.clone(),

            split: c.split.clone(),
        })
        .collect();
    let test_cases: Vec<QueryCase> = cases
        .iter()
        .filter(|c| c.split == "test")
        .map(|c| QueryCase {
            query: c.query.clone(),
            expected: c.expected.clone(),
            category: c.category.clone(),

            split: c.split.clone(),
        })
        .collect();

    if !train_cases.is_empty() && !test_cases.is_empty() {
        let train_m = compute_metrics(&train_cases, &scry_fn, "scry (train)");
        let test_m = compute_metrics(&test_cases, &scry_fn, "scry (test)");

        println!("\n━━━ Train vs Test (Scry) ━━━\n");
        println!(
            "{:<25} {:>6} {:>8} {:>8} {:>8}",
            "Split", "N", "P@5", "P@10", "MRR"
        );
        println!("{}", "─".repeat(58));
        for m in [&train_m, &test_m] {
            println!(
                "{:<25} {:>6} {:>7.1}% {:>7.1}% {:>8.3}",
                m.name,
                m.num_queries,
                m.p5 * 100.0,
                m.p10 * 100.0,
                m.mrr,
            );
        }
    }

    // ================================================================
    // Scry-vs-Assay comparison (Phase 4 exit criterion: ≥5/20)
    // ================================================================
    println!("\n━━━ Scry vs Assay Comparison ━━━\n");
    println!("Running same conceptual queries through both systems...\n");

    // Assay query function
    let assay_fn = |q: &str| -> Vec<String> {
        let options = SearchOptions {
            limit: 10,
            include_issues: false,
            repo: None,
        };
        match assay_search(q, &options) {
            Ok(results) => results.into_iter().map(|r| r.source_id).collect(),
            Err(_) => Vec::new(),
        }
    };

    let mut scry_only_hits = 0usize;
    let mut assay_only_hits = 0usize;
    let mut both_hit = 0usize;
    let mut both_miss = 0usize;

    println!("{:<55} {:>10} {:>10}", "Query", "Scry", "Assay");
    println!("{}", "─".repeat(77));

    for case in &cases {
        let expected: std::collections::HashSet<String> =
            case.expected.iter().map(|p| normalize_path(p)).collect();

        let scry_results = scry_fn(&case.query);
        let assay_results = assay_fn(&case.query);

        let scry_hit = scry_results
            .iter()
            .take(10)
            .any(|id| expected.contains(&extract_file_from_doc_id(id)));
        let assay_hit = assay_results
            .iter()
            .take(10)
            .any(|id| expected.contains(&extract_file_from_doc_id(id)));

        match (scry_hit, assay_hit) {
            (true, false) => scry_only_hits += 1,
            (false, true) => assay_only_hits += 1,
            (true, true) => both_hit += 1,
            (false, false) => both_miss += 1,
        }

        let scry_str = if scry_hit { "HIT" } else { "miss" };
        let assay_str = if assay_hit { "HIT" } else { "miss" };

        let display_q = if case.query.len() > 53 {
            format!("{}...", &case.query[..50])
        } else {
            case.query.clone()
        };
        println!("{:<55} {:>10} {:>10}", display_q, scry_str, assay_str);
    }

    let total = cases.len();
    println!("\n━━━ Comparison Summary ━━━\n");
    println!(
        "  Scry HIT, Assay miss:  {} / {} queries",
        scry_only_hits, total
    );
    println!("  Both HIT:              {} / {} queries", both_hit, total);
    println!(
        "  Assay HIT, Scry miss:  {} / {} queries",
        assay_only_hits, total
    );
    println!("  Both miss:             {} / {} queries", both_miss, total);

    // Phase 4 exit criterion
    let criterion_met = scry_only_hits >= 5;
    println!(
        "\n  Phase 4 criterion (scry finds ≥5/20 that assay misses): {} ({}/20)",
        if criterion_met { "PASS" } else { "FAIL" },
        scry_only_hits
    );

    // Summary
    println!("\n━━━ Summary ━━━\n");
    println!("  Scry Mean P@5:    {:.1}%", scry_metrics.p5 * 100.0);
    println!("  Scry Mean P@10:   {:.1}%", scry_metrics.p10 * 100.0);
    println!("  Scry MRR:         {:.3}", scry_metrics.mrr);
    println!(
        "  Scry-only value:  {} queries where semantic finds answers FTS5 misses",
        scry_only_hits
    );

    Ok(())
}

/// Raw E5-base-v2 diagnostic — brute-force cosine without projection (Phase 5d)
///
/// Embeds the knowledge corpus with raw 768-dim E5 vectors (no trained projection),
/// then runs the 20 scry eval queries against this corpus using brute-force cosine
/// similarity. Compares against the projected results to determine:
/// - Projection HELPS: raw < projected (projection adds value)
/// - Projection HURTS: raw > projected (projection adds noise)
/// - Projection NEUTRAL: raw ≈ projected (projection irrelevant)
pub fn execute_raw() -> Result<()> {
    println!("📊 Raw E5 Diagnostic — No Projection Baseline (Phase 5d)\n");
    println!("Comparing raw E5-base-v2 (768-dim) vs projected (256-dim)...\n");

    // Load eval queries
    let test_path = "resources/eval/scry-queries.json";
    let content = std::fs::read_to_string(test_path).context(format!("Cannot read {test_path}"))?;
    let cases: Vec<QueryCase> =
        serde_json::from_str(&content).context("Failed to parse scry-queries.json")?;

    println!("Loaded {} eval queries\n", cases.len());

    // Load knowledge corpus from DB
    let db_path = patina::eventlog::patina_db_path()?;
    let conn = Connection::open(db_path).context("Cannot open database")?;
    let corpus = oxidize::query_knowledge_corpus(&conn)?;
    println!("Knowledge corpus: {} items\n", corpus.len());

    if corpus.is_empty() {
        println!("No corpus items — run `patina oxidize` first.");
        return Ok(());
    }

    // Build key→doc_id map from DB
    let key_to_doc_id = build_key_to_doc_id(&conn, &corpus)?;

    // Create embedder and embed corpus
    println!("🔮 Embedding corpus with raw E5 (this takes ~30-60 seconds)...");
    let mut embedder = create_embedder()?;

    let mut corpus_embeddings: Vec<(i64, Vec<f32>)> = Vec::with_capacity(corpus.len());
    for (i, (key, text)) in corpus.iter().enumerate() {
        let embedding = embedder.embed_passage(text)?;
        corpus_embeddings.push((*key, embedding));
        if (i + 1) % 100 == 0 {
            println!("   Embedded {}/{} items...", i + 1, corpus.len());
        }
    }
    println!("   Embedded all {} items\n", corpus_embeddings.len());

    // Pre-compute all query embeddings (avoids &mut borrow in closure)
    println!("   Embedding {} eval queries...", cases.len());
    let mut query_embeddings: std::collections::HashMap<String, Vec<f32>> =
        std::collections::HashMap::new();
    for case in &cases {
        let emb = embedder.embed_query(&case.query)?;
        query_embeddings.insert(case.query.clone(), emb);
    }
    println!("   Done.\n");

    // Raw E5 query function: brute-force cosine over 768-dim embeddings
    let raw_fn = |q: &str| -> Vec<String> {
        let query_embedding = match query_embeddings.get(q) {
            Some(e) => e,
            None => return Vec::new(),
        };

        // Compute cosine similarity against all corpus items
        let mut scores: Vec<(i64, f32)> = corpus_embeddings
            .iter()
            .map(|(key, emb)| (*key, cosine_similarity(query_embedding, emb)))
            .collect();

        // Sort by similarity descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top-10 doc_ids
        scores
            .into_iter()
            .take(10)
            .filter_map(|(key, _score)| key_to_doc_id.get(&key).cloned())
            .collect()
    };

    // Run raw E5 eval
    println!("━━━ Per-Query Detail (Raw E5, 768-dim) ━━━\n");
    print_per_query_detail(&cases, &raw_fn);

    let raw_metrics = compute_metrics(&cases, &raw_fn, "raw E5 (768-dim)");
    println!("\n━━━ Raw E5 Overall ━━━\n");
    print_metrics(&raw_metrics);

    // Run projected eval for comparison
    println!("\n━━━ Projected Comparison ━━━\n");
    let engine = QueryEngine::new();
    let proj_fn = |q: &str| -> Vec<String> {
        match engine.query(q, 10) {
            Ok(results) => results.into_iter().map(|r| r.doc_id).collect(),
            Err(_) => Vec::new(),
        }
    };
    let proj_metrics = compute_metrics(&cases, &proj_fn, "projected (256-dim)");
    print_metrics(&proj_metrics);

    // Side-by-side comparison
    println!("\n━━━ Raw vs Projected Comparison ━━━\n");

    println!("{:<55} {:>10} {:>10}", "Query", "Raw E5", "Projected");
    println!("{}", "─".repeat(77));

    let mut raw_only = 0usize;
    let mut proj_only = 0usize;
    let mut both_hit = 0usize;
    let mut both_miss = 0usize;

    for case in &cases {
        let expected: std::collections::HashSet<String> =
            case.expected.iter().map(|p| normalize_path(p)).collect();

        let raw_results = raw_fn(&case.query);
        let proj_results = proj_fn(&case.query);

        let raw_hit = raw_results
            .iter()
            .take(10)
            .any(|id| expected.contains(&extract_file_from_doc_id(id)));
        let proj_hit = proj_results
            .iter()
            .take(10)
            .any(|id| expected.contains(&extract_file_from_doc_id(id)));

        match (raw_hit, proj_hit) {
            (true, false) => raw_only += 1,
            (false, true) => proj_only += 1,
            (true, true) => both_hit += 1,
            (false, false) => both_miss += 1,
        }

        let raw_str = if raw_hit { "HIT" } else { "miss" };
        let proj_str = if proj_hit { "HIT" } else { "miss" };

        let display_q = if case.query.len() > 53 {
            format!("{}...", &case.query[..50])
        } else {
            case.query.clone()
        };
        println!("{:<55} {:>10} {:>10}", display_q, raw_str, proj_str);
    }

    let total = cases.len();
    println!("\n━━━ Diagnostic Summary ━━━\n");
    println!("  Raw E5 HIT, Proj miss:  {} / {}", raw_only, total);
    println!("  Both HIT:               {} / {}", both_hit, total);
    println!("  Proj HIT, Raw miss:     {} / {}", proj_only, total);
    println!("  Both miss:              {} / {}", both_miss, total);

    println!(
        "\n  {:>25} {:>8} {:>8} {:>8} {:>8}",
        "Method", "P@5", "P@10", "MRR", "Hits"
    );
    println!("  {}", "─".repeat(58));
    println!(
        "  {:>25} {:>7.1}% {:>7.1}% {:>8.3} {:>7.1}%",
        "Raw E5 (768-dim)",
        raw_metrics.p5 * 100.0,
        raw_metrics.p10 * 100.0,
        raw_metrics.mrr,
        raw_metrics.hit_rate * 100.0,
    );
    println!(
        "  {:>25} {:>7.1}% {:>7.1}% {:>8.3} {:>7.1}%",
        "Projected (256-dim)",
        proj_metrics.p5 * 100.0,
        proj_metrics.p10 * 100.0,
        proj_metrics.mrr,
        proj_metrics.hit_rate * 100.0,
    );

    let delta_p10 = (raw_metrics.p10 - proj_metrics.p10) * 100.0;
    let delta_hits = (raw_metrics.hit_rate - proj_metrics.hit_rate) * 100.0;

    println!("\n  Verdict:");
    if delta_p10 > 2.0 {
        println!(
            "  Projection HURTS — raw E5 is {:.1}pp better at P@10",
            delta_p10
        );
        println!("  The trained projection adds noise to E5's embedding space.");
    } else if delta_p10 < -2.0 {
        println!(
            "  Projection HELPS — projected is {:.1}pp better at P@10",
            -delta_p10
        );
        println!("  The trained projection improves over raw E5 embeddings.");
    } else {
        println!(
            "  Projection NEUTRAL — delta is only {:.1}pp P@10",
            delta_p10.abs()
        );
        println!("  The projection neither helps nor hurts meaningfully.");
        if delta_hits.abs() > 5.0 {
            println!(
                "  (But hit rate differs by {:.1}pp — worth investigating)",
                delta_hits.abs()
            );
        }
    }

    Ok(())
}

/// Build a map from corpus key to doc_id (for result enrichment in raw E5 eval)
///
/// Uses ID offsets from `patina::embeddings::offsets`.
fn build_key_to_doc_id(
    conn: &Connection,
    corpus: &[(i64, String)],
) -> Result<std::collections::HashMap<i64, String>> {
    use patina::embeddings::offsets::{BELIEF_ID_OFFSET, COMMIT_ID_OFFSET, PATTERN_ID_OFFSET};

    let mut map = std::collections::HashMap::new();

    for (key, _) in corpus {
        let key = *key;
        if key >= BELIEF_ID_OFFSET {
            let rowid = key - BELIEF_ID_OFFSET;
            if let Ok(id) =
                conn.query_row("SELECT id FROM beliefs WHERE rowid = ?", [rowid], |row| {
                    row.get::<_, String>(0)
                })
            {
                map.insert(key, id);
            }
        } else if key >= COMMIT_ID_OFFSET {
            let rowid = key - COMMIT_ID_OFFSET;
            if let Ok(sha) =
                conn.query_row("SELECT sha FROM commits WHERE rowid = ?", [rowid], |row| {
                    row.get::<_, String>(0)
                })
            {
                map.insert(key, sha);
            }
        } else if key >= PATTERN_ID_OFFSET {
            let rowid = key - PATTERN_ID_OFFSET;
            if let Ok(file_path) = conn.query_row(
                "SELECT file_path FROM patterns WHERE rowid = ?",
                [rowid],
                |row| row.get::<_, String>(0),
            ) {
                map.insert(key, file_path);
            }
        }
    }

    Ok(map)
}

/// Cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < 1e-8 || norm_b < 1e-8 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
