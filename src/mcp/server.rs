//! MCP server - stdio transport

use anyhow::Result;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use super::protocol::{Request, Response};
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
                    "name": "patina_query",
                    "description": "Search codebase knowledge using hybrid retrieval. Returns relevant code, patterns, decisions, and session history fused from semantic search, lexical search, and persona.",
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
                    "name": "patina_context",
                    "description": "Get project context, patterns, and architectural rules from the knowledge layer. Returns core patterns (eternal principles), surface patterns (active architecture), and project-specific conventions.",
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
        "patina_query" => {
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
        "patina_context" => {
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
        _ => Response::error(req.id.clone(), -32602, &format!("Unknown tool: {}", name)),
    }
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
