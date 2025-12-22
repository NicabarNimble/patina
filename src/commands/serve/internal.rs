//! Internal implementation of the Mothership server

use anyhow::Result;
use rouille::{router, Request, Response};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

use super::ServeOptions;
use crate::commands::persona;
use crate::commands::scry::{self, ScryOptions, ScryResult};
use crate::retrieval::{QueryEngine, QueryOptions};

/// Server state shared across request handlers
pub struct ServerState {
    start_time: Instant,
    version: String,
}

impl ServerState {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_secs: u64,
}

/// Scry API request
#[derive(Deserialize)]
struct ScryRequest {
    /// Query text
    query: String,
    /// Optional dimension (semantic, temporal, dependency)
    dimension: Option<String>,
    /// Optional repo name
    repo: Option<String>,
    /// Query all repos
    #[serde(default)]
    all_repos: bool,
    /// Include GitHub issues
    #[serde(default)]
    include_issues: bool,
    /// Include persona knowledge (default: true)
    #[serde(default = "default_include_persona")]
    include_persona: bool,
    /// Maximum results (default: 10)
    #[serde(default = "default_limit")]
    limit: usize,
    /// Minimum score (default: 0.0)
    #[serde(default)]
    min_score: f32,
    /// Use hybrid search (RRF fusion of all oracles)
    #[serde(default)]
    hybrid: bool,
}

fn default_limit() -> usize {
    10
}

fn default_include_persona() -> bool {
    true
}

/// Scry API response
#[derive(Serialize)]
struct ScryResponse {
    results: Vec<ScryResultJson>,
    count: usize,
}

/// Single result in JSON format
#[derive(Serialize)]
struct ScryResultJson {
    id: i64,
    content: String,
    score: f32,
    event_type: String,
    source_id: String,
    timestamp: String,
}

/// Run the Mothership HTTP server
pub fn run_server(options: ServeOptions) -> Result<()> {
    let addr = format!("{}:{}", options.host, options.port);
    let state = Arc::new(ServerState::new());

    println!("🚀 Mothership daemon starting...");
    println!("   Listening on http://{}", addr);
    println!("   Press Ctrl+C to stop\n");

    rouille::start_server(&addr, move |request| {
        let state = Arc::clone(&state);
        handle_request(request, &state)
    });
}

/// Route requests to handlers
fn handle_request(request: &Request, state: &ServerState) -> Response {
    router!(request,
        // Health check
        (GET) ["/health"] => {
            handle_health(state)
        },

        // Version info
        (GET) ["/version"] => {
            handle_version(state)
        },

        // Scry API - semantic/lexical search
        (POST) ["/api/scry"] => {
            handle_scry(request)
        },

        // 404 for unknown routes
        _ => {
            Response::text("Not Found").with_status_code(404)
        }
    )
}

/// Handle GET /health
fn handle_health(state: &ServerState) -> Response {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: state.version.clone(),
        uptime_secs: state.uptime_secs(),
    };

    Response::json(&response)
}

/// Handle GET /version
fn handle_version(state: &ServerState) -> Response {
    Response::json(&serde_json::json!({
        "version": state.version,
        "name": "patina-mothership"
    }))
}

/// Handle POST /api/scry
fn handle_scry(request: &Request) -> Response {
    // Parse JSON body
    let body = match rouille::input::json_input::<ScryRequest>(request) {
        Ok(req) => req,
        Err(e) => {
            return Response::json(&serde_json::json!({
                "error": format!("Invalid JSON: {}", e)
            }))
            .with_status_code(400);
        }
    };

    // Handle hybrid mode (RRF fusion) vs standard mode
    let results: Vec<ScryResult> = if body.hybrid {
        // Use QueryEngine with RRF fusion
        let engine = QueryEngine::new();
        let query_opts = QueryOptions {
            repo: body.repo.clone(),
            all_repos: body.all_repos,
            include_issues: body.include_issues,
        };

        match engine.query_with_options(&body.query, body.limit, &query_opts) {
            Ok(fused) => fused
                .into_iter()
                .map(|r| ScryResult {
                    id: 0,
                    content: r.content,
                    score: r.fused_score,
                    event_type: r.sources.join("+"),
                    source_id: r.doc_id,
                    timestamp: r.metadata.timestamp.unwrap_or_default(),
                })
                .collect(),
            Err(e) => {
                return Response::json(&serde_json::json!({
                    "error": format!("Hybrid scry failed: {}", e)
                }))
                .with_status_code(500);
            }
        }
    } else {
        // Standard mode - single oracle + manual persona
        let options = ScryOptions {
            limit: body.limit,
            min_score: body.min_score,
            dimension: body.dimension,
            file: None, // File-based queries not supported via API yet
            repo: body.repo,
            all_repos: body.all_repos,
            include_issues: body.include_issues,
            include_persona: body.include_persona,
            hybrid: false,
            explain: false,
        };

        let mut results: Vec<ScryResult> = match scry::scry_text(&body.query, &options) {
            Ok(results) => results,
            Err(e) => {
                return Response::json(&serde_json::json!({
                    "error": format!("Scry failed: {}", e)
                }))
                .with_status_code(500);
            }
        };

        // Query persona if enabled
        if options.include_persona {
            if let Ok(persona_results) =
                persona::query(&body.query, options.limit, options.min_score, None)
            {
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

        // Sort combined results by score and truncate
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(options.limit);
        results
    };

    // Convert to JSON response
    let json_results: Vec<ScryResultJson> = results
        .into_iter()
        .map(|r| ScryResultJson {
            id: r.id,
            content: r.content,
            score: r.score,
            event_type: r.event_type,
            source_id: r.source_id,
            timestamp: r.timestamp,
        })
        .collect();

    let response = ScryResponse {
        count: json_results.len(),
        results: json_results,
    };

    Response::json(&response)
}
