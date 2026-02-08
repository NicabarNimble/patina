//! Assay command - Query codebase structure and factual knowledge
//!
//! Assay is the factual layer: exact structural queries + ranked text search.
//! - Module inventory with line counts, function counts
//! - Import/importer relationships
//! - Caller/callee relationships from call graph
//! - Ranked FTS5 search across code, commits, and patterns

pub(crate) mod internal;

use anyhow::{Context, Result};
use internal::{
    collect_inventory_json, execute_callees, execute_callers, execute_derive,
    execute_derive_moments, execute_functions, execute_importers, execute_imports,
    execute_inventory, execute_search, SearchOptions,
};
use rusqlite::Connection;

// Re-export search types for MCP and external consumers
#[allow(unused_imports)]
pub use internal::search::{assay_search, assay_search_json, SearchResult};

const DB_PATH: &str = ".patina/local/data/patina.db";

/// Query type for assay command
#[derive(Debug, Clone, Default)]
pub enum QueryType {
    #[default]
    Inventory,
    Imports,
    Importers,
    Functions,
    Callers,
    Callees,
    Derive,
    DeriveMoments,
    /// Ranked factual search using FTS5
    Search { query: String },
}

/// Options for assay command
#[derive(Debug, Clone, Default)]
pub struct AssayOptions {
    pub query_type: QueryType,
    pub pattern: Option<String>,
    pub limit: usize,
    pub json: bool,
    /// Query a specific registered repo by name
    pub repo: Option<String>,
    /// Query all registered repos
    pub all_repos: bool,
    /// Include GitHub issues in search results
    pub include_issues: bool,
}

/// Execute assay command
pub fn execute(options: AssayOptions) -> Result<()> {
    // Handle search separately (doesn't need structural DB connection)
    if let QueryType::Search { ref query } = options.query_type {
        let search_opts = SearchOptions {
            limit: options.limit,
            include_issues: options.include_issues,
            repo: options.repo.clone(),
        };
        if options.json {
            let json = internal::search::assay_search_json(query, &search_opts)?;
            println!("{}", json);
            return Ok(());
        }
        return execute_search(query, &search_opts);
    }

    // Handle all_repos mode: iterate over all registered repos
    if options.all_repos {
        return execute_all_repos(&options);
    }

    // Resolve database path: specific repo or current directory
    let db_path = match &options.repo {
        Some(name) => crate::commands::repo::get_db_path(name)?,
        None => DB_PATH.to_string(),
    };

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Show repo context if specified
    if let Some(ref repo) = options.repo {
        println!("Repository: {}\n", repo);
    }

    match options.query_type {
        QueryType::Inventory => execute_inventory(&conn, &options, None),
        QueryType::Imports => execute_imports(&conn, &options),
        QueryType::Importers => execute_importers(&conn, &options),
        QueryType::Functions => execute_functions(&conn, &options),
        QueryType::Callers => execute_callers(&conn, &options),
        QueryType::Callees => execute_callees(&conn, &options),
        QueryType::Derive => execute_derive(&conn, &options),
        QueryType::DeriveMoments => execute_derive_moments(&conn, &options),
        QueryType::Search { .. } => unreachable!("handled above"),
    }
}

/// Execute assay across all registered repos
fn execute_all_repos(options: &AssayOptions) -> Result<()> {
    let repos = crate::commands::repo::list()?;

    if repos.is_empty() {
        println!("No registered repos. Use 'patina repo add <url>' to add repos.");
        return Ok(());
    }

    // Also query current project if it has a database
    let current_has_db = std::path::Path::new(DB_PATH).exists();

    if options.json {
        // JSON mode: collect all results into a single array
        let mut all_results: Vec<serde_json::Value> = Vec::new();

        if current_has_db {
            if let Ok(conn) = Connection::open(DB_PATH) {
                if let Ok(results) = collect_inventory_json(&conn, options, Some("(current)")) {
                    all_results.extend(results);
                }
            }
        }

        for repo in &repos {
            let db_path = std::path::Path::new(&repo.path).join(".patina/local/data/patina.db");
            if let Ok(conn) = Connection::open(&db_path) {
                if let Ok(results) = collect_inventory_json(&conn, options, Some(&repo.name)) {
                    all_results.extend(results);
                }
            }
        }

        println!("{}", serde_json::to_string_pretty(&all_results)?);
    } else {
        // Text mode: print each repo's results with headers
        if current_has_db {
            println!("━━━ (current) ━━━\n");
            if let Ok(conn) = Connection::open(DB_PATH) {
                let _ = execute_inventory(&conn, options, Some("(current)"));
            }
            println!();
        }

        for repo in &repos {
            println!("━━━ {} ━━━\n", repo.name);
            let db_path = std::path::Path::new(&repo.path).join(".patina/local/data/patina.db");
            if let Ok(conn) = Connection::open(&db_path) {
                let _ = execute_inventory(&conn, options, Some(&repo.name));
            } else {
                println!("  (database not found)\n");
            }
            println!();
        }
    }

    Ok(())
}
