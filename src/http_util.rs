//! Shared HTTP utilities — URL validation, client construction, leak detection.
//!
//! Pure HTTP plumbing. No plugin concepts, no connection concepts.
//! Used by both `plugin/internal/host_support.rs` and `broker/http.rs`.

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
}
