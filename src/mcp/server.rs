//! MCP server - stdio transport

use anyhow::Result;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use super::protocol::{Request, Response};
use crate::commands::assay::{AssayOptions, QueryType};
use crate::retrieval::{FusedResult, QueryEngine, QueryOptions};

/// Check project secrets compliance before starting MCP server.
///
/// For v2 (age-encrypted vaults), validates:
/// 1. Identity is available (PATINA_IDENTITY env or Keychain)
/// 2. Global or project vault exists
///
/// Returns Ok(()) if compliant (or no vaults configured).
fn check_secrets_gate() -> Result<()> {
    use patina::secrets;

    let project_root = std::env::current_dir().ok();
    let status = secrets::check_status(project_root.as_deref())?;

    // Check if any vault exists
    let has_global = status.global.exists;
    let has_project = status.project.as_ref().map(|p| p.exists).unwrap_or(false);

    // No vaults - pass the gate (no secrets configured)
    if !has_global && !has_project {
        return Ok(());
    }

    eprintln!("Checking secrets...");

    // Check identity
    if status.identity_source.is_none() {
        eprintln!("  ✗ No identity configured");
        anyhow::bail!(
            "\n❌ Cannot start MCP server.\n   Run: patina secrets add <name> to create vault and identity"
        );
    }
    eprintln!("  ✓ Identity via {}", status.identity_source.unwrap());

    if has_global {
        eprintln!(
            "  ✓ Global vault ({} recipients)",
            status.global.recipient_count
        );
    }

    if has_project {
        let project = status.project.unwrap();
        eprintln!("  ✓ Project vault ({} recipients)", project.recipient_count);
    }

    Ok(())
}

/// Run MCP server over stdio
pub fn run_mcp_server() -> Result<()> {
    // Gate: validate secrets before starting
    check_secrets_gate()?;

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());

    // Initialize query engine
    let engine = QueryEngine::new();

    eprintln!("patina: MCP server ready");

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        let request: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = Response::error(None, -32700, &format!("Parse error: {}", e));
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            let resp = Response::error(
                request.id.clone(),
                -32600,
                &format!(
                    "Invalid JSON-RPC version: expected 2.0, got {}",
                    request.jsonrpc
                ),
            );
            writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            stdout.flush()?;
            continue;
        }

        let response = dispatch(&request, &engine);
        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

fn dispatch(req: &Request, engine: &QueryEngine) -> Response {
    match req.method.as_str() {
        "initialize" => handle_initialize(req, engine),
        "initialized" => Response::success(req.id.clone(), serde_json::json!({})),
        "tools/list" => handle_list_tools(req),
        "tools/call" => handle_tool_call(req, engine),
        _ => Response::error(req.id.clone(), -32601, "Method not found"),
    }
}

fn handle_initialize(req: &Request, engine: &QueryEngine) -> Response {
    Response::success(
        req.id.clone(),
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "patina",
                "version": env!("CARGO_PKG_VERSION"),
                "oracles": engine.available_oracles()
            }
        }),
    )
}

fn handle_list_tools(req: &Request) -> Response {
    Response::success(
        req.id.clone(),
        serde_json::json!({
            "tools": [
                {
                    "name": "scry",
                    "description": "Search codebase knowledge - USE THIS FIRST for any question about the code. Fast hybrid search over indexed symbols, functions, types, git history, and session learnings. Prefer this over manual file exploration.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Natural language question or code search query"
                            },
                            "mode": {
                                "type": "string",
                                "enum": ["find", "orient", "recent", "why", "use"],
                                "default": "find",
                                "description": "Query mode: 'find' (default), 'orient' (structural ranking), 'recent' (temporal ranking), 'why' (explain result), 'use' (log result usage)"
                            },
                            "query_id": {
                                "type": "string",
                                "description": "Query ID for use mode (from previous scry response)"
                            },
                            "rank": {
                                "type": "integer",
                                "description": "Result rank for use mode (1-based)"
                            },
                            "path": {
                                "type": "string",
                                "description": "Directory path for orient mode (e.g., 'src/retrieval/')"
                            },
                            "days": {
                                "type": "integer",
                                "default": 7,
                                "description": "Days to look back for recent mode (default: 7)"
                            },
                            "doc_id": {
                                "type": "string",
                                "description": "Document ID for why mode (e.g., 'src/retrieval/engine.rs')"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Maximum results to return (default: 10)",
                                "default": 10
                            },
                            "repo": {
                                "type": "string",
                                "description": "Query a specific registered repo by name (from registry)"
                            },
                            "all_repos": {
                                "type": "boolean",
                                "description": "Query all registered repos (default: false)",
                                "default": false
                            },
                            "include_issues": {
                                "type": "boolean",
                                "description": "Include GitHub issues in results (default: false)",
                                "default": false
                            }
                        },
                        "required": []
                    }
                },
                {
                    "name": "context",
                    "description": "Get project patterns and conventions - USE THIS to understand design rules before making architectural changes. Returns core patterns (eternal principles) and surface patterns (active architecture).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "topic": {
                                "type": "string",
                                "description": "Optional topic to focus on (e.g., 'error handling', 'testing', 'architecture')"
                            },
                            "repo": {
                                "type": "string",
                                "description": "Query a specific repo by name (from registry)"
                            },
                            "all_repos": {
                                "type": "boolean",
                                "description": "Query all registered repos (default: false)",
                                "default": false
                            }
                        }
                    }
                },
                {
                    "name": "assay",
                    "description": "Query codebase structure - modules, imports, functions, call graph. Use for exact structural questions like 'list all modules', 'what imports X', 'show largest files'. For semantic similarity, use scry instead. Use 'derive' to compute/view structural signals (usage, activity, centrality).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query_type": {
                                "type": "string",
                                "enum": ["inventory", "imports", "importers", "functions", "callers", "callees", "derive"],
                                "default": "inventory",
                                "description": "Type of structural query"
                            },
                            "pattern": {
                                "type": "string",
                                "description": "Path pattern or function name to filter results"
                            },
                            "limit": {
                                "type": "integer",
                                "default": 50,
                                "description": "Maximum results to return"
                            },
                            "repo": {
                                "type": "string",
                                "description": "Query a specific registered repo by name (from registry)"
                            },
                            "all_repos": {
                                "type": "boolean",
                                "default": false,
                                "description": "Query all registered repos (default: false)"
                            }
                        }
                    }
                }
            ]
        }),
    )
}

fn handle_tool_call(req: &Request, engine: &QueryEngine) -> Response {
    let name = req
        .params
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let args = req.params.get("arguments").cloned().unwrap_or_default();

    match name {
        "scry" => {
            let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("find");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

            // Handle modes
            match mode {
                "orient" => {
                    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                    if path.is_empty() {
                        return Response::error(
                            req.id.clone(),
                            -32602,
                            "orient mode requires 'path' parameter",
                        );
                    }

                    match handle_orient(path, limit) {
                        Ok(text) => Response::success(
                            req.id.clone(),
                            serde_json::json!({
                                "content": [{ "type": "text", "text": text }]
                            }),
                        ),
                        Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
                    }
                }
                "recent" => {
                    let query = args.get("query").and_then(|v| v.as_str());
                    let days = args.get("days").and_then(|v| v.as_u64()).unwrap_or(7) as u32;

                    match handle_recent(query, days, limit) {
                        Ok(text) => Response::success(
                            req.id.clone(),
                            serde_json::json!({
                                "content": [{ "type": "text", "text": text }]
                            }),
                        ),
                        Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
                    }
                }
                "why" => {
                    let doc_id = args.get("doc_id").and_then(|v| v.as_str()).unwrap_or("");
                    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

                    if doc_id.is_empty() || query.is_empty() {
                        return Response::error(
                            req.id.clone(),
                            -32602,
                            "why mode requires 'doc_id' and 'query' parameters",
                        );
                    }

                    match handle_why(doc_id, query, engine) {
                        Ok(text) => Response::success(
                            req.id.clone(),
                            serde_json::json!({
                                "content": [{ "type": "text", "text": text }]
                            }),
                        ),
                        Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
                    }
                }
                "use" => {
                    // Phase 3: Log result usage from agent
                    let query_id = args.get("query_id").and_then(|v| v.as_str()).unwrap_or("");
                    let rank = args.get("rank").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                    if query_id.is_empty() || rank == 0 {
                        return Response::error(
                            req.id.clone(),
                            -32602,
                            "use mode requires 'query_id' and 'rank' parameters",
                        );
                    }

                    match handle_use(query_id, rank) {
                        Ok(text) => Response::success(
                            req.id.clone(),
                            serde_json::json!({
                                "content": [{ "type": "text", "text": text }]
                            }),
                        ),
                        Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
                    }
                }
                _ => {
                    // Default find mode
                    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                    let repo = args.get("repo").and_then(|v| v.as_str()).map(String::from);
                    let all_repos = args
                        .get("all_repos")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let include_issues = args
                        .get("include_issues")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if query.is_empty() {
                        return Response::error(
                            req.id.clone(),
                            -32602,
                            "find mode requires 'query' parameter",
                        );
                    }

                    let options = QueryOptions {
                        repo,
                        all_repos,
                        include_issues,
                    };

                    match engine.query_with_options(query, limit, &options) {
                        Ok(results) => {
                            // Log query and get query_id for feedback loop (Phase 3)
                            let query_id = log_mcp_query(query, "find", &results);
                            let text = format_results_with_query_id(&results, query_id.as_deref());
                            Response::success(
                                req.id.clone(),
                                serde_json::json!({
                                    "content": [{ "type": "text", "text": text }]
                                }),
                            )
                        }
                        Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
                    }
                }
            }
        }
        "context" => {
            let topic = args.get("topic").and_then(|v| v.as_str());
            match get_project_context(topic) {
                Ok(text) => Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                ),
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        "assay" => {
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

            let query_type = match query_type_str {
                "imports" => QueryType::Imports,
                "importers" => QueryType::Importers,
                "functions" => QueryType::Functions,
                "callers" => QueryType::Callers,
                "callees" => QueryType::Callees,
                "derive" => QueryType::Derive,
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
            };

            match execute_assay(&options) {
                Ok(text) => Response::success(
                    req.id.clone(),
                    serde_json::json!({
                        "content": [{ "type": "text", "text": text }]
                    }),
                ),
                Err(e) => Response::error(req.id.clone(), -32603, &e.to_string()),
            }
        }
        _ => Response::error(req.id.clone(), -32602, &format!("Unknown tool: {}", name)),
    }
}

/// Execute assay query and return JSON result
fn execute_assay(options: &AssayOptions) -> Result<String> {
    use rusqlite::Connection;

    const DB_PATH: &str = ".patina/data/patina.db";

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
    }
}

/// Execute assay across all registered repos (MCP version)
fn execute_assay_all_repos(options: &AssayOptions) -> Result<String> {
    use rusqlite::Connection;
    use std::path::Path;

    const DB_PATH: &str = ".patina/data/patina.db";

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
        let db_path = Path::new(&repo.path).join(".patina/data/patina.db");
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

/// Log an MCP query to the eventlog and return query_id (Phase 3)
fn log_mcp_query(query: &str, mode: &str, results: &[FusedResult]) -> Option<String> {
    use rusqlite::Connection;

    const DB_PATH: &str = ".patina/data/patina.db";

    // Get session_id from active session
    let session_id = std::fs::read_to_string(".claude/context/active-session.md")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("**ID**:"))
                .map(|l| l.replace("**ID**:", "").trim().to_string())
        })?;

    // Generate query_id
    let now = chrono::Utc::now();
    let random_suffix: String = (0..3)
        .map(|_| (b'a' + fastrand::u8(0..26)) as char)
        .collect();
    let query_id = format!("q_{}_{}", now.format("%Y%m%d_%H%M%S"), random_suffix);

    // Build results array for logging
    let results_json: Vec<serde_json::Value> = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            serde_json::json!({
                "doc_id": r.doc_id,
                "score": r.fused_score,
                "rank": i + 1,
                "event_type": r.metadata.event_type
            })
        })
        .collect();

    let query_data = serde_json::json!({
        "query": query,
        "query_id": query_id,
        "mode": mode,
        "session_id": session_id,
        "results": results_json
    });

    // Best-effort insert into eventlog
    let conn = Connection::open(DB_PATH).ok()?;
    let timestamp = now.to_rfc3339();
    conn.execute(
        "INSERT INTO eventlog (event_type, timestamp, source_id, data) VALUES (?, ?, ?, ?)",
        rusqlite::params!["scry.query", timestamp, query_id, query_data.to_string()],
    )
    .ok()?;

    Some(query_id)
}

/// Format results with query_id for feedback (Phase 3)
fn format_results_with_query_id(results: &[FusedResult], query_id: Option<&str>) -> String {
    let mut output = format_results(results);
    if let Some(qid) = query_id {
        output.push_str(&format!(
            "\n---\nQuery ID: {} (use with scry mode='use' to log usage)\n",
            qid
        ));
    }
    output
}

fn format_results(results: &[FusedResult]) -> String {
    if results.is_empty() {
        return "No results found.".to_string();
    }

    let mut output = String::new();
    for (i, result) in results.iter().enumerate() {
        // Header: rank, sources with ranks, fused score
        let mut contributions_str: String = result
            .contributions
            .iter()
            .map(|(name, c)| {
                let score_display = match c.score_type {
                    "co_change_count" => format!("co-changes: {}", c.raw_score as i32),
                    "bm25" => format!("{:.1} BM25", c.raw_score),
                    _ => format!("{:.2}", c.raw_score),
                };
                format!("{} #{} ({})", name, c.rank, score_display)
            })
            .collect::<Vec<_>>()
            .join(" | ");

        // Add structural annotations if available
        let ann = &result.annotations;
        if let Some(count) = ann.importer_count {
            if count > 0 {
                contributions_str.push_str(&format!(" | imp {}", count));
            }
        }
        if let Some(true) = ann.is_entry_point {
            contributions_str.push_str(" | entry");
        }

        let source_tag = if result.sources.contains(&"persona") {
            "[PERSONA] "
        } else {
            ""
        };
        output.push_str(&format!(
            "{}. {}[{}] (score: {:.3})",
            i + 1,
            source_tag,
            contributions_str,
            result.fused_score
        ));

        // Location: file path or doc_id for non-file results
        if let Some(ref path) = result.metadata.file_path {
            output.push_str(&format!(" {}", path));
        } else {
            // For persona/session results without file path, show doc_id
            output.push_str(&format!(" {}", result.doc_id));
        }

        // Event type (e.g., "code_chunk", "session", "observation")
        if let Some(ref event_type) = result.metadata.event_type {
            output.push_str(&format!(" ({})", event_type));
        }

        // Timestamp if available
        if let Some(ref ts) = result.metadata.timestamp {
            if !ts.is_empty() {
                output.push_str(&format!(" @{}", ts));
            }
        }
        output.push('\n');

        // Content
        output.push_str(&result.content);
        output.push_str("\n\n");
    }
    output
}

/// Handle orient mode - rank files in a directory by structural importance
fn handle_orient(dir_path: &str, limit: usize) -> Result<String> {
    use anyhow::Context;
    use rusqlite::Connection;

    let db_path = ".patina/data/patina.db";
    let conn = Connection::open(db_path)
        .with_context(|| "Failed to open database. Run 'patina scrape' first.")?;

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

    // Normalize path (ensure ./ prefix)
    let normalized_path = dir_path.trim_end_matches('/');
    let normalized_path = if normalized_path.starts_with("./") {
        normalized_path.to_string()
    } else {
        format!("./{}", normalized_path)
    };

    // Query files ranked by structural composite score
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
    let results: Vec<(String, f64, i64, String, bool, bool, i64)> = stmt
        .query_map(rusqlite::params![pattern, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)? != 0,
                row.get::<_, i64>(5)? != 0,
                row.get::<_, i64>(6)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if results.is_empty() {
        return Ok(format!(
            "No files found in '{}' with structural signals.\n\nRun 'patina assay derive' to compute signals.",
            dir_path
        ));
    }

    let mut output = format!("# Orient: {} ({} files)\n\n", dir_path, results.len());

    for (i, (path, score, importers, activity, is_entry, is_test, commits)) in
        results.iter().enumerate()
    {
        let mut flags = Vec::new();
        if *is_entry {
            flags.push("entry_point");
        }
        if *is_test {
            flags.push("test");
        }
        let flags_str = if flags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", flags.join(", "))
        };

        output.push_str(&format!(
            "{}. **{}** (score: {:.0})\n   {} importers | {} activity | {} commits{}\n\n",
            i + 1,
            path,
            score,
            importers,
            activity,
            commits,
            flags_str
        ));
    }

    Ok(output)
}

/// Handle recent mode - show recently changed files
fn handle_recent(query: Option<&str>, days: u32, limit: usize) -> Result<String> {
    use anyhow::Context;
    use rusqlite::Connection;

    let db_path = ".patina/data/patina.db";
    let conn = Connection::open(db_path)
        .with_context(|| "Failed to open database. Run 'patina scrape' first.")?;

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

    if results.is_empty() {
        return Ok(format!("No changes found in the last {} days.", days));
    }

    // Deduplicate
    let mut seen = std::collections::HashSet::new();
    let unique: Vec<_> = results
        .into_iter()
        .filter(|(path, _, _, _)| seen.insert(path.clone()))
        .take(limit)
        .collect();

    let mut output = format!(
        "# Recent Changes{} ({} days)\n\n",
        query
            .map(|q| format!(" matching '{}'", q))
            .unwrap_or_default(),
        days
    );

    for (i, (path, timestamp, message, author)) in unique.iter().enumerate() {
        let date = timestamp.split('T').next().unwrap_or(timestamp);
        let short_msg: String = message.chars().take(50).collect();
        output.push_str(&format!(
            "{}. **{}** ({})\n   {} - {}\n\n",
            i + 1,
            path,
            date,
            author,
            if message.len() > 50 {
                format!("{}...", short_msg)
            } else {
                short_msg
            }
        ));
    }

    Ok(output)
}

/// Handle why mode - explain a specific result
fn handle_why(doc_id: &str, query: &str, engine: &QueryEngine) -> Result<String> {
    let options = QueryOptions::default();
    let results = engine.query_with_options(query, 50, &options)?;

    let matching = results
        .iter()
        .find(|r| r.doc_id == doc_id || r.doc_id.ends_with(doc_id) || doc_id.ends_with(&r.doc_id));

    match matching {
        Some(result) => {
            let rank = results
                .iter()
                .position(|r| r.doc_id == result.doc_id)
                .unwrap_or(0)
                + 1;

            let mut output = format!(
                "# Why: {}\n\nQuery: \"{}\"\nRank: #{}\nFused Score: {:.4}\n\n## Oracle Contributions\n\n",
                result.doc_id, query, rank, result.fused_score
            );

            for (oracle_name, contrib) in &result.contributions {
                let score_display = match contrib.score_type {
                    "co_change_count" => format!("{} co-changes", contrib.raw_score as i32),
                    "bm25" => format!("{:.2} BM25", contrib.raw_score),
                    "cosine" => format!("{:.3} cosine", contrib.raw_score),
                    _ => format!("{:.3} {}", contrib.raw_score, contrib.score_type),
                };

                output.push_str(&format!(
                    "- **{}**: rank #{} ({})\n",
                    oracle_name, contrib.rank, score_display
                ));
            }

            let ann = &result.annotations;
            if ann.importer_count.is_some() || ann.activity_level.is_some() {
                output.push_str("\n## Structural Signals\n\n");
                if let Some(count) = ann.importer_count {
                    output.push_str(&format!("- Importers: {}\n", count));
                }
                if let Some(ref level) = ann.activity_level {
                    output.push_str(&format!("- Activity: {}\n", level));
                }
            }

            Ok(output)
        }
        None => {
            let mut output = format!(
                "'{}' not found in top 50 results for query \"{}\".\n\nTop 5 results:\n",
                doc_id, query
            );
            for (i, r) in results.iter().take(5).enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, r.doc_id));
            }
            Ok(output)
        }
    }
}

/// Get project context from the knowledge layer
///
/// Reads patterns from layer/core/ (eternal principles) and layer/surface/ (active patterns)
/// Optionally filters by topic if provided
fn get_project_context(topic: Option<&str>) -> Result<String> {
    let mut output = String::new();

    // Check if we're in a patina project
    let layer_path = Path::new("layer");
    if !layer_path.exists() {
        return Ok(
            "No knowledge layer found. Run 'patina init' to initialize a project.".to_string(),
        );
    }

    // Read core patterns (eternal principles)
    let core_path = layer_path.join("core");
    let core_patterns = read_patterns(&core_path, topic)?;

    // Read surface patterns (active architecture)
    let surface_path = layer_path.join("surface");
    let surface_patterns = read_patterns(&surface_path, topic)?;

    // Format output
    if !core_patterns.is_empty() {
        output.push_str("# Core Patterns (Eternal Principles)\n\n");
        for (name, content) in &core_patterns {
            output.push_str(&format!("## {}\n\n{}\n\n", name, content));
        }
    }

    if !surface_patterns.is_empty() {
        output.push_str("# Surface Patterns (Active Architecture)\n\n");
        for (name, content) in &surface_patterns {
            output.push_str(&format!("## {}\n\n{}\n\n", name, content));
        }
    }

    if output.is_empty() {
        if let Some(t) = topic {
            output = format!("No patterns found matching topic: '{}'", t);
        } else {
            output = "No patterns found in the knowledge layer.".to_string();
        }
    }

    Ok(output)
}

/// Read markdown patterns from a directory
fn read_patterns(dir: &Path, topic: Option<&str>) -> Result<Vec<(String, String)>> {
    let mut patterns = Vec::new();

    if !dir.exists() {
        return Ok(patterns);
    }

    // Read .md files in the directory
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only process markdown files
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Skip certain files
            if name == "README" || name.starts_with('.') {
                continue;
            }

            let content = fs::read_to_string(&path)?;

            // If topic filter provided, check if content matches
            if let Some(t) = topic {
                let topic_lower = t.to_lowercase();
                let content_lower = content.to_lowercase();
                let name_lower = name.to_lowercase();

                if !content_lower.contains(&topic_lower) && !name_lower.contains(&topic_lower) {
                    continue;
                }
            }

            // Extract summary (first non-frontmatter paragraph)
            let summary = extract_summary(&content);
            patterns.push((name, summary));
        }
    }

    // Sort by name for consistent output
    patterns.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(patterns)
}

/// Handle use mode - log result usage from agent (Phase 3 feedback)
fn handle_use(query_id: &str, rank: usize) -> Result<String> {
    use rusqlite::Connection;

    const DB_PATH: &str = ".patina/data/patina.db";

    let conn = Connection::open(DB_PATH)?;

    // Get the query results to find the doc_id for this rank
    let data: String = conn.query_row(
        "SELECT data FROM eventlog WHERE event_type = 'scry.query' AND source_id = ?",
        [query_id],
        |row| row.get(0),
    )?;

    let parsed: serde_json::Value = serde_json::from_str(&data)?;
    let results = parsed["results"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No results in query"))?;

    if rank == 0 || rank > results.len() {
        anyhow::bail!(
            "Invalid rank {}. Query had {} results.",
            rank,
            results.len()
        );
    }

    let doc_id = results[rank - 1]["doc_id"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // Get session_id from active session
    let session_id = std::fs::read_to_string(".claude/context/active-session.md")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("**ID**:"))
                .map(|l| l.replace("**ID**:", "").trim().to_string())
        });

    // Log the usage event
    let use_data = serde_json::json!({
        "query_id": query_id,
        "result_used": doc_id,
        "rank": rank,
        "session_id": session_id
    });

    let timestamp = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO eventlog (event_type, timestamp, source_id, data) VALUES (?, ?, ?, ?)",
        rusqlite::params!["scry.use", timestamp, query_id, use_data.to_string()],
    )?;

    Ok(format!(
        "Usage logged: {} rank #{} ({})",
        query_id, rank, doc_id
    ))
}

/// Extract a summary from markdown content (skip frontmatter, get first paragraphs)
fn extract_summary(content: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();

    // Skip YAML frontmatter if present
    if lines.first().map(|l| *l == "---").unwrap_or(false) {
        if let Some(end) = lines.iter().skip(1).position(|l| *l == "---") {
            lines = lines[end + 2..].to_vec();
        }
    }

    // Skip title line (# ...)
    if lines.first().map(|l| l.starts_with('#')).unwrap_or(false) {
        lines = lines[1..].to_vec();
    }

    // Get first ~500 chars of meaningful content
    let mut summary = String::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !summary.is_empty() {
                summary.push('\n');
            }
            continue;
        }
        summary.push_str(trimmed);
        summary.push(' ');

        if summary.len() > 500 {
            // Truncate at char boundary
            let truncated: String = summary.chars().take(500).collect();
            summary = truncated;
            summary.push_str("...");
            break;
        }
    }

    summary.trim().to_string()
}
