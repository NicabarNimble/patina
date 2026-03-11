//! Production pipe/http handler for broker-spawned children.
//!
//! Thin mapper: AuthPlan -> HttpProxyConfig, then wraps the shared proxy
//! with broker-owned telemetry (Measure events). The security stack lives
//! in patina-pipe's http_proxy module.

use anyhow::Result;
use patina_pipe::harness::HttpHandler;
use patina_pipe::http_proxy::{
    build_http_proxy, HttpProxyConfig, ProxyCredential, ProxyInjection,
};
use patina_pipe_types::PipeHttpRequest;

use crate::connect::{AuthPlan, InjectionStrategy};

/// Map an AuthPlan's credential to proxy types.
fn map_credential(auth_plan: &AuthPlan) -> Option<ProxyCredential> {
    auth_plan.credential.as_ref().map(|c| ProxyCredential {
        value: c.value.clone(),
        injection: match &c.injection {
            InjectionStrategy::Bearer => ProxyInjection::Bearer,
            InjectionStrategy::Header { name } => ProxyInjection::Header {
                name: name.clone(),
            },
            InjectionStrategy::InProcess => ProxyInjection::InProcess,
        },
    })
}

/// Build a production HTTP handler for a broker-managed child.
///
/// Maps AuthPlan to HttpProxyConfig, builds the shared proxy, then wraps
/// it with Measure emission. Every pipe/http call gets a telemetry event.
pub fn build_production_handler(auth_plan: &AuthPlan, child_name: &str) -> Result<HttpHandler> {
    let config = HttpProxyConfig {
        allowed_domains: auth_plan.allowed_domains.clone(),
        credential: map_credential(auth_plan),
    };

    let mut proxy = build_http_proxy(config).map_err(|e| anyhow::anyhow!(e))?;
    let child_name_owned = child_name.to_string();

    Ok(Box::new(move |req: &PipeHttpRequest| {
        let start = std::time::Instant::now();
        let result = proxy(req);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connect::ResolvedCredential;
    use patina_pipe::http_proxy::normalize_domain;
    use std::collections::HashMap;

    // Regression tests for normalize_domain — the implementation moved
    // to patina-pipe but the behavior contract stays.
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
        let handler_result = build_production_handler(&plan, "test-child");
        assert!(handler_result.is_ok());
    }
}
