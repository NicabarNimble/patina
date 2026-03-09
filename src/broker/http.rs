//! Production pipe/http handler for broker-spawned children.
//!
//! Dispatches on `AuthPlan` injection strategy (Bearer, Header, InProcess).
//! Imports from `connect` + `http_util` — no plugin auth types.
//! Per-child HTTP client caching. Domain normalization: lowercase, port-stripped.

use anyhow::Result;
use patina_pipe::harness::HttpHandler;
use patina_pipe_types::{PipeHttpRequest, PipeHttpResponse};
use std::collections::{HashMap, HashSet};

use crate::connect::{AuthPlan, InjectionStrategy, ResolvedCredential};
use crate::http_util;

/// Build a production HTTP handler for a broker-managed child.
///
/// The handler validates domains against the allowlist, injects credentials
/// per the AuthPlan's injection strategy, and returns the response. One HTTP
/// client per child (avoids TLS handshake churn).
///
/// Every pipe/http call emits a Measure event (verb=capture, tool=pipe,
/// mode=http) with duration, bytes, policy decision, and child name.
/// Set PATINA_SANDBOX_DEBUG=1 for enhanced domain rejection logging.
pub fn build_production_handler(auth_plan: &AuthPlan, child_name: &str) -> Result<HttpHandler> {
    let client = http_util::build_http_client()?;

    // Normalize allowlist: lowercase, port-stripped
    let allowed: HashSet<String> = auth_plan
        .allowed_domains
        .iter()
        .map(|d| normalize_domain(d))
        .collect();

    let cred: Option<ResolvedCredential> = auth_plan.credential.clone();
    let child_name_owned = child_name.to_string();
    let sandbox_debug = std::env::var("PATINA_SANDBOX_DEBUG").is_ok();

    Ok(Box::new(move |req: &PipeHttpRequest| {
        let start = std::time::Instant::now();

        let result: Result<PipeHttpResponse, String> = (|| {
            let domain = http_util::validate_http_url(&req.url)?;
            let normalized = normalize_domain(&domain);

            if !allowed.contains(&normalized) {
                if sandbox_debug {
                    eprintln!(
                        "[pipe/http] SANDBOX_DEBUG: '{}' not in allowlist {:?}",
                        domain, allowed
                    );
                }
                return Err(format!("domain '{}' not in allowlist", domain));
            }

            // Build the request
            let mut builder = match req.method.to_uppercase().as_str() {
                "GET" => client.get(&req.url),
                "POST" => client
                    .post(&req.url)
                    .body(req.body.clone().unwrap_or_default()),
                "PUT" => client
                    .put(&req.url)
                    .body(req.body.clone().unwrap_or_default()),
                "PATCH" => client
                    .patch(&req.url)
                    .body(req.body.clone().unwrap_or_default()),
                "DELETE" => client.delete(&req.url),
                other => return Err(format!("unsupported HTTP method: {}", other)),
            };

            // Add request headers
            for (key, value) in &req.headers {
                builder = builder.header(key, value);
            }

            // Inject credential based on AuthPlan injection strategy
            if let Some(ref resolved) = cred {
                match &resolved.injection {
                    InjectionStrategy::Bearer => {
                        builder = builder
                            .header("Authorization", format!("Bearer {}", resolved.value));
                    }
                    InjectionStrategy::Header { name } => {
                        builder = builder.header(name, &resolved.value);
                    }
                    InjectionStrategy::InProcess => {
                        // No HTTP injection — credential delivered via pipe/initialize
                    }
                }
            }

            let response = builder
                .send()
                .map_err(|e| format!("HTTP {} failed: {}", req.method, e))?;
            let status = response.status().as_u16();
            let resp_headers: HashMap<String, String> = response
                .headers()
                .iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|vs| (k.to_string(), vs.to_string())))
                .collect();
            let resp_body = response.text().map_err(|e| format!("read body: {}", e))?;

            // Leak detection: scan response for injected credential value
            let resp_body = match &cred {
                Some(ref resolved) => {
                    http_util::leak_check(&resp_body, "connection-credential", &resolved.value)
                }
                None => resp_body,
            };

            Ok(PipeHttpResponse {
                status,
                headers: resp_headers,
                body: resp_body,
            })
        })();

        // Measure emission: every pipe/http call gets an event
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        let (policy_allowed, status_code, response_bytes) = match &result {
            Ok(resp) => (1, resp.status as u64, resp.body.len() as u64),
            Err(msg) if msg.contains("not in allowlist") => (0, 0, 0),
            Err(_) => (1, 0, 0), // allowed but HTTP failed
        };
        crate::measure::emit_or_warn(
            "capture",
            "pipe",
            "http",
            &serde_json::json!({
                "duration_ms": duration_ms,
                "response_bytes": response_bytes,
                "status_code": status_code,
                "policy_allowed": policy_allowed,
                "child": &*child_name_owned,
            }),
        );

        result
    }))
}

/// Normalize a domain for comparison: lowercase, strip default port.
fn normalize_domain(domain: &str) -> String {
    let lower = domain.to_lowercase();
    // Strip :443 (implicit for HTTPS)
    lower.strip_suffix(":443").unwrap_or(&lower).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_lowercase() {
        assert_eq!(normalize_domain("API.GitHub.COM"), "api.github.com");
    }

    #[test]
    fn normalize_strip_port_443() {
        assert_eq!(normalize_domain("api.github.com:443"), "api.github.com");
    }

    #[test]
    fn normalize_keep_non_default_port() {
        assert_eq!(
            normalize_domain("api.github.com:8443"),
            "api.github.com:8443"
        );
    }

    #[test]
    fn normalize_combined() {
        assert_eq!(normalize_domain("API.GitHub.COM:443"), "api.github.com");
    }

    #[test]
    fn handler_rejects_unlisted_domain() {
        let plan = AuthPlan {
            child: "test-child".to_string(),
            credential: None,
            allowed_domains: vec!["api.github.com".to_string()],
        };
        let handler_result = build_production_handler(&plan, "test-child");
        assert!(handler_result.is_ok());
        let mut handler = handler_result.unwrap();

        let req = PipeHttpRequest {
            method: "GET".to_string(),
            url: "https://evil.com/data".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = handler(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not in allowlist"));
    }

    #[test]
    fn handler_with_bearer_auth_plan() {
        let plan = AuthPlan {
            child: "test-child".to_string(),
            credential: Some(ResolvedCredential {
                value: "ghp_test123".to_string(),
                injection: InjectionStrategy::Bearer,
            }),
            allowed_domains: vec!["api.github.com".to_string()],
        };
        // Just verify handler construction succeeds with AuthPlan
        let handler_result = build_production_handler(&plan, "test-child");
        assert!(handler_result.is_ok());
    }

    #[test]
    fn handler_with_header_auth_plan() {
        let plan = AuthPlan {
            child: "test-child".to_string(),
            credential: Some(ResolvedCredential {
                value: "apikey123".to_string(),
                injection: InjectionStrategy::Header {
                    name: "X-Api-Key".to_string(),
                },
            }),
            allowed_domains: vec!["api.example.com".to_string()],
        };
        let handler_result = build_production_handler(&plan, "test-child");
        assert!(handler_result.is_ok());
    }

    #[test]
    fn handler_with_inprocess_auth_plan() {
        let plan = AuthPlan {
            child: "test-child".to_string(),
            credential: Some(ResolvedCredential {
                value: "token123".to_string(),
                injection: InjectionStrategy::InProcess,
            }),
            allowed_domains: vec![],
        };
        // InProcess means no HTTP injection — handler still builds
        let handler_result = build_production_handler(&plan, "test-child");
        assert!(handler_result.is_ok());
    }
}
