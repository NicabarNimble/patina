//! MCP server - stdio transport

use anyhow::Result;
use std::io::{BufRead, BufReader, Write};

use super::protocol::{Request, Response};
use crate::retrieval::{FusedResult, QueryEngine};

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
                            }
                        },
                        "required": ["query"]
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

            if query.is_empty() {
                return Response::error(
                    req.id.clone(),
                    -32602,
                    "Missing required parameter: query",
                );
            }

            match engine.query(query, limit) {
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
