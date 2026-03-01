//! Assay (structural query) MCP handlers

use anyhow::Result;
use serde::Deserialize;

use super::super::protocol::{Request, Response};
use crate::commands::assay::internal;
use crate::commands::assay::{AssayOptions, QueryType};

#[derive(Deserialize)]
pub(super) struct AssayArgs {
    pub query_type: Option<String>,
    pub query: Option<String>,
    pub pattern: Option<String>,
    pub limit: Option<usize>,
    pub repo: Option<String>,
    #[serde(default)]
    pub all_repos: bool,
}

pub(super) fn handle(req: &Request, args: AssayArgs, conn: &rusqlite::Connection) -> Response {
    let query_type_str = args.query_type.as_deref().unwrap_or("inventory");
    let pattern = args.pattern;
    let limit = args.limit.unwrap_or(50);
    let repo = args.repo;
    let all_repos = args.all_repos;

    let query = args.query;

    let query_type = match query_type_str {
        "imports" => QueryType::Imports,
        "importers" => QueryType::Importers,
        "functions" => QueryType::Functions,
        "callers" => QueryType::Callers,
        "callees" => QueryType::Callees,
        "derive" => QueryType::Derive,
        "search" => {
            let q = query.or_else(|| pattern.clone()).unwrap_or_default();
            if q.is_empty() {
                return Response::error(
                    req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    "search query_type requires 'query' or 'pattern' parameter",
                );
            }
            QueryType::Search { query: q }
        }
        "cochange" => {
            let file = pattern.clone().unwrap_or_default();
            if file.is_empty() {
                return Response::error(
                    req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    "cochange query_type requires 'pattern' parameter (file path)",
                );
            }
            QueryType::Cochange { file }
        }
        "belief" => {
            let id = pattern.clone().unwrap_or_default();
            if id.is_empty() {
                return Response::error(
                    req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    "belief query_type requires 'pattern' parameter (belief ID)",
                );
            }
            QueryType::Belief { id }
        }
        _ => QueryType::Inventory,
    };

    // For pattern-required queries, validate pattern is provided
    if matches!(
        query_type,
        QueryType::Imports | QueryType::Importers | QueryType::Callers | QueryType::Callees
    ) && pattern.is_none()
    {
        return Response::error(
            req.id.clone(),
            super::ERR_INVALID_PARAMS,
            &format!(
                "The '{}' query type requires a 'pattern' parameter",
                query_type_str
            ),
        );
    }

    let options = AssayOptions {
        query_type,
        pattern,
        limit,
        json: true, // Always use JSON for MCP
        repo,
        all_repos,
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = execute_assay(&options, conn);

    // Emit usage event (best-effort)
    super::scry::emit_usage_event(
        "assay.query",
        query_type_str,
        &serde_json::json!({
            "query_type": query_type_str,
            "pattern": &options.pattern,
            "duration_ms": start.elapsed().as_millis() as u64,
            "source": "mcp",
        }),
    );

    match result {
        Ok(text) => Response::success(
            req.id.clone(),
            serde_json::json!({
                "content": [{ "type": "text", "text": text }]
            }),
        ),
        Err(e) => Response::error(req.id.clone(), super::ERR_DATABASE, &e.to_string()),
    }
}

/// Execute assay query and return JSON result
fn execute_assay(options: &AssayOptions, shared_conn: &rusqlite::Connection) -> Result<String> {
    use rusqlite::Connection;

    // Handle all_repos mode
    if options.all_repos {
        return execute_assay_all_repos(options);
    }

    // Use shared connection for default DB, open per-call for specific repos
    let specific_conn;
    let conn = match &options.repo {
        Some(name) => {
            let db_path = crate::commands::repo::get_db_path(name)?;
            specific_conn = Connection::open(&db_path)?;
            &specific_conn
        }
        None => shared_conn,
    };

    let limit = if options.limit > 0 { options.limit } else { 100 };

    match options.query_type {
        QueryType::Inventory => {
            let pattern = options.pattern.as_deref().unwrap_or("%");
            let inv_limit = if options.limit > 0 { options.limit } else { 1000 };
            internal::inventory_json(conn, pattern, inv_limit)
        }
        QueryType::Imports => {
            let pattern = options.pattern.as_ref().unwrap();
            internal::imports_json(conn, pattern, limit)
        }
        QueryType::Importers => {
            let pattern = options.pattern.as_ref().unwrap();
            internal::importers_json(conn, pattern, limit)
        }
        QueryType::Functions => {
            internal::functions_json(conn, options.pattern.as_deref(), limit)
        }
        QueryType::Callers => {
            let pattern = options.pattern.as_ref().unwrap();
            internal::callers_json(conn, pattern, limit)
        }
        QueryType::Callees => {
            let pattern = options.pattern.as_ref().unwrap();
            internal::callees_json(conn, pattern, limit)
        }
        QueryType::Derive => {
            internal::derive_signals_json(conn)
        }
        QueryType::DeriveMoments => {
            Ok(serde_json::to_string_pretty(&serde_json::json!({
                "error": "derive-moments not yet supported in MCP, use 'patina assay derive-moments' CLI"
            }))?)
        }
        QueryType::Search { ref query } => {
            let search_opts = internal::search::SearchOptions {
                limit: options.limit,
                include_issues: options.include_issues,
                repo: options.repo.clone(),
            };
            internal::search::assay_search_json(query, &search_opts)
        }
        QueryType::Cochange { ref file } => {
            let cochange_db = match &options.repo {
                Some(name) => crate::commands::repo::get_db_path(name)?,
                None => ".patina/local/data/patina.db".to_string(),
            };
            internal::temporal::execute_cochange_json(file, options.limit, &cochange_db)
        }
        QueryType::Belief { ref id } => {
            internal::belief::execute_belief_grounding_json(id, options.limit)
        }
    }
}

/// Execute assay across all registered repos (MCP version)
fn execute_assay_all_repos(options: &AssayOptions) -> Result<String> {
    use rusqlite::Connection;
    use std::path::Path;

    const DB_PATH: &str = ".patina/local/data/patina.db";

    let repos = crate::commands::repo::list()?;
    let current_has_db = Path::new(DB_PATH).exists();

    // For now, only inventory query type supports all_repos in MCP
    // Other query types would need more complex aggregation
    if !matches!(options.query_type, QueryType::Inventory) {
        anyhow::bail!("all_repos mode currently only supports 'inventory' query type");
    }

    let pattern = options.pattern.as_deref().unwrap_or("%");
    let limit = if options.limit > 0 {
        options.limit
    } else {
        1000
    };

    let sql = r#"
        SELECT
            i.path,
            COALESCE(i.line_count, 0) as lines,
            i.size as bytes,
            COALESCE((SELECT COUNT(*) FROM function_facts WHERE file = i.path), 0) as functions,
            COALESCE((SELECT COUNT(*) FROM import_facts WHERE file = i.path), 0) as imports
        FROM index_state i
        WHERE i.path LIKE ?
        ORDER BY lines DESC
        LIMIT ?
    "#;

    let mut all_modules: Vec<serde_json::Value> = Vec::new();
    let mut total_failures = 0usize;

    // Query current project if it has a database
    if current_has_db {
        match Connection::open(DB_PATH) {
            Ok(conn) => {
                if let Ok(mut stmt) = conn.prepare(sql) {
                    if let Ok(rows) = stmt.query_map([pattern, &limit.to_string()], |row| {
                        Ok(serde_json::json!({
                            "repo": "(current)",
                            "path": row.get::<_, String>(0)?,
                            "lines": row.get::<_, i64>(1)?,
                            "bytes": row.get::<_, i64>(2)?,
                            "functions": row.get::<_, i64>(3)?,
                            "imports": row.get::<_, i64>(4)?
                        }))
                    }) {
                        let (modules, failures) = internal::collect_rows(rows);
                        total_failures += failures;
                        all_modules.extend(modules);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(db = DB_PATH, error = %e, "failed to open current project DB");
            }
        }
    }

    // Query each registered repo
    for repo in &repos {
        let db_path = Path::new(&repo.path).join(".patina/local/data/patina.db");
        match Connection::open(&db_path) {
            Ok(conn) => {
                if let Ok(mut stmt) = conn.prepare(sql) {
                    let repo_name = repo.name.clone();
                    if let Ok(rows) = stmt.query_map([pattern, &limit.to_string()], |row| {
                        Ok(serde_json::json!({
                            "repo": repo_name.clone(),
                            "path": row.get::<_, String>(0)?,
                            "lines": row.get::<_, i64>(1)?,
                            "bytes": row.get::<_, i64>(2)?,
                            "functions": row.get::<_, i64>(3)?,
                            "imports": row.get::<_, i64>(4)?
                        }))
                    }) {
                        let (modules, failures) = internal::collect_rows(rows);
                        total_failures += failures;
                        all_modules.extend(modules);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(db = %db_path.display(), repo = %repo.name, error = %e, "failed to open repo DB");
            }
        }
    }

    let total_lines: i64 = all_modules.iter().filter_map(|m| m["lines"].as_i64()).sum();
    let total_functions: i64 = all_modules
        .iter()
        .filter_map(|m| m["functions"].as_i64())
        .sum();

    let result = serde_json::json!({
        "modules": all_modules,
        "summary": {
            "total_files": all_modules.len(),
            "total_lines": total_lines,
            "total_functions": total_functions,
            "repos_queried": repos.len() + if current_has_db { 1 } else { 0 }
        }
    });

    internal::serialize_result(result, total_failures)
}
