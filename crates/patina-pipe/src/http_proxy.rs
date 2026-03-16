//! Shared HTTP proxy security stack for pipe protocol children.
//!
//! Domain validation, credential injection, leak detection, cross-domain
//! redirect rejection. No telemetry — callers own their own observability.
//! Per [[telemetry-is-process-owned]].
//!
//! Feature-gated behind `http-proxy` — children that don't proxy HTTP
//! get the lean protocol-only crate.

use std::collections::{HashMap, HashSet};

use patina_pipe_types::{PipeHttpRequest, PipeHttpResponse};

use crate::harness::HttpHandler;

/// Configuration for building an HTTP proxy handler.
pub struct HttpProxyConfig {
    /// Domains the proxy is allowed to contact (normalized: lowercase, port-stripped).
    pub allowed_domains: Vec<String>,
    /// Credential to inject into outgoing requests. None if auth not required.
    pub credential: Option<ProxyCredential>,
}

/// A resolved credential for HTTP proxy injection.
pub struct ProxyCredential {
    /// The decrypted secret value.
    pub value: String,
    /// How to inject this credential into HTTP requests.
    pub injection: ProxyInjection,
}

/// How a credential is injected into proxied HTTP requests.
pub enum ProxyInjection {
    /// `Authorization: Bearer {value}`
    Bearer,
    /// Custom header: `{name}: {value}`
    Header { name: String },
    /// Credential delivered via pipe/initialize, not injected into HTTP.
    InProcess,
}

/// Build an HTTP proxy handler from configuration.
///
/// The returned handler enforces:
/// 1. HTTPS only — rejects `http://`
/// 2. No IP/localhost — rejects IPs, localhost
/// 3. Domain allowlist — normalized, case-insensitive, port-stripped
/// 4. Credential injection — Bearer, Header, or InProcess (no-op)
/// 5. Cross-domain redirect rejection — stops on host change
/// 6. Leak detection — scans response for credential value
///
/// Set `PATINA_SANDBOX_DEBUG=1` for enhanced domain rejection logging.
pub fn build_http_proxy(config: HttpProxyConfig) -> Result<HttpHandler, String> {
    let client = build_http_client().map_err(|e| e.to_string())?;

    let allowed: HashSet<String> = config
        .allowed_domains
        .iter()
        .map(|d| normalize_domain(d))
        .collect();

    let cred = config.credential;
    let sandbox_debug = std::env::var("PATINA_SANDBOX_DEBUG").is_ok();

    Ok(Box::new(move |req: &PipeHttpRequest| {
        let domain = validate_http_url(&req.url)?;
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

        // Inject credential based on injection strategy
        if let Some(ref resolved) = cred {
            match &resolved.injection {
                ProxyInjection::Bearer => {
                    builder = builder.header("Authorization", format!("Bearer {}", resolved.value));
                }
                ProxyInjection::Header { name } => {
                    builder = builder.header(name, &resolved.value);
                }
                ProxyInjection::InProcess => {
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
            Some(ref resolved) => leak_check(&resp_body, "connection-credential", &resolved.value),
            None => resp_body,
        };

        Ok(PipeHttpResponse {
            status,
            headers: resp_headers,
            body: resp_body,
        })
    }))
}

/// Build an HTTP client with cross-domain redirect rejection.
///
/// If a response redirects to a different host, the request is stopped
/// (prevents allowlist bypass via open redirectors).
pub fn build_http_client() -> anyhow::Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent(format!("patina/{}", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.url().host_str() != attempt.previous().last().and_then(|u| u.host_str()) {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|e| anyhow::anyhow!("build HTTP client: {}", e))
}

/// Validate and parse an HTTP URL for domain-allowlisted access.
///
/// Returns the extracted domain on success. Enforces:
/// - HTTPS only (no plaintext HTTP)
/// - No IP addresses (IPv4 or IPv6)
/// - No localhost
pub fn validate_http_url(url: &str) -> Result<String, String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("invalid URL: {}", e))?;

    // HTTPS only
    if parsed.scheme() != "https" {
        return Err(format!("only HTTPS allowed, got '{}'", parsed.scheme()));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "no host in URL".to_string())?;

    // No localhost
    if host == "localhost" {
        return Err("localhost not allowed".to_string());
    }

    // No IP addresses (IPv4 or IPv6)
    // host_str() returns brackets for IPv6 (e.g., "[::1]") — strip them
    let bare_host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    if bare_host.parse::<std::net::IpAddr>().is_ok() {
        return Err("IP addresses not allowed".to_string());
    }

    Ok(bare_host.to_string())
}

/// Scan response body for leaked credential values, replacing with [REDACTED].
pub fn leak_check(body: &str, secret_name: &str, secret_value: &str) -> String {
    if body.contains(secret_value) {
        eprintln!(
            "[host] credential leak detected in response: secret '{}' found in body, redacting",
            secret_name
        );
        body.replace(secret_value, "[REDACTED]")
    } else {
        body.to_string()
    }
}

/// Normalize a domain for comparison: lowercase, strip default port.
pub fn normalize_domain(domain: &str) -> String {
    let lower = domain.to_lowercase();
    // Strip :443 (implicit for HTTPS)
    lower.strip_suffix(":443").unwrap_or(&lower).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_https_only() {
        assert!(validate_http_url("http://example.com").is_err());
        assert!(validate_http_url("https://example.com").is_ok());
    }

    #[test]
    fn validate_no_localhost() {
        assert!(validate_http_url("https://localhost/path").is_err());
    }

    #[test]
    fn validate_no_ip_addresses() {
        assert!(validate_http_url("https://127.0.0.1/path").is_err());
        assert!(validate_http_url("https://[::1]/path").is_err());
    }

    #[test]
    fn validate_returns_domain() {
        assert_eq!(
            validate_http_url("https://api.github.com/repos").unwrap(),
            "api.github.com"
        );
    }

    #[test]
    fn leak_check_detects_leak() {
        let body = "token is ghp_secret123 here";
        let result = leak_check(body, "github-token", "ghp_secret123");
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("ghp_secret123"));
    }

    #[test]
    fn leak_check_passes_clean_body() {
        let body = "no secrets here";
        let result = leak_check(body, "github-token", "ghp_secret123");
        assert_eq!(result, body);
    }

    #[test]
    fn build_client_succeeds() {
        assert!(build_http_client().is_ok());
    }

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
    fn build_proxy_rejects_unlisted_domain() {
        let config = HttpProxyConfig {
            allowed_domains: vec!["api.github.com".to_string()],
            credential: None,
        };
        let mut proxy = build_http_proxy(config).expect("build_http_proxy failed");

        let req = PipeHttpRequest {
            method: "GET".to_string(),
            url: "https://evil.com/data".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = proxy(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not in allowlist"));
    }

    #[test]
    fn build_proxy_rejects_http() {
        let config = HttpProxyConfig {
            allowed_domains: vec!["example.com".to_string()],
            credential: None,
        };
        let mut proxy = build_http_proxy(config).expect("build_http_proxy failed");

        let req = PipeHttpRequest {
            method: "GET".to_string(),
            url: "http://example.com/data".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = proxy(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("HTTPS"));
    }

    #[test]
    fn build_proxy_rejects_ip() {
        let config = HttpProxyConfig {
            allowed_domains: vec![],
            credential: None,
        };
        let mut proxy = build_http_proxy(config).expect("build_http_proxy failed");

        let req = PipeHttpRequest {
            method: "GET".to_string(),
            url: "https://127.0.0.1/data".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = proxy(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("IP addresses not allowed"));
    }

    #[test]
    fn build_proxy_inprocess_does_not_inject() {
        // InProcess credential should not cause HTTP injection
        let config = HttpProxyConfig {
            allowed_domains: vec![],
            credential: Some(ProxyCredential {
                value: "secret-token".to_string(),
                injection: ProxyInjection::InProcess,
            }),
        };
        // Just verify construction succeeds — InProcess is a no-op for HTTP
        let _proxy = build_http_proxy(config).expect("build_http_proxy failed");
    }
}
