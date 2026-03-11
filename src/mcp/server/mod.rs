//! MCP server - stdio transport

use anyhow::Result;
use rusqlite::Connection;
use serde::Deserialize;
use std::io::{BufRead, BufReader, Write};
use tracing::{info, warn};

use super::protocol::{Request, Response};
use crate::retrieval::QueryEngine;

mod assay;
mod scry;
mod session;
mod spec;
mod tools;

// JSON-RPC error codes — differentiated for actionable client guidance
const ERR_INVALID_PARAMS: i32 = -32602; // malformed request
const ERR_INTERNAL: i32 = -32603; // bugs, unexpected failures
const ERR_MISSING_INDEX: i32 = -32001; // "run `patina scrape` first"
const ERR_DATABASE: i32 = -32002; // connection, query, schema mismatch

/// Typed tool call parameters — deserialized once at the protocol boundary.
#[derive(Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: serde_json::Value,
}

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

    info!("checking secrets gate");

    // Check identity
    if status.identity_source.is_none() {
        warn!("no identity configured for secrets");
        anyhow::bail!(
            "\n❌ Cannot start MCP server.\n   Run: patina secrets add <name> to create vault and identity"
        );
    }
    info!(source = %status.identity_source.as_ref().unwrap(), "identity available");

    if has_global {
        info!(recipients = status.global.recipient_count, "global vault");
    }

    if has_project {
        let project = status.project.unwrap();
        info!(recipients = project.recipient_count, "project vault");
    }

    Ok(())
}

/// Run MCP server over stdio
pub fn run_mcp_server() -> Result<()> {
    // Initialize structured logging to file (before anything else)
    let log_dir = ".patina/local/logs";
    std::fs::create_dir_all(log_dir)?;
    let log_path = format!("{}/mcp-server.log", log_dir);
    let log_file = std::fs::File::create(&log_path)?;
    tracing_subscriber::fmt()
        .json()
        .with_writer(log_file)
        .with_target(false)
        .init();

    // Gate: validate secrets before starting
    check_secrets_gate()?;

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());

    // Initialize query engine
    let engine = QueryEngine::new();

    // Open patina.db once for the server lifetime (single-threaded, &Connection is safe)
    let db_path = ".patina/local/data/patina.db";
    std::fs::create_dir_all(".patina/local/data").ok();
    let patina_conn = Connection::open(db_path)?;
    info!(db = db_path, log = %log_path, "MCP server ready");

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

        let response = dispatch(&request, &engine, &patina_conn);
        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

fn dispatch(req: &Request, engine: &QueryEngine, conn: &Connection) -> Response {
    match req.method.as_str() {
        "initialize" => handle_initialize(req, engine),
        "initialized" => Response::success(req.id.clone(), serde_json::json!({})),
        "tools/list" => tools::handle_list_tools(req),
        "tools/call" => handle_tool_call(req, engine, conn),
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

fn handle_measure(req: &Request) -> Response {
    match crate::commands::measure::mcp_measure() {
        Ok(data) => Response::success(
            req.id.clone(),
            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&data).unwrap_or_default()
                }]
            }),
        ),
        Err(e) => Response::error(
            req.id.clone(),
            ERR_INTERNAL,
            &format!("measure failed: {}", e),
        ),
    }
}

fn handle_tool_call(req: &Request, engine: &QueryEngine, conn: &Connection) -> Response {
    let params: ToolCallParams = match serde_json::from_value(req.params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return Response::error(
                req.id.clone(),
                ERR_INVALID_PARAMS,
                &format!("Invalid tool call params: {}", e),
            )
        }
    };

    let ToolCallParams { name, arguments } = params;

    match name.as_str() {
        "scry" => {
            let args: scry::ScryArgs = match serde_json::from_value(arguments) {
                Ok(a) => a,
                Err(e) => {
                    return Response::error(
                        req.id.clone(),
                        ERR_INVALID_PARAMS,
                        &format!("Invalid scry params: {}", e),
                    )
                }
            };
            scry::handle_scry(req, args, engine, conn)
        }
        "context" => {
            let args: scry::ContextArgs = match serde_json::from_value(arguments) {
                Ok(a) => a,
                Err(e) => {
                    return Response::error(
                        req.id.clone(),
                        ERR_INVALID_PARAMS,
                        &format!("Invalid context params: {}", e),
                    )
                }
            };
            scry::handle_context(req, args)
        }
        "mother" => {
            let args: scry::MotherArgs = match serde_json::from_value(arguments) {
                Ok(a) => a,
                Err(e) => {
                    return Response::error(
                        req.id.clone(),
                        ERR_INVALID_PARAMS,
                        &format!("Invalid mother params: {}", e),
                    )
                }
            };
            scry::handle_mother(req, args)
        }
        "assay" => {
            let args: assay::AssayArgs = match serde_json::from_value(arguments) {
                Ok(a) => a,
                Err(e) => {
                    return Response::error(
                        req.id.clone(),
                        ERR_INVALID_PARAMS,
                        &format!("Invalid assay params: {}", e),
                    )
                }
            };
            assay::handle(req, args, conn)
        }
        "measure" => handle_measure(req),
        n if n.starts_with("session.") => {
            let tool_name = name.clone();
            let args: session::SessionArgs = match serde_json::from_value(arguments) {
                Ok(a) => a,
                Err(e) => {
                    return Response::error(
                        req.id.clone(),
                        ERR_INVALID_PARAMS,
                        &format!("Invalid session params: {}", e),
                    )
                }
            };
            session::handle(req, &tool_name, args)
        }
        n if n.starts_with("spec.") || n.starts_with("schemas.") => {
            let tool_name = name.clone();
            let args: spec::SpecArgs = match serde_json::from_value(arguments) {
                Ok(a) => a,
                Err(e) => {
                    return Response::error(
                        req.id.clone(),
                        ERR_INVALID_PARAMS,
                        &format!("Invalid {} params: {}", tool_name, e),
                    )
                }
            };
            spec::handle(req, n, args)
        }
        _ => Response::error(
            req.id.clone(),
            ERR_INVALID_PARAMS,
            &format!("Unknown tool: {}", name),
        ),
    }
}
