//! Assay (structural query) MCP handlers

use anyhow::Result;

use super::super::protocol::{Request, Response};
use crate::commands::assay::{AssayOptions, QueryType};

pub(super) fn handle(req: &Request, args: &serde_json::Value) -> Response {
    let query_type_str = args
        .get("query_type")
        .and_then(|v| v.as_str())
        .unwrap_or("inventory");
    let pattern = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .map(String::from);
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let repo = args.get("repo").and_then(|v| v.as_str()).map(String::from);
    let all_repos = args
        .get("all_repos")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let query = args.get("query").and_then(|v| v.as_str()).map(String::from);

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
                    -32602,
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
                    -32602,
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
                    -32602,
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
            -32602,
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
    let result = execute_assay(&options);

    // Emit usage event (best-effort)
    super::scry::emit_usage_event("assay.query", query_type_str, &serde_json::json!({
        "query_type": query_type_str,
        "pattern": &options.pattern,
        "duration_ms": start.elapsed().as_millis() as u64,
        "source": "mcp",
    }));

    match result {
        Ok(text) => Response::success(
            req.id.clone(),
            serde_json::json!({
                "content": [{ "type": "text", "text": text }]
            }),
        ),
        Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
    }
}

/// Execute assay query and return JSON result
fn execute_assay(options: &AssayOptions) -> Result<String> {
    use rusqlite::Connection;

    const DB_PATH: &str = ".patina/local/data/patina.db";

    // Handle all_repos mode
    if options.all_repos {
        return execute_assay_all_repos(options);
    }

    // Resolve database path: specific repo or current directory
    let db_path = match &options.repo {
        Some(name) => crate::commands::repo::get_db_path(name)?,
        None => DB_PATH.to_string(),
    };

    let conn = Connection::open(&db_path)?;

    match options.query_type {
        QueryType::Inventory => {
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

            let mut stmt = conn.prepare(sql)?;
            let modules: Vec<serde_json::Value> = stmt
                .query_map([pattern, &limit.to_string()], |row| {
                    Ok(serde_json::json!({
                        "path": row.get::<_, String>(0)?,
                        "lines": row.get::<_, i64>(1)?,
                        "bytes": row.get::<_, i64>(2)?,
                        "functions": row.get::<_, i64>(3)?,
                        "imports": row.get::<_, i64>(4)?
                    }))
                })?
                .filter_map(|r| r.ok())
                .collect();

            let total_lines: i64 = modules.iter().filter_map(|m| m["lines"].as_i64()).sum();
            let total_functions: i64 = modules.iter().filter_map(|m| m["functions"].as_i64()).sum();

            let result = serde_json::json!({
                "modules": modules,
                "summary": {
                    "total_files": modules.len(),
                    "total_lines": total_lines,
                    "total_functions": total_functions
                }
            });
            Ok(serde_json::to_string_pretty(&result)?)
        }
        QueryType::Imports => {
            let pattern = options.pattern.as_ref().unwrap();
            let limit = if options.limit > 0 {
                options.limit
            } else {
                100
            };

            let sql = r#"
                SELECT import_path, import_kind
                FROM import_facts
                WHERE file LIKE ?
                ORDER BY import_path
                LIMIT ?
            "#;

            let mut stmt = conn.prepare(sql)?;
            let imports: Vec<serde_json::Value> = stmt
                .query_map([format!("%{}%", pattern), limit.to_string()], |row| {
                    Ok(serde_json::json!({
                        "path": row.get::<_, String>(0)?,
                        "kind": row.get::<_, String>(1)?
                    }))
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(serde_json::to_string_pretty(&imports)?)
        }
        QueryType::Importers => {
            let pattern = options.pattern.as_ref().unwrap();
            let limit = if options.limit > 0 {
                options.limit
            } else {
                100
            };

            let sql = r#"
                SELECT file, imported_names
                FROM import_facts
                WHERE import_path LIKE ?
                ORDER BY file
                LIMIT ?
            "#;

            let mut stmt = conn.prepare(sql)?;
            let importers: Vec<serde_json::Value> = stmt
                .query_map([format!("%{}%", pattern), limit.to_string()], |row| {
                    Ok(serde_json::json!({
                        "file": row.get::<_, String>(0)?,
                        "names": row.get::<_, Option<String>>(1)?.unwrap_or_default()
                    }))
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(serde_json::to_string_pretty(&importers)?)
        }
        QueryType::Functions => {
            let limit = if options.limit > 0 {
                options.limit
            } else {
                100
            };

            let (sql, params): (&str, Vec<String>) = if let Some(pattern) = &options.pattern {
                (
                    r#"
                    SELECT name, file, is_public, is_async, parameters, return_type
                    FROM function_facts
                    WHERE name LIKE ? OR file LIKE ?
                    ORDER BY file, name
                    LIMIT ?
                    "#,
                    vec![
                        format!("%{}%", pattern),
                        format!("%{}%", pattern),
                        limit.to_string(),
                    ],
                )
            } else {
                (
                    r#"
                    SELECT name, file, is_public, is_async, parameters, return_type
                    FROM function_facts
                    ORDER BY file, name
                    LIMIT ?
                    "#,
                    vec![limit.to_string()],
                )
            };

            let mut stmt = conn.prepare(sql)?;
            let functions: Vec<serde_json::Value> = if options.pattern.is_some() {
                stmt.query_map([&params[0], &params[1], &params[2]], |row| {
                    Ok(serde_json::json!({
                        "name": row.get::<_, String>(0)?,
                        "file": row.get::<_, String>(1)?,
                        "is_public": row.get::<_, bool>(2)?,
                        "is_async": row.get::<_, bool>(3)?,
                        "parameters": row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                        "return_type": row.get::<_, Option<String>>(5)?
                    }))
                })?
                .filter_map(|r| r.ok())
                .collect()
            } else {
                stmt.query_map([&params[0]], |row| {
                    Ok(serde_json::json!({
                        "name": row.get::<_, String>(0)?,
                        "file": row.get::<_, String>(1)?,
                        "is_public": row.get::<_, bool>(2)?,
                        "is_async": row.get::<_, bool>(3)?,
                        "parameters": row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                        "return_type": row.get::<_, Option<String>>(5)?
                    }))
                })?
                .filter_map(|r| r.ok())
                .collect()
            };

            Ok(serde_json::to_string_pretty(&functions)?)
        }
        QueryType::Callers => {
            let pattern = options.pattern.as_ref().unwrap();
            let limit = if options.limit > 0 {
                options.limit
            } else {
                100
            };

            let sql = r#"
                SELECT caller, callee, file, call_type
                FROM call_graph
                WHERE callee LIKE ?
                ORDER BY file, caller
                LIMIT ?
            "#;

            let mut stmt = conn.prepare(sql)?;
            let callers: Vec<serde_json::Value> = stmt
                .query_map([format!("%{}%", pattern), limit.to_string()], |row| {
                    Ok(serde_json::json!({
                        "caller": row.get::<_, String>(0)?,
                        "callee": row.get::<_, String>(1)?,
                        "file": row.get::<_, String>(2)?,
                        "call_type": row.get::<_, String>(3)?
                    }))
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(serde_json::to_string_pretty(&callers)?)
        }
        QueryType::Callees => {
            let pattern = options.pattern.as_ref().unwrap();
            let limit = if options.limit > 0 {
                options.limit
            } else {
                100
            };

            let sql = r#"
                SELECT caller, callee, file, call_type
                FROM call_graph
                WHERE caller LIKE ?
                ORDER BY file, callee
                LIMIT ?
            "#;

            let mut stmt = conn.prepare(sql)?;
            let callees: Vec<serde_json::Value> = stmt
                .query_map([format!("%{}%", pattern), limit.to_string()], |row| {
                    Ok(serde_json::json!({
                        "caller": row.get::<_, String>(0)?,
                        "callee": row.get::<_, String>(1)?,
                        "file": row.get::<_, String>(2)?,
                        "call_type": row.get::<_, String>(3)?
                    }))
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(serde_json::to_string_pretty(&callees)?)
        }
        QueryType::Derive => {
            // Derive signals and return them
            // Ensure table exists
            conn.execute(
                "CREATE TABLE IF NOT EXISTS module_signals (
                    path TEXT PRIMARY KEY,
                    is_used INTEGER,
                    importer_count INTEGER,
                    activity_level TEXT,
                    last_commit_days INTEGER,
                    top_contributors TEXT,
                    centrality_score REAL,
                    staleness_flags TEXT,
                    computed_at TEXT
                )",
                [],
            )?;

            // Query existing signals (derive should be run via CLI first)
            let sql = r#"
                SELECT path, is_used, importer_count, activity_level,
                       last_commit_days, centrality_score, computed_at
                FROM module_signals
                ORDER BY importer_count DESC
                LIMIT 100
            "#;

            let mut stmt = conn.prepare(sql)?;
            let signals: Vec<serde_json::Value> = stmt
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "path": row.get::<_, String>(0)?,
                        "is_used": row.get::<_, i32>(1)? != 0,
                        "importer_count": row.get::<_, i64>(2)?,
                        "activity_level": row.get::<_, String>(3)?,
                        "last_commit_days": row.get::<_, Option<i64>>(4)?,
                        "centrality_score": row.get::<_, f64>(5)?,
                        "computed_at": row.get::<_, Option<String>>(6)?
                    }))
                })?
                .filter_map(|r| r.ok())
                .collect();

            let result = serde_json::json!({
                "signals": signals,
                "summary": {
                    "total_modules": signals.len(),
                    "used_modules": signals.iter().filter(|s| s["is_used"].as_bool().unwrap_or(false)).count()
                }
            });
            Ok(serde_json::to_string_pretty(&result)?)
        }
        QueryType::DeriveMoments => {
            // DeriveMoments not yet supported in MCP - use CLI instead
            Ok(serde_json::to_string_pretty(&serde_json::json!({
                "error": "derive-moments not yet supported in MCP, use 'patina assay derive-moments' CLI"
            }))?)
        }
        QueryType::Search { ref query } => {
            let search_opts = crate::commands::assay::internal::search::SearchOptions {
                limit: options.limit,
                include_issues: options.include_issues,
                repo: options.repo.clone(),
            };
            crate::commands::assay::internal::search::assay_search_json(query, &search_opts)
        }
        QueryType::Cochange { ref file } => {
            crate::commands::assay::internal::temporal::execute_cochange_json(
                file,
                options.limit,
                &db_path,
            )
        }
        QueryType::Belief { ref id } => {
            crate::commands::assay::internal::belief::execute_belief_grounding_json(
                id,
                options.limit,
            )
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

    // Query current project if it has a database
    if current_has_db {
        if let Ok(conn) = Connection::open(DB_PATH) {
            if let Ok(mut stmt) = conn.prepare(sql) {
                let modules: Vec<serde_json::Value> = stmt
                    .query_map([pattern, &limit.to_string()], |row| {
                        Ok(serde_json::json!({
                            "repo": "(current)",
                            "path": row.get::<_, String>(0)?,
                            "lines": row.get::<_, i64>(1)?,
                            "bytes": row.get::<_, i64>(2)?,
                            "functions": row.get::<_, i64>(3)?,
                            "imports": row.get::<_, i64>(4)?
                        }))
                    })
                    .ok()
                    .map(|iter| iter.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default();
                all_modules.extend(modules);
            }
        }
    }

    // Query each registered repo
    for repo in &repos {
        let db_path = Path::new(&repo.path).join(".patina/local/data/patina.db");
        if let Ok(conn) = Connection::open(&db_path) {
            if let Ok(mut stmt) = conn.prepare(sql) {
                let repo_name = repo.name.clone();
                let modules: Vec<serde_json::Value> = stmt
                    .query_map([pattern, &limit.to_string()], |row| {
                        Ok(serde_json::json!({
                            "repo": repo_name.clone(),
                            "path": row.get::<_, String>(0)?,
                            "lines": row.get::<_, i64>(1)?,
                            "bytes": row.get::<_, i64>(2)?,
                            "functions": row.get::<_, i64>(3)?,
                            "imports": row.get::<_, i64>(4)?
                        }))
                    })
                    .ok()
                    .map(|iter| iter.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default();
                all_modules.extend(modules);
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

    Ok(serde_json::to_string_pretty(&result)?)
}
