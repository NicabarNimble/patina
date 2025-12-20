//! MCP server - stdio transport

use anyhow::Result;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use super::protocol::{Request, Response};
use crate::commands::assay::{AssayOptions, QueryType};
use crate::retrieval::{FusedResult, QueryEngine, QueryOptions};

/// Run MCP server over stdio
pub fn run_mcp_server() -> Result<()> {
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
                        "required": ["query"]
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
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
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
                    "Missing required parameter: query",
                );
            }

            let options = QueryOptions {
                repo,
                all_repos,
                include_issues,
            };

            match engine.query_with_options(query, limit, &options) {
                Ok(results) => {
                    let text = format_results(&results);
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

fn format_results(results: &[FusedResult]) -> String {
    if results.is_empty() {
        return "No results found.".to_string();
    }

    let mut output = String::new();
    for (i, result) in results.iter().enumerate() {
        // Header: rank, sources, score
        output.push_str(&format!(
            "{}. [{}] (score: {:.3})",
            i + 1,
            result.sources.join("+"),
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
