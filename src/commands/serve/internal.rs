//! Internal implementation of the serve daemon

use anyhow::Result;
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

// === Transport-free request/response types ===
// Handlers use these — never rouille/hyper/raw-socket types.
// Transport adapter (rouille today, raw HTTP tomorrow) converts at the boundary.

/// HTTP request independent of transport
struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// HTTP response independent of transport
struct HttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpRequest {
    /// Get header value by name (case-insensitive)
    fn header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
    }
}

impl HttpResponse {
    /// Create a JSON response
    fn json(status: u16, value: &impl Serialize) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: serde_json::to_vec(value).unwrap_or_default(),
        }
    }

    /// Add a header
    fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

// === Server state ===

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

// === API types ===

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

// === Helpers ===

/// Generate a random 32-byte hex token
fn generate_token() -> String {
    (0..32)
        .map(|_| format!("{:02x}", fastrand::u8(..)))
        .collect()
}

/// Check bearer token authorization
fn check_auth(request: &HttpRequest, token: &str) -> bool {
    request
        .header("Authorization")
        .map(|h| h == format!("Bearer {}", token))
        .unwrap_or(false)
}

/// Add security headers to response
fn with_security_headers(response: HttpResponse) -> HttpResponse {
    response
        .with_header("X-Content-Type-Options", "nosniff")
        .with_header("X-Frame-Options", "DENY")
}

/// Consistent JSON error response
fn json_error(status: u16, message: &str) -> HttpResponse {
    HttpResponse::json(status, &serde_json::json!({"error": message}))
}

// === Transport-free handlers ===
// Business logic below this line never touches transport types.

/// Route request to handler
fn route_request(request: &HttpRequest, state: &ServerState) -> HttpResponse {
    let response = match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => handle_health(state),
        ("GET", "/version") => handle_version(state),
        ("POST", "/api/scry") => handle_scry(request, state),
        _ => json_error(404, "Not found"),
    };
    with_security_headers(response)
}

/// Handle GET /health
fn handle_health(state: &ServerState) -> HttpResponse {
    HttpResponse::json(
        200,
        &HealthResponse {
            status: "ok".to_string(),
            version: state.version.clone(),
            uptime_secs: state.uptime_secs(),
        },
    )
}

/// Handle GET /version
fn handle_version(state: &ServerState) -> HttpResponse {
    HttpResponse::json(
        200,
        &serde_json::json!({
            "version": state.version,
            "name": "patina-mother"
        }),
    )
}

/// Handle POST /api/scry
fn handle_scry(request: &HttpRequest, state: &ServerState) -> HttpResponse {
    // Auth check (bearer token required)
    if !check_auth(request, &state.token) {
        return json_error(401, "Unauthorized");
    }

    // Body already read and size-checked at transport boundary
    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    // Parse JSON from body
    let mut body: ScryRequest = match serde_json::from_slice(&request.body) {
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

    HttpResponse::json(200, &response)
}

// === rouille transport adapter ===
// Converts between rouille types and transport-free types.
// Replaced entirely when rouille is removed (commit 4).

/// Convert rouille request to transport-free HttpRequest
fn from_rouille(request: &rouille::Request) -> HttpRequest {
    let headers: Vec<(String, String)> = request
        .headers()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    // Read body with size cap (Read::take, not Content-Length trust)
    let body = match request.data() {
        Some(data) => {
            let mut buf = Vec::new();
            let _ = data.take((MAX_BODY_SIZE + 1) as u64).read_to_end(&mut buf);
            buf
        }
        None => Vec::new(),
    };

    HttpRequest {
        method: request.method().to_string(),
        path: request.url().to_string(),
        headers,
        body,
    }
}

/// Convert transport-free HttpResponse to rouille response
fn to_rouille(response: HttpResponse) -> rouille::Response {
    let content_type = response
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Content-Type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let mut r =
        rouille::Response::from_data(content_type, response.body).with_status_code(response.status);

    for (name, value) in &response.headers {
        if !name.eq_ignore_ascii_case("Content-Type") {
            r = r.with_additional_header(name.clone(), value.clone());
        }
    }
    r
}

/// Run the serve daemon
pub fn run_server(options: ServeOptions) -> Result<()> {
    let addr = format!("{}:{}", options.host, options.port);

    // Bind warning when not localhost
    if options.host != "127.0.0.1" && options.host != "localhost" {
        eprintln!(
            "WARNING: Binding to {} exposes the server to the network.",
            options.host
        );
        eprintln!(
            "  The server has no encryption (HTTP only). Use a reverse proxy for production."
        );
    }

    // Bearer token auth
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
        let req = from_rouille(request);
        // Body size enforcement at transport boundary
        if req.body.len() > MAX_BODY_SIZE {
            return to_rouille(with_security_headers(json_error(413, "Request too large")));
        }
        to_rouille(route_request(&req, &state))
    });
}
