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

    // =========================================================================
    // Exhaustive AuthPlan -> HttpProxyConfig mapping tests
    //
    // The mapping is a security boundary — the only place where policy
    // decisions translate to proxy behavior. These tests catch regressions
    // if new InjectionStrategy variants are added.
    // =========================================================================

    #[test]
    fn map_bearer_produces_proxy_bearer() {
        let plan = AuthPlan {
            child: "test".to_string(),
            credential: Some(ResolvedCredential {
                value: "ghp_abc".to_string(),
                injection: InjectionStrategy::Bearer,
            }),
            allowed_domains: vec![],
        };
        let cred = map_credential(&plan).expect("credential should be Some");
        assert_eq!(cred.value, "ghp_abc");
        assert!(matches!(cred.injection, ProxyInjection::Bearer));
    }

    #[test]
    fn map_header_produces_proxy_header_with_name() {
        let plan = AuthPlan {
            child: "test".to_string(),
            credential: Some(ResolvedCredential {
                value: "key123".to_string(),
                injection: InjectionStrategy::Header {
                    name: "X-Api-Key".to_string(),
                },
            }),
            allowed_domains: vec![],
        };
        let cred = map_credential(&plan).expect("credential should be Some");
        assert_eq!(cred.value, "key123");
        match cred.injection {
            ProxyInjection::Header { name } => assert_eq!(name, "X-Api-Key"),
            other => panic!("expected Header, got {:?}", variant_name(&other)),
        }
    }

    #[test]
    fn map_inprocess_produces_proxy_inprocess() {
        let plan = AuthPlan {
            child: "test".to_string(),
            credential: Some(ResolvedCredential {
                value: "secret".to_string(),
                injection: InjectionStrategy::InProcess,
            }),
            allowed_domains: vec![],
        };
        let cred = map_credential(&plan).expect("credential should be Some");
        assert_eq!(cred.value, "secret");
        assert!(matches!(cred.injection, ProxyInjection::InProcess));
    }

    #[test]
    fn map_no_credential_produces_none() {
        let plan = AuthPlan {
            child: "test".to_string(),
            credential: None,
            allowed_domains: vec!["api.github.com".to_string()],
        };
        assert!(map_credential(&plan).is_none());
    }

    /// The dangerous direction: InProcess credentials must NOT become
    /// Bearer or Header injections. This would expose secrets via HTTP
    /// headers that should only travel via pipe/initialize.
    #[test]
    fn inprocess_never_becomes_bearer_or_header() {
        let plan = AuthPlan {
            child: "test".to_string(),
            credential: Some(ResolvedCredential {
                value: "pipe-only-secret".to_string(),
                injection: InjectionStrategy::InProcess,
            }),
            allowed_domains: vec![],
        };
        let cred = map_credential(&plan).unwrap();
        assert!(
            !matches!(cred.injection, ProxyInjection::Bearer),
            "InProcess must not map to Bearer"
        );
        assert!(
            !matches!(cred.injection, ProxyInjection::Header { .. }),
            "InProcess must not map to Header"
        );
    }

    #[test]
    fn map_domains_passed_through() {
        let plan = AuthPlan {
            child: "test".to_string(),
            credential: None,
            allowed_domains: vec![
                "api.github.com".to_string(),
                "uploads.github.com".to_string(),
            ],
        };
        let config = HttpProxyConfig {
            allowed_domains: plan.allowed_domains.clone(),
            credential: map_credential(&plan),
        };
        assert_eq!(config.allowed_domains.len(), 2);
        assert_eq!(config.allowed_domains[0], "api.github.com");
        assert_eq!(config.allowed_domains[1], "uploads.github.com");
    }

    /// Helper to name a ProxyInjection variant for error messages.
    fn variant_name(inj: &ProxyInjection) -> &'static str {
        match inj {
            ProxyInjection::Bearer => "Bearer",
            ProxyInjection::Header { .. } => "Header",
            ProxyInjection::InProcess => "InProcess",
        }
    }
}
