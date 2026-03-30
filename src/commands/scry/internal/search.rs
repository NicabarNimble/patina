//! Core search functions for scry command
//!
//! Implements semantic vector search, belief grounding, and file-based queries.
//! Lexical FTS5 search lives in assay (see src/commands/assay/internal/search.rs).

use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

use patina::embeddings::create_embedder;

use super::super::{ScryOptions, ScryResult};
use super::enrichment::{enrich_results, SearchResults};

/// Get database and embeddings paths (handles --repo flag)
pub fn get_paths(options: &ScryOptions) -> Result<(String, String)> {
    if let Some(ref repo_name) = options.repo {
        let repo_root = crate::commands::repo::get_path(repo_name)?;
        let model = patina::project::load(&repo_root)
            .ok()
            .map(|c| c.embeddings.model)
            .unwrap_or_else(|| "e5-base-v2".to_string());
        let db_path = crate::commands::repo::get_db_path(repo_name)?;
        let embeddings_dir = patina::paths::project::model_projections_dir(&repo_root, &model)
            .to_string_lossy()
            .to_string();
        Ok((db_path, embeddings_dir))
    } else {
        // For local project, read model from config
        let model = get_embedding_model();
        let project_root = std::env::current_dir()?;
        let db_path = patina::eventlog::resolve_patina_db_path(&project_root)
            .to_string_lossy()
            .to_string();
        Ok((
            db_path,
            patina::paths::project::model_projections_dir(&project_root, &model)
                .to_string_lossy()
                .to_string(),
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
    // For projects, prefer knowledge domain; reference repos may only have dependency
    let dimension = if let Some(ref dim) = options.dimension {
        dim.as_str()
    } else {
        detect_best_dimension(&embeddings_dir)
    };
    let index_path = format!("{}/{}.usearch", embeddings_dir, dimension);

    if !Path::new(&index_path).exists() {
        anyhow::bail!(
            "Semantic index not found: {}\n\
             Run 'patina oxidize' to build the knowledge domain index.\n\
             For keyword search, use 'patina assay search <query>' instead.",
            index_path
        );
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

/// Belief-based scry - look up belief's vector and find neighbors across all content types
pub fn scry_belief(belief_id: &str, options: &ScryOptions) -> Result<Vec<ScryResult>> {
    let (db_path, embeddings_dir) = get_paths(options)?;

    // Look up belief rowid from beliefs table
    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    let rowid: i64 = conn
        .query_row(
            "SELECT rowid FROM beliefs WHERE id = ?",
            [belief_id],
            |row| row.get(0),
        )
        .with_context(|| format!("Belief '{}' not found in database", belief_id))?;

    use patina::embeddings::offsets::BELIEF_ID_OFFSET;
    let belief_index = (BELIEF_ID_OFFSET + rowid) as u64;

    // Load knowledge/semantic index (beliefs live in vector space)
    let dimension = detect_best_dimension(&embeddings_dir);
    let index_path = format!("{}/{}.usearch", embeddings_dir, dimension);
    if !Path::new(&index_path).exists() {
        anyhow::bail!(
            "Semantic index not found: {}. Run 'patina oxidize' first.",
            index_path
        );
    }

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

    // Get the belief's existing vector from the index
    let mut belief_vector = vec![0.0_f32; 256];
    index
        .get(belief_index, &mut belief_vector)
        .with_context(|| {
            format!(
                "Failed to get vector for belief '{}' (index {})",
                belief_id, belief_index
            )
        })?;

    println!("Searching for neighbors of belief '{}'...", belief_id);

    // Request extra results to account for self-filtering (belief + pattern entries)
    // and type filtering (code may be sparse in top results)
    let search_limit = if options.content_type.is_some() {
        options.limit * 5 + 2
    } else {
        options.limit + 2 // +2 for both belief.surface and pattern.surface self-entries
    };

    let matches = index
        .search(&belief_vector, search_limit)
        .with_context(|| "Vector search failed")?;

    // Build SearchResults for enrichment
    let results = SearchResults {
        keys: matches.keys,
        distances: matches.distances,
    };

    // Enrich with metadata from SQLite
    let mut enriched = enrich_results(&conn, &results, dimension, options.min_score)?;

    // Filter out self — belief appears as both belief.surface and pattern.surface
    // Pattern source_id is now file_path (e.g., "layer/surface/epistemic/beliefs/foo.md")
    // so match on either exact id or file_path containing the belief_id
    enriched.retain(|r| {
        if r.event_type == "belief.surface" && r.source_id == belief_id {
            return false; // Exact belief match
        }
        if r.event_type.starts_with("pattern.") && r.source_id.contains(belief_id) {
            return false; // Pattern entry for this belief (file_path contains id)
        }
        true
    });

    // Apply content type filter if specified
    if let Some(ref type_filter) = options.content_type {
        enriched.retain(|r| match type_filter.as_str() {
            "code" => r.event_type.starts_with("code."),
            "commits" => r.event_type == "git.commit",
            "sessions" => r.event_type.starts_with("session."),
            "patterns" => r.event_type.starts_with("pattern."),
            "beliefs" => r.event_type == "belief.surface",
            _ => true,
        });
    }

    enriched.truncate(options.limit);
    Ok(enriched)
}

/// Legacy alias for text-based scry
pub fn scry(query: &str, options: &ScryOptions) -> Result<Vec<ScryResult>> {
    scry_text(query, options)
}

/// Detect the best available dimension for vector search
/// Priority: knowledge > semantic > dependency > temporal
/// Matches SemanticOracle's preference (knowledge domain first, legacy semantic fallback)
pub fn detect_best_dimension(embeddings_dir: &str) -> &'static str {
    // Knowledge domain (Phase 2+) — beliefs + patterns + commits
    let knowledge_path = format!("{}/knowledge.usearch", embeddings_dir);
    if Path::new(&knowledge_path).exists() {
        return "knowledge";
    }

    // Legacy semantic index (pre-split, session-polluted)
    let semantic_path = format!("{}/semantic.usearch", embeddings_dir);
    if Path::new(&semantic_path).exists() {
        return "semantic";
    }

    // Reference repos typically only have dependency
    let dependency_path = format!("{}/dependency.usearch", embeddings_dir);
    if Path::new(&dependency_path).exists() {
        return "dependency";
    }

    let temporal_path = format!("{}/temporal.usearch", embeddings_dir);
    if Path::new(&temporal_path).exists() {
        return "temporal";
    }

    // Default to knowledge (will trigger error with guidance)
    "knowledge"
}
