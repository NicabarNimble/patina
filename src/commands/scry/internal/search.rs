//! Core search functions for scry command
//!
//! Implements semantic vector search, lexical FTS5 search, and file-based queries.

use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

use patina::embeddings::create_embedder;

use super::super::{ScryOptions, ScryResult};
use super::enrichment::{enrich_results, SearchResults};
use super::query_prep::prepare_fts_query;

/// Get database and embeddings paths (handles --repo flag)
pub fn get_paths(options: &ScryOptions) -> Result<(String, String)> {
    if let Some(ref repo_name) = options.repo {
        // For repos, model name is stored in repo's config (future: read from repo metadata)
        // For now, default to e5-base-v2 for repo queries
        let db_path = crate::commands::repo::get_db_path(repo_name)?;
        let embeddings_dir = db_path.replace("patina.db", "embeddings/e5-base-v2/projections");
        Ok((db_path, embeddings_dir))
    } else {
        // For local project, read model from config
        let model = get_embedding_model();
        Ok((
            ".patina/local/data/patina.db".to_string(),
            format!(".patina/local/data/embeddings/{}/projections", model),
        ))
    }
}

/// Get embedding model from project config (defaults to e5-base-v2)
pub fn get_embedding_model() -> String {
    patina::project::load(std::path::Path::new("."))
        .ok()
        .map(|c| c.embeddings.model)
        .unwrap_or_else(|| "e5-base-v2".to_string())
}

/// Text-based scry - embed query and search (for semantic dimension)
pub fn scry_text(query: &str, options: &ScryOptions) -> Result<Vec<ScryResult>> {
    let (db_path, embeddings_dir) = get_paths(options)?;

    // Determine which dimension to search
    // For reference repos, only dependency is available; for projects, prefer semantic
    let dimension = if let Some(ref dim) = options.dimension {
        dim.as_str()
    } else {
        // Auto-detect best available dimension
        detect_best_dimension(&embeddings_dir)
    };
    let index_path = format!("{}/{}.usearch", embeddings_dir, dimension);

    if !Path::new(&index_path).exists() {
        // Graceful fallback: index missing, use FTS5 instead
        eprintln!(
            "⚠️  {} index not found, falling back to lexical search (FTS5)",
            dimension
        );
        eprintln!("   Run 'patina oxidize' for vector search\n");
        println!("Mode: Lexical (FTS5) [fallback]\n");
        return scry_lexical(query, options);
    }

    // Create embedder and embed query
    println!("Embedding query...");
    let mut embedder = create_embedder()?;
    let query_embedding = embedder.embed_query(query)?;

    // Load projection and project query embedding
    let projection_path = format!("{}/{}.safetensors", embeddings_dir, dimension);
    let projected = if Path::new(&projection_path).exists() {
        use crate::commands::oxidize::trainer::Projection;
        let projection = Projection::load_safetensors(Path::new(&projection_path))?;
        projection.forward(&query_embedding)
    } else {
        query_embedding
    };

    // Search index
    println!("Searching {} index...", dimension);

    // Create index with matching options (256-dim projection output, cosine)
    let index_options = IndexOptions {
        dimensions: 256,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };

    let index = Index::new(&index_options).with_context(|| "Failed to create index")?;

    index
        .load(&index_path)
        .with_context(|| format!("Failed to load index: {}", index_path))?;

    let matches = index
        .search(&projected, options.limit)
        .with_context(|| "Vector search failed")?;

    // Convert to our SearchResults struct
    let results = SearchResults {
        keys: matches.keys,
        distances: matches.distances,
    };

    // Enrich with metadata from SQLite
    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    let enriched = enrich_results(&conn, &results, dimension, options.min_score)?;

    Ok(enriched)
}

/// File-based scry - look up file's vector and find neighbors (for temporal/dependency)
pub fn scry_file(file_path: &str, options: &ScryOptions) -> Result<Vec<ScryResult>> {
    let (db_path, embeddings_dir) = get_paths(options)?;

    // Default to temporal for file-based queries
    let dimension = options.dimension.as_deref().unwrap_or("temporal");
    let index_path = format!("{}/{}.usearch", embeddings_dir, dimension);

    if !Path::new(&index_path).exists() {
        anyhow::bail!(
            "Index not found: {}. Run 'patina oxidize' first.",
            index_path
        );
    }

    // Open database to find file index
    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Get list of files in the temporal index
    let files: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT file_a FROM co_changes
             UNION
             SELECT DISTINCT file_b FROM co_changes
             ORDER BY 1",
        )?;
        let mut rows = stmt.query([])?;
        let mut files = Vec::new();
        while let Some(row) = rows.next()? {
            files.push(row.get(0)?);
        }
        files
    };

    // Find the file's index position
    let file_index = files
        .iter()
        .position(|f| f == file_path || f.ends_with(file_path) || file_path.ends_with(f))
        .ok_or_else(|| anyhow::anyhow!("File '{}' not found in {} index", file_path, dimension))?;

    println!("Found file at index {} in {} index", file_index, dimension);

    // Load index
    let index_options = IndexOptions {
        dimensions: 256,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };

    let index = Index::new(&index_options).with_context(|| "Failed to create index")?;

    index
        .load(&index_path)
        .with_context(|| format!("Failed to load index: {}", index_path))?;

    // Get the file's existing vector from the index
    let mut file_vector = vec![0.0_f32; 256];
    index
        .get(file_index as u64, &mut file_vector)
        .with_context(|| format!("Failed to get vector for file index {}", file_index))?;

    println!("Searching for neighbors...");

    // Search for neighbors (request extra to filter out self)
    let matches = index
        .search(&file_vector, options.limit + 1)
        .with_context(|| "Vector search failed")?;

    // Build results, filtering out the query file itself
    let mut results = Vec::new();
    for i in 0..matches.keys.len() {
        let key = matches.keys[i] as usize;
        let distance = matches.distances[i];
        let score = 1.0 - distance;

        // Skip self
        if key == file_index {
            continue;
        }

        if score < options.min_score {
            continue;
        }

        if key < files.len() {
            let related_file = &files[key];
            results.push(ScryResult {
                id: key as i64,
                event_type: "file.cochange".to_string(),
                source_id: related_file.clone(),
                timestamp: String::new(),
                content: format!("Co-changes with: {}", file_path),
                score,
            });
        }

        if results.len() >= options.limit {
            break;
        }
    }

    // Sort by score descending
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(results)
}

/// Legacy alias for text-based scry
pub fn scry(query: &str, options: &ScryOptions) -> Result<Vec<ScryResult>> {
    scry_text(query, options)
}

/// Check if query looks like a lexical/exact-match query
///
/// This function gates the routing decision: lexical queries go to FTS5,
/// everything else to semantic vector search. It must be at least as
/// permissive as `is_code_like()` in query_prep.rs — otherwise code
/// patterns get routed to semantic mode where they produce noise.
pub fn is_lexical_query(query: &str) -> bool {
    let lower = query.to_lowercase();

    // Explicit lexical patterns (natural language triggers)
    lower.starts_with("find ")
        || lower.starts_with("where is ")
        || lower.starts_with("show me the ")
        || lower.starts_with("show me ")
        || lower.contains(" defined")
        // Code symbol patterns (original)
        || query.contains("::")
        || query.contains("()")
        || query.contains("fn ")
        || query.contains("struct ")
        || query.contains("const ")
        || query.contains("impl ")
        // Aligned with is_code_like() — these were missing and caused
        // insert_event, create_uid_if_missing, allow(dead_code) etc.
        // to fall through to semantic mode
        || (query.contains('_') && !query.contains(' '))  // snake_case without spaces
        || query.chars().all(|c| c.is_alphanumeric() || c == '_')  // single identifier
        || (query.contains('(') && query.contains(')'))  // parens (not just "()" pair)
        // Keyword at end of query (e.g., "async fn" has no trailing space)
        || lower.ends_with(" fn")
        || lower.ends_with(" struct")
}

/// Lexical search using FTS5 for exact matches
pub fn scry_lexical(query: &str, options: &ScryOptions) -> Result<Vec<ScryResult>> {
    let (db_path, _) = get_paths(options)?;

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Prepare the FTS5 query
    let fts_query = prepare_fts_query(query);

    println!("FTS5 query: {}", fts_query);

    let mut collected: Vec<ScryResult> = Vec::new();

    // 1. Search code_fts
    let event_type_filter = if options.include_issues {
        "event_type LIKE 'code.%' OR event_type = 'github.issue'"
    } else {
        "event_type LIKE 'code.%'"
    };

    let code_sql = format!(
        "SELECT
            symbol_name,
            file_path,
            snippet(code_fts, 2, '>>>', '<<<', '...', 64) as snippet,
            event_type,
            bm25(code_fts) as score
         FROM code_fts
         WHERE code_fts MATCH ?
           AND ({})
         ORDER BY score
         LIMIT ?",
        event_type_filter
    );

    let mut stmt = conn.prepare(&code_sql)?;
    let code_results =
        stmt.query_map(rusqlite::params![&fts_query, options.limit as i64], |row| {
            let symbol: String = row.get(0)?;
            let file_path: String = row.get(1)?;
            let snippet: String = row.get(2)?;
            let event_type: String = row.get(3)?;
            let bm25_score: f64 = row.get(4)?;

            // Use file_path directly - it's already source_id format (path::name)
            // Don't append symbol again (was causing path::name:name doubling)
            let source_id = if event_type == "github.issue" {
                format!("[ISSUE] {}", symbol)
            } else {
                file_path.clone()
            };

            Ok(ScryResult {
                id: 0,
                content: snippet,
                // BM25 is negative, convert to positive (don't cap - preserve ranking)
                score: -bm25_score as f32,
                event_type,
                source_id,
                timestamp: String::new(),
            })
        })?;
    collected.extend(code_results.filter_map(|r| r.ok()));

    // 2. Search commits_fts (git narrative)
    let commits_sql = "SELECT
            sha,
            snippet(commits_fts, 1, '>>>', '<<<', '...', 64) as snippet,
            author_name,
            bm25(commits_fts) as score
         FROM commits_fts
         WHERE commits_fts MATCH ?
         ORDER BY score
         LIMIT ?";

    if let Ok(mut stmt) = conn.prepare(commits_sql) {
        let commit_results =
            stmt.query_map(rusqlite::params![&fts_query, options.limit as i64], |row| {
                let sha: String = row.get(0)?;
                let snippet: String = row.get(1)?;
                let author: String = row.get(2)?;
                let bm25_score: f64 = row.get(3)?;

                Ok(ScryResult {
                    id: 0,
                    content: format!("{} ({})", snippet, author),
                    score: -bm25_score as f32,
                    event_type: "git.commit".to_string(),
                    source_id: sha,
                    timestamp: String::new(),
                })
            })?;
        collected.extend(commit_results.filter_map(|r| r.ok()));
    }

    // 3. Search pattern_fts (layer docs)
    let pattern_sql = "SELECT
            id,
            title,
            snippet(pattern_fts, 2, '>>>', '<<<', '...', 64) as snippet,
            file_path,
            bm25(pattern_fts) as score
         FROM pattern_fts
         WHERE pattern_fts MATCH ?
         ORDER BY score
         LIMIT ?";

    if let Ok(mut stmt) = conn.prepare(pattern_sql) {
        let pattern_results =
            stmt.query_map(rusqlite::params![&fts_query, options.limit as i64], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let snippet: String = row.get(2)?;
                let file_path: String = row.get(3)?;
                let bm25_score: f64 = row.get(4)?;

                // Determine layer from file path
                let layer = if file_path.contains("layer/core") {
                    "core"
                } else {
                    "surface"
                };

                Ok(ScryResult {
                    id: 0,
                    content: format!("{}: {}", title, snippet),
                    // BM25 is negative, convert to positive (don't cap - preserve ranking)
                    score: -bm25_score as f32,
                    event_type: format!("pattern.{}", layer),
                    source_id: id,
                    timestamp: String::new(),
                })
            })?;
        collected.extend(pattern_results.filter_map(|r| r.ok()));
    }

    // Sort by score (higher is better) and limit
    collected.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    collected.truncate(options.limit);

    // Filter by min_score
    collected.retain(|r| r.score >= options.min_score);

    Ok(collected)
}

/// Detect the best available dimension for vector search
/// Priority: semantic > dependency > temporal
/// Reference repos typically only have dependency
pub fn detect_best_dimension(embeddings_dir: &str) -> &'static str {
    // Check for available indices in priority order
    let semantic_path = format!("{}/semantic.usearch", embeddings_dir);
    if Path::new(&semantic_path).exists() {
        return "semantic";
    }

    let dependency_path = format!("{}/dependency.usearch", embeddings_dir);
    if Path::new(&dependency_path).exists() {
        return "dependency";
    }

    let temporal_path = format!("{}/temporal.usearch", embeddings_dir);
    if Path::new(&temporal_path).exists() {
        return "temporal";
    }

    // Default to semantic (will trigger fallback to FTS5)
    "semantic"
}
