//! Production pipe/http handler for broker-spawned children.
//!
//! Reuses host_support functions (validate_http_url, build_http_client,
//! inject_credential, leak_check) with pub(crate) visibility.
//! Per-child HTTP client caching. Domain normalization: lowercase, port-stripped.

use anyhow::Result;
use patina_pipe::harness::HttpHandler;
use patina_pipe_types::{PipeHttpRequest, PipeHttpResponse};
use std::collections::{HashMap, HashSet};

use crate::plugin::internal::host_support;
use crate::plugin::{CredentialMapping, InjectionLocation};

/// Build a production HTTP handler for a broker-managed child.
///
/// The handler validates domains against the allowlist, injects credentials
/// for authed domains, and returns the response. One HTTP client per child
/// (avoids TLS handshake churn).
///
/// Every pipe/http call emits a Measure event (verb=capture, tool=pipe,
/// mode=http) with duration, bytes, policy decision, and child name.
/// Set PATINA_SANDBOX_DEBUG=1 for enhanced domain rejection logging.
pub fn build_production_handler(
    allowed_domains: &[String],
    credential: Option<(String, String)>, // (secret_name, secret_value)
    child_name: &str,
) -> Result<HttpHandler> {
    let client = host_support::build_http_client()?;

    // Normalize allowlist: lowercase, port-stripped
    let allowed: HashSet<String> = allowed_domains
        .iter()
        .map(|d| normalize_domain(d))
        .collect();

    let cred = credential;
    let child_name_owned = child_name.to_string();
    let sandbox_debug = std::env::var("PATINA_SANDBOX_DEBUG").is_ok();

    Ok(Box::new(move |req: &PipeHttpRequest| {
        let start = std::time::Instant::now();

        let result: Result<PipeHttpResponse, String> = (|| {
            let domain = host_support::validate_http_url(&req.url)?;
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

            // Inject credential if available for this domain
            if let Some((ref _secret_name, ref secret_value)) = cred {
                let mapping = CredentialMapping {
                    secret_name: String::new(),
                    location: InjectionLocation::Bearer,
                };
                builder = host_support::inject_credential(builder, &mapping, secret_value);
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
                Some((ref secret_name, ref secret_value)) => {
                    host_support::leak_check(&resp_body, secret_name, secret_value)
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
        let handler_result =
            build_production_handler(&["api.github.com".to_string()], None, "test-child");
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
}
