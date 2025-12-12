//! Scry command - Query knowledge using vector search
//!
//! Unified query interface for searching project knowledge.
//! Phase 2.5b: MVP implementation for validating retrieval quality.
//!
//! # Remote Execution
//! If `PATINA_MOTHERSHIP` is set, queries are routed to a remote daemon.
//! This enables containers to query the Mac mothership.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

use crate::commands::persona;
use patina::embeddings::create_embedder;
use patina::mothership;

/// Result from a scry query
#[derive(Debug)]
pub struct ScryResult {
    pub id: i64,
    pub content: String,
    pub score: f32,
    pub event_type: String,
    pub source_id: String,
    pub timestamp: String,
}

/// Options for scry query
#[derive(Debug, Clone)]
pub struct ScryOptions {
    pub limit: usize,
    pub min_score: f32,
    pub dimension: Option<String>,
    pub file: Option<String>,
    pub repo: Option<String>,
    pub all_repos: bool,
    pub include_issues: bool,
    pub include_persona: bool,
}

impl Default for ScryOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            min_score: 0.0,
            dimension: None, // Use semantic by default
            file: None,
            repo: None,
            all_repos: false,
            include_issues: false,
            include_persona: true, // Include persona by default
        }
    }
}

/// Execute scry command
pub fn execute(query: Option<&str>, options: ScryOptions) -> Result<()> {
    // Check if we should route to mothership
    if mothership::is_configured() {
        return execute_via_mothership(query, &options);
    }

    println!("🔮 Scry - Searching knowledge base\n");

    // Handle --all-repos mode
    if options.all_repos {
        return execute_all_repos(query, &options);
    }

    // Show repo context if specified
    if let Some(ref repo) = options.repo {
        println!("Repo: {}", repo);
    }

    // Show if including issues
    if options.include_issues {
        println!("Including: GitHub issues");
    }
    println!();

    // Determine query mode
    let mut results = match (&options.file, query) {
        (Some(file), _) => {
            println!("File: {}\n", file);
            scry_file(file, &options)?
        }
        (None, Some(q)) => {
            println!("Query: \"{}\"\n", q);

            // If dimension explicitly specified, use vector search (skip lexical auto-detect)
            if options.dimension.is_some() {
                println!(
                    "Mode: Vector ({} dimension)\n",
                    options.dimension.as_deref().unwrap()
                );
                scry_text(q, &options)?
            } else if is_lexical_query(q) {
                // Auto-detect lexical patterns only when no dimension specified
                println!("Mode: Lexical (FTS5)\n");
                scry_lexical(q, &options)?
            } else {
                println!("Mode: Semantic (vector)\n");
                scry_text(q, &options)?
            }
        }
        (None, None) => {
            anyhow::bail!("Either a query text or --file must be provided");
        }
    };

    // Query persona if enabled and we have a text query
    if options.include_persona {
        if let Some(q) = query {
            if let Ok(persona_results) = persona::query(q, options.limit, options.min_score, None) {
                for p in persona_results {
                    results.push(ScryResult {
                        id: 0,
                        content: p.content,
                        score: p.score,
                        event_type: "[PERSONA]".to_string(),
                        source_id: format!("{} ({})", p.source, p.domains.join(", ")),
                        timestamp: p.timestamp,
                    });
                }
            }
        }
    }

    // Sort combined results by score
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(options.limit);

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!("Found {} results:\n", results.len());
    println!("{}", "─".repeat(60));

    for (i, result) in results.iter().enumerate() {
        let timestamp_display = if result.timestamp.is_empty() {
            String::new()
        } else {
            format!(" | {}", result.timestamp)
        };
        println!(
            "\n[{}] Score: {:.3} | {} | {}{}",
            i + 1,
            result.score,
            result.event_type,
            result.source_id,
            timestamp_display
        );
        println!("    {}", truncate_content(&result.content, 200));
    }

    println!("\n{}", "─".repeat(60));

    Ok(())
}

/// Get database and embeddings paths (handles --repo flag)
fn get_paths(options: &ScryOptions) -> Result<(String, String)> {
    if let Some(ref repo_name) = options.repo {
        let db_path = crate::commands::repo::get_db_path(repo_name)?;
        let embeddings_dir = db_path.replace("patina.db", "embeddings/e5-base-v2/projections");
        Ok((db_path, embeddings_dir))
    } else {
        Ok((
            ".patina/data/patina.db".to_string(),
            ".patina/data/embeddings/e5-base-v2/projections".to_string(),
        ))
    }
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
pub fn is_lexical_query(query: &str) -> bool {
    let lower = query.to_lowercase();

    // Explicit lexical patterns
    lower.starts_with("find ")
        || lower.starts_with("where is ")
        || lower.starts_with("show me the ")
        || lower.starts_with("show me ")
        || lower.contains(" defined")
        // Code symbol patterns
        || query.contains("::")
        || query.contains("()")
        || query.contains("fn ")
        || query.contains("struct ")
        || query.contains("const ")
        || query.contains("impl ")
}

/// Lexical search using FTS5 for exact matches
pub fn scry_lexical(query: &str, options: &ScryOptions) -> Result<Vec<ScryResult>> {
    let (db_path, _) = get_paths(options)?;

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Prepare the FTS5 query
    let fts_query = prepare_fts_query(query);

    println!("FTS5 query: {}", fts_query);

    // Build event type filter based on include_issues flag
    let event_type_filter = if options.include_issues {
        // Include both code and github events
        "event_type LIKE 'code.%' OR event_type = 'github.issue'"
    } else {
        // Code events only (default)
        "event_type LIKE 'code.%'"
    };

    // Search using FTS5 with BM25 scoring
    let sql = format!(
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

    let mut stmt = conn.prepare(&sql)?;

    let results = stmt.query_map(rusqlite::params![&fts_query, options.limit as i64], |row| {
        let symbol: String = row.get(0)?;
        let file_path: String = row.get(1)?;
        let snippet: String = row.get(2)?;
        let event_type: String = row.get(3)?;
        let bm25_score: f64 = row.get(4)?;

        // Format source_id based on event type
        let source_id = if event_type == "github.issue" {
            format!("[ISSUE] {}", symbol) // symbol contains issue title for github events
        } else {
            format!("{}:{}", file_path, symbol)
        };

        Ok(ScryResult {
            id: 0,
            content: snippet,
            // BM25 is negative (lower = better), convert to positive score
            score: (-bm25_score as f32).min(1.0),
            event_type,
            source_id,
            timestamp: String::new(),
        })
    })?;

    let mut collected: Vec<ScryResult> = results.filter_map(|r| r.ok()).collect();

    // Filter by min_score
    collected.retain(|r| r.score >= options.min_score);

    Ok(collected)
}

/// Prepare query for FTS5 (strip prefixes, quote if needed)
fn prepare_fts_query(query: &str) -> String {
    // Strip common prefixes
    let cleaned = query
        .trim()
        .trim_start_matches("find ")
        .trim_start_matches("where is ")
        .trim_start_matches("show me the ")
        .trim_start_matches("show me ")
        .trim();

    // If it contains special characters, use phrase search
    if cleaned.contains("::") || cleaned.contains("()") || cleaned.contains(' ') {
        format!("\"{}\"", cleaned)
    } else {
        cleaned.to_string()
    }
}

/// Search results from USearch
struct SearchResults {
    keys: Vec<u64>,
    distances: Vec<f32>,
}

/// Enrich vector search results with SQLite metadata
fn enrich_results(
    conn: &Connection,
    results: &SearchResults,
    dimension: &str,
    min_score: f32,
) -> Result<Vec<ScryResult>> {
    let mut enriched = Vec::new();

    // Code facts use ID offset to distinguish from eventlog entries
    const CODE_ID_OFFSET: i64 = 1_000_000_000;

    match dimension {
        "semantic" => {
            // Semantic index contains both eventlog entries and code facts
            for i in 0..results.keys.len() {
                let key = results.keys[i] as i64;
                let distance = results.distances[i];
                // Convert distance to similarity score (cosine: 1 - distance)
                let score = 1.0 - distance;

                if score < min_score {
                    continue;
                }

                // Check if this is a code fact (ID >= offset) or eventlog entry
                if key >= CODE_ID_OFFSET {
                    // Code fact - look up in function_facts
                    let rowid = key - CODE_ID_OFFSET;
                    let result = conn.query_row(
                        "SELECT rowid, file, name, parameters, return_type, is_public, is_async
                         FROM function_facts
                         WHERE rowid = ?",
                        [rowid],
                        |row| {
                            let file: String = row.get(1)?;
                            let name: String = row.get(2)?;
                            let params: Option<String> = row.get(3)?;
                            let return_type: Option<String> = row.get(4)?;
                            let is_public: bool = row.get(5)?;
                            let is_async: bool = row.get(6)?;

                            // Reconstruct the description
                            let mut desc = format!("Function `{}` in `{}`", name, file);
                            if is_public {
                                desc.push_str(", public");
                            }
                            if is_async {
                                desc.push_str(", async");
                            }
                            if let Some(p) = params {
                                if !p.is_empty() {
                                    desc.push_str(&format!(", params: {}", p));
                                }
                            }
                            if let Some(rt) = return_type {
                                if !rt.is_empty() {
                                    desc.push_str(&format!(", returns: {}", rt));
                                }
                            }

                            Ok(ScryResult {
                                id: key,
                                event_type: "code.function".to_string(),
                                source_id: format!("{}:{}", file, name),
                                timestamp: String::new(),
                                content: desc,
                                score,
                            })
                        },
                    );

                    if let Ok(r) = result {
                        enriched.push(r);
                    }
                } else {
                    // Eventlog entry
                    let result = conn.query_row(
                        "SELECT seq, event_type, source_id, timestamp,
                                json_extract(data, '$.content') as content
                         FROM eventlog
                         WHERE seq = ?",
                        [key],
                        |row| {
                            Ok(ScryResult {
                                id: row.get(0)?,
                                event_type: row.get(1)?,
                                source_id: row.get(2)?,
                                timestamp: row.get(3)?,
                                content: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                                score,
                            })
                        },
                    );

                    if let Ok(r) = result {
                        enriched.push(r);
                    }
                }
            }
        }
        "temporal" => {
            // Temporal index uses sequential file index as key
            // Need to look up file path from the index
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

            for i in 0..results.keys.len() {
                let key = results.keys[i] as usize;
                let distance = results.distances[i];
                let score = 1.0 - distance;

                if score < min_score {
                    continue;
                }

                if key < files.len() {
                    let file_path = &files[key];
                    enriched.push(ScryResult {
                        id: key as i64,
                        event_type: "file.cochange".to_string(),
                        source_id: file_path.clone(),
                        timestamp: String::new(),
                        content: format!("File: {} (temporal co-change relationship)", file_path),
                        score,
                    });
                }
            }
        }
        "dependency" => {
            // Dependency index uses sequential function index as key
            // Need to look up function name from call_graph
            let functions: Vec<String> = {
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT caller FROM call_graph
                     UNION
                     SELECT DISTINCT callee FROM call_graph
                     ORDER BY 1",
                )?;
                let mut rows = stmt.query([])?;
                let mut funcs = Vec::new();
                while let Some(row) = rows.next()? {
                    funcs.push(row.get(0)?);
                }
                funcs
            };

            for i in 0..results.keys.len() {
                let key = results.keys[i] as usize;
                let distance = results.distances[i];
                let score = 1.0 - distance;

                if score < min_score {
                    continue;
                }

                if key < functions.len() {
                    let func_name = &functions[key];
                    enriched.push(ScryResult {
                        id: key as i64,
                        event_type: "function.dependency".to_string(),
                        source_id: func_name.clone(),
                        timestamp: String::new(),
                        content: format!("Function: {} (dependency relationship)", func_name),
                        score,
                    });
                }
            }
        }
        _ => {
            anyhow::bail!("Unknown dimension: {}", dimension);
        }
    }

    // Sort by score descending
    enriched.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(enriched)
}

/// Execute query across all repos (current project + all reference repos)
fn execute_all_repos(query: Option<&str>, options: &ScryOptions) -> Result<()> {
    let query = query.ok_or_else(|| anyhow::anyhow!("Query required for --all-repos"))?;

    println!("Mode: All Repos (cross-project search)\n");
    println!("Query: \"{}\"\n", query);

    let mut all_results: Vec<(String, ScryResult)> = Vec::new();

    // 1. Query current project if we're in one
    let in_project = Path::new(".patina/data/patina.db").exists();
    if in_project {
        println!("📂 Searching current project...");
        let project_options = ScryOptions {
            repo: None,
            all_repos: false,
            ..options.clone()
        };
        match scry_text(query, &project_options) {
            Ok(results) => {
                println!("   Found {} results", results.len());
                for r in results {
                    all_results.push(("[PROJECT]".to_string(), r));
                }
            }
            Err(e) => {
                eprintln!("   ⚠️  Project search failed: {}", e);
            }
        }
    }

    // 2. Query all registered reference repos
    let repos = crate::commands::repo::list()?;
    for repo in repos {
        println!("📚 Searching {}...", repo.name);
        let repo_options = ScryOptions {
            repo: Some(repo.name.clone()),
            all_repos: false,
            ..options.clone()
        };
        match scry_text(query, &repo_options) {
            Ok(results) => {
                println!("   Found {} results", results.len());
                for r in results {
                    all_results.push((format!("[{}]", repo.name.to_uppercase()), r));
                }
            }
            Err(e) => {
                eprintln!("   ⚠️  {} search failed: {}", repo.name, e);
            }
        }
    }

    // 3. Query persona if enabled
    if options.include_persona {
        println!("🧠 Searching persona...");
        if let Ok(persona_results) = persona::query(query, options.limit, options.min_score, None) {
            println!("   Found {} results", persona_results.len());
            for p in persona_results {
                all_results.push((
                    "[PERSONA]".to_string(),
                    ScryResult {
                        id: 0,
                        content: p.content,
                        score: p.score,
                        event_type: p.source.clone(),
                        source_id: p.domains.join(", "),
                        timestamp: p.timestamp,
                    },
                ));
            }
        }
    }

    // 4. Sort by score and take top limit
    all_results.sort_by(|a, b| {
        b.1.score
            .partial_cmp(&a.1.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_results.truncate(options.limit);

    println!();

    if all_results.is_empty() {
        println!("No results found across any repos.");
        return Ok(());
    }

    println!("Found {} results (combined):\n", all_results.len());
    println!("{}", "─".repeat(60));

    for (i, (source, result)) in all_results.iter().enumerate() {
        let timestamp_display = if result.timestamp.is_empty() {
            String::new()
        } else {
            format!(" | {}", result.timestamp)
        };
        println!(
            "\n[{}] {} Score: {:.3} | {} | {}{}",
            i + 1,
            source,
            result.score,
            result.event_type,
            result.source_id,
            timestamp_display
        );
        println!("    {}", truncate_content(&result.content, 200));
    }

    println!("\n{}", "─".repeat(60));

    Ok(())
}

/// Detect the best available dimension for vector search
/// Priority: semantic > dependency > temporal
/// Reference repos typically only have dependency
fn detect_best_dimension(embeddings_dir: &str) -> &'static str {
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

/// Truncate content for display
fn truncate_content(content: &str, max_len: usize) -> String {
    let content = content.replace('\n', " ").trim().to_string();
    if content.len() <= max_len {
        content
    } else {
        format!("{}...", &content[..max_len])
    }
}

/// Execute scry via mothership daemon
fn execute_via_mothership(query: Option<&str>, options: &ScryOptions) -> Result<()> {
    let address = mothership::get_address().unwrap_or_else(|| "unknown".to_string());
    println!("🔮 Scry - Querying mothership at {}\n", address);

    // File-based queries not supported via mothership yet
    if options.file.is_some() {
        anyhow::bail!("File-based queries (--file) not supported via mothership. Run locally.");
    }

    let query = query.ok_or_else(|| anyhow::anyhow!("Query text required"))?;
    println!("Query: \"{}\"\n", query);

    // Build request
    let request = mothership::ScryRequest {
        query: query.to_string(),
        dimension: options.dimension.clone(),
        repo: options.repo.clone(),
        all_repos: options.all_repos,
        include_issues: options.include_issues,
        include_persona: options.include_persona,
        limit: options.limit,
        min_score: options.min_score,
    };

    // Execute query
    let response = mothership::scry(request)?;

    if response.results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!("Found {} results:\n", response.count);
    println!("{}", "─".repeat(60));

    for (i, result) in response.results.iter().enumerate() {
        let timestamp_display = if result.timestamp.is_empty() {
            String::new()
        } else {
            format!(" | {}", result.timestamp)
        };
        println!(
            "\n[{}] Score: {:.3} | {} | {}{}",
            i + 1,
            result.score,
            result.event_type,
            result.source_id,
            timestamp_display
        );
        println!("    {}", truncate_content(&result.content, 200));
    }

    println!("\n{}", "─".repeat(60));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_content() {
        assert_eq!(truncate_content("short", 10), "short");
        assert_eq!(truncate_content("a very long string", 10), "a very lon...");
        assert_eq!(truncate_content("with\nnewlines", 20), "with newlines");
    }

    #[test]
    fn test_default_options() {
        let opts = ScryOptions::default();
        assert_eq!(opts.limit, 10);
        assert_eq!(opts.min_score, 0.0);
        assert!(opts.dimension.is_none());
        assert!(opts.include_persona); // Persona enabled by default
    }
}
