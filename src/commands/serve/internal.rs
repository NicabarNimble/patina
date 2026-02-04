//! Internal implementation of the Mother server

use anyhow::Result;
use rouille::{router, Request, Response};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::Arc;
use std::time::Instant;

use super::ServeOptions;
use crate::commands::persona;
use crate::commands::scry::{self, ScryOptions, ScryResult};
use crate::retrieval::{QueryEngine, QueryOptions};

/// Maximum request body size (1 MB)
const MAX_BODY_SIZE: usize = 1_048_576;

/// Maximum results per query
const MAX_LIMIT: usize = 1000;

/// Server state shared across request handlers
pub struct ServerState {
    start_time: Instant,
    version: String,
    token: String,
}

impl ServerState {
    fn new(token: String) -> Self {
        Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            token,
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

/// Generate a random 32-byte hex token
fn generate_token() -> String {
    (0..32)
        .map(|_| format!("{:02x}", fastrand::u8(..)))
        .collect()
}

/// Check bearer token authorization
fn check_auth(request: &Request, token: &str) -> bool {
    request
        .header("Authorization")
        .map(|h| h == format!("Bearer {}", token))
        .unwrap_or(false)
}

/// Add security headers to a response (CORS deny-by-default: omit origin header entirely)
fn with_security_headers(response: Response) -> Response {
    response
        .with_additional_header("X-Content-Type-Options", "nosniff")
        .with_additional_header("X-Frame-Options", "DENY")
}

/// Consistent JSON error response
fn json_error(status: u16, message: &str) -> Response {
    Response::json(&serde_json::json!({"error": message})).with_status_code(status)
}

/// Run the Mother HTTP server
pub fn run_server(options: ServeOptions) -> Result<()> {
    let addr = format!("{}:{}", options.host, options.port);

    // Fix 6: Bind warning when not localhost
    if options.host != "127.0.0.1" && options.host != "localhost" {
        eprintln!(
            "WARNING: Binding to {} exposes the server to the network.",
            options.host
        );
        eprintln!(
            "  The server has no encryption (HTTP only). Use a reverse proxy for production."
        );
    }

    // Fix 1: Bearer token auth
    let token = std::env::var("PATINA_SERVE_TOKEN").unwrap_or_else(|_| {
        let t = generate_token();
        eprintln!("Generated auth token: {}", t);
        eprintln!("  Set PATINA_SERVE_TOKEN={} to use a fixed token", t);
        t
    });

    let state = Arc::new(ServerState::new(token));

    println!("🚀 Mother daemon starting...");
    println!("   Listening on http://{}", addr);
    println!("   Press Ctrl+C to stop\n");

    rouille::start_server(&addr, move |request| {
        let state = Arc::clone(&state);
        handle_request(request, &state)
    });
}

/// Route requests to handlers
fn handle_request(request: &Request, state: &ServerState) -> Response {
    let response = router!(request,
        // Health check (no auth required)
        (GET) ["/health"] => {
            handle_health(state)
        },

        // Version info (no auth required)
        (GET) ["/version"] => {
            handle_version(state)
        },

        // Scry API - semantic/lexical search (auth required)
        (POST) ["/api/scry"] => {
            handle_scry(request, state)
        },

        // 404 for unknown routes
        _ => {
            json_error(404, "Not found")
        }
    );

    with_security_headers(response)
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
        "name": "patina-mother"
    }))
}

/// Handle POST /api/scry
fn handle_scry(request: &Request, state: &ServerState) -> Response {
    // Auth check (bearer token required)
    if !check_auth(request, &state.token) {
        return json_error(401, "Unauthorized");
    }

    // Read body with size cap — do not trust Content-Length
    let data = match request.data() {
        Some(d) => d,
        None => return json_error(400, "Missing request body"),
    };
    let mut buf = Vec::new();
    if let Err(e) = data.take((MAX_BODY_SIZE + 1) as u64).read_to_end(&mut buf) {
        return json_error(400, &format!("Failed to read body: {}", e));
    }
    if buf.len() > MAX_BODY_SIZE {
        return json_error(413, "Request too large");
    }

    // Parse JSON from bytes
    let mut body: ScryRequest = match serde_json::from_slice(&buf) {
        Ok(req) => req,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    // Cap limit
    body.limit = body.limit.min(MAX_LIMIT);

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
                return json_error(500, &format!("Hybrid scry failed: {}", e));
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
            ..Default::default()
        };

        let mut results: Vec<ScryResult> = match scry::scry_text(&body.query, &options) {
            Ok(results) => results,
            Err(e) => {
                return json_error(500, &format!("Scry failed: {}", e));
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
