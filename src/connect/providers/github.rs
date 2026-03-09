//! GitHub provider — OAuth device flow (RFC 8628) credential acquisition.
//!
//! Implements the Provider trait for GitHub. Handles:
//! - OAuth device flow: POST /login/device/code → poll /login/oauth/access_token
//! - Manual token: user-provided PAT
//! - Account probe: GET /user → login
//!
//! All network calls use reqwest (blocking). The device flow prints a
//! user code and opens the verification URL in the default browser.
//! User consent is required before any network activity.

use crate::connect::internal::model::{AuthMethod, ConnectError, InjectionStrategy};
use crate::connect::providers::{AcquisitionResult, Provider};

/// GitHub OAuth App client ID.
///
/// TODO: Replace with registered OAuth App client_id.
/// See DESIGN.md Open Question 1 — registration is quick but requires
/// a GitHub account decision (personal vs org). Code is testable without it.
const GITHUB_CLIENT_ID: &str = "PLACEHOLDER_CLIENT_ID";

/// Default scopes for GitHub OAuth.
const DEFAULT_SCOPES: &[&str] = &["repo", "read:org"];

/// Poll interval floor (seconds). GitHub enforces a minimum of 5s.
const DEVICE_POLL_INTERVAL_SECS: u64 = 5;

/// Maximum poll duration before timeout (seconds).
const DEVICE_POLL_TIMEOUT_SECS: u64 = 900; // 15 minutes

pub struct GitHubProvider;

impl Provider for GitHubProvider {
    fn name(&self) -> &str {
        "github"
    }

    fn acquire(&self) -> Result<AcquisitionResult, ConnectError> {
        // Step 1: Request device code
        let device = request_device_code()?;

        // Step 2: Show user code and open browser
        eprintln!();
        eprintln!("  GitHub OAuth Device Authorization");
        eprintln!("  ─────────────────────────────────");
        eprintln!("  Your code: {}", device.user_code);
        eprintln!();
        eprintln!("  Opening: {}", device.verification_uri);
        eprintln!("  Paste the code above, then authorize the app.");
        eprintln!();

        // Attempt to open the verification URI in the user's browser.
        // Fall back silently — the user code is already displayed.
        let _ = open_browser(&device.verification_uri);

        // Step 3: Poll for token
        let token = poll_for_token(&device)?;

        // Step 4: Probe account
        let account_id = match probe_github_account(&token) {
            Ok(Some(login)) => {
                eprintln!("  Authenticated as: {}", login);
                Some(login)
            }
            Ok(None) => None,
            Err(e) => {
                eprintln!("  Warning: could not probe account: {}", e);
                None
            }
        };

        Ok(AcquisitionResult {
            credential: token,
            account_id,
            auth_method: AuthMethod::OAuth,
            scopes: DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
            expires_at: None, // GitHub OAuth tokens don't expire by default
        })
    }

    fn acquire_manual(&self, token: &str) -> Result<AcquisitionResult, ConnectError> {
        // Optionally probe account to populate account_id
        let account_id = match probe_github_account(token) {
            Ok(login) => login,
            Err(e) => {
                eprintln!("  Warning: could not probe account: {}", e);
                None
            }
        };

        Ok(AcquisitionResult {
            credential: token.to_string(),
            account_id,
            auth_method: AuthMethod::Manual,
            scopes: vec![], // Unknown for manual tokens
            expires_at: None,
        })
    }

    fn probe_account(&self, credential: &str) -> Result<Option<String>, ConnectError> {
        probe_github_account(credential)
    }

    fn default_scopes(&self) -> Vec<String> {
        DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect()
    }

    fn default_child(&self) -> &str {
        "github-connector"
    }

    fn default_injection(&self) -> InjectionStrategy {
        InjectionStrategy::Bearer
    }

    fn allowed_domains(&self) -> Vec<String> {
        vec!["api.github.com".into()]
    }
}

// =============================================================================
// OAuth Device Flow (RFC 8628)
// =============================================================================

/// Response from GitHub's device code endpoint.
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    interval: u64,
    expires_in: u64,
}

/// Step 1: POST to /login/device/code to get a device code + user code.
fn request_device_code() -> Result<DeviceCodeResponse, ConnectError> {
    let scope = DEFAULT_SCOPES.join(" ");

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[("client_id", GITHUB_CLIENT_ID), ("scope", &scope)])
        .send()
        .map_err(|e| ConnectError::AcquisitionFailed {
            provider: "github".into(),
            detail: format!("device code request failed: {}", e),
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(ConnectError::AcquisitionFailed {
            provider: "github".into(),
            detail: format!("device code request returned {}: {}", status, body),
        });
    }

    let json: serde_json::Value = resp.json().map_err(|e| ConnectError::AcquisitionFailed {
        provider: "github".into(),
        detail: format!("device code response parse error: {}", e),
    })?;

    Ok(DeviceCodeResponse {
        device_code: json_str(&json, "device_code", "github")?,
        user_code: json_str(&json, "user_code", "github")?,
        verification_uri: json_str(&json, "verification_uri", "github")?,
        interval: json["interval"]
            .as_u64()
            .unwrap_or(DEVICE_POLL_INTERVAL_SECS),
        expires_in: json["expires_in"]
            .as_u64()
            .unwrap_or(DEVICE_POLL_TIMEOUT_SECS),
    })
}

/// Step 3: Poll /login/oauth/access_token until the user authorizes.
fn poll_for_token(device: &DeviceCodeResponse) -> Result<String, ConnectError> {
    let client = reqwest::blocking::Client::new();
    let interval = std::cmp::max(device.interval, DEVICE_POLL_INTERVAL_SECS);
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs(std::cmp::min(
            device.expires_in,
            DEVICE_POLL_TIMEOUT_SECS,
        ));

    loop {
        std::thread::sleep(std::time::Duration::from_secs(interval));

        if std::time::Instant::now() > deadline {
            return Err(ConnectError::AcquisitionFailed {
                provider: "github".into(),
                detail: "device flow timed out — user did not authorize in time".into(),
            });
        }

        let resp = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&[
                ("client_id", GITHUB_CLIENT_ID),
                ("device_code", device.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .map_err(|e| ConnectError::AcquisitionFailed {
                provider: "github".into(),
                detail: format!("token poll request failed: {}", e),
            })?;

        let json: serde_json::Value = resp.json().map_err(|e| ConnectError::AcquisitionFailed {
            provider: "github".into(),
            detail: format!("token poll response parse error: {}", e),
        })?;

        // Check for error responses per RFC 8628 §3.5
        if let Some(error) = json["error"].as_str() {
            match error {
                "authorization_pending" => {
                    // User hasn't authorized yet — keep polling
                    continue;
                }
                "slow_down" => {
                    // Server wants us to increase the interval — sleep extra
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    continue;
                }
                "expired_token" => {
                    return Err(ConnectError::AcquisitionFailed {
                        provider: "github".into(),
                        detail: "device code expired — please try again".into(),
                    });
                }
                "access_denied" => {
                    return Err(ConnectError::AcquisitionFailed {
                        provider: "github".into(),
                        detail: "user denied authorization".into(),
                    });
                }
                other => {
                    let desc = json["error_description"]
                        .as_str()
                        .unwrap_or("unknown error");
                    return Err(ConnectError::AcquisitionFailed {
                        provider: "github".into(),
                        detail: format!("{}: {}", other, desc),
                    });
                }
            }
        }

        // Success — extract the access token
        if let Some(token) = json["access_token"].as_str() {
            return Ok(token.to_string());
        }

        // Unexpected response shape
        return Err(ConnectError::AcquisitionFailed {
            provider: "github".into(),
            detail: format!("unexpected token response: {}", json),
        });
    }
}

// =============================================================================
// Account Probe
// =============================================================================

/// GET https://api.github.com/user → login name.
fn probe_github_account(credential: &str) -> Result<Option<String>, ConnectError> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", credential))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "patina")
        .send()
        .map_err(|e| ConnectError::AcquisitionFailed {
            provider: "github".into(),
            detail: format!("account probe failed: {}", e),
        })?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let json: serde_json::Value = resp.json().map_err(|e| ConnectError::AcquisitionFailed {
        provider: "github".into(),
        detail: format!("account probe parse error: {}", e),
    })?;

    Ok(json["login"].as_str().map(String::from))
}

// =============================================================================
// Helpers
// =============================================================================

/// Extract a required string field from JSON.
fn json_str(json: &serde_json::Value, field: &str, provider: &str) -> Result<String, ConnectError> {
    json[field]
        .as_str()
        .map(String::from)
        .ok_or_else(|| ConnectError::AcquisitionFailed {
            provider: provider.into(),
            detail: format!("missing '{}' in response", field),
        })
}

/// Open a URL in the default browser. Best-effort — returns error on failure.
fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("failed to open browser: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("failed to open browser: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", url])
            .spawn()
            .map_err(|e| format!("failed to open browser: {}", e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_provider_name() {
        let provider = GitHubProvider;
        assert_eq!(provider.name(), "github");
    }

    #[test]
    fn github_provider_default_scopes() {
        let provider = GitHubProvider;
        let scopes = provider.default_scopes();
        assert_eq!(scopes, vec!["repo", "read:org"]);
    }

    #[test]
    fn github_provider_default_child() {
        let provider = GitHubProvider;
        assert_eq!(provider.default_child(), "github-connector");
    }

    #[test]
    fn github_provider_default_injection() {
        let provider = GitHubProvider;
        assert_eq!(provider.default_injection(), InjectionStrategy::Bearer);
    }

    #[test]
    fn github_provider_allowed_domains() {
        let provider = GitHubProvider;
        assert_eq!(provider.allowed_domains(), vec!["api.github.com"]);
    }

    #[test]
    fn github_provider_defaults_match_child_toml() {
        // Verify our defaults match children/github-connector/child.toml:
        //   [auth] required = true, provider = "github"
        //   [domains] allowed = ["api.github.com"]
        let provider = GitHubProvider;
        assert_eq!(provider.name(), "github");
        assert_eq!(provider.allowed_domains(), vec!["api.github.com"]);
        assert_eq!(provider.default_child(), "github-connector");
    }

    #[test]
    fn json_str_extracts_field() {
        let json = serde_json::json!({"foo": "bar", "num": 42});
        assert_eq!(json_str(&json, "foo", "test").unwrap(), "bar");
    }

    #[test]
    fn json_str_missing_field_returns_error() {
        let json = serde_json::json!({"foo": "bar"});
        let err = json_str(&json, "missing", "test");
        assert!(matches!(err, Err(ConnectError::AcquisitionFailed { .. })));
    }

    #[test]
    fn json_str_non_string_field_returns_error() {
        let json = serde_json::json!({"num": 42});
        let err = json_str(&json, "num", "test");
        assert!(matches!(err, Err(ConnectError::AcquisitionFailed { .. })));
    }

    #[test]
    fn acquisition_result_oauth_fields() {
        let result = AcquisitionResult {
            credential: "ghp_test123".into(),
            account_id: Some("octocat".into()),
            auth_method: AuthMethod::OAuth,
            scopes: vec!["repo".into(), "read:org".into()],
            expires_at: None,
        };
        assert_eq!(result.credential, "ghp_test123");
        assert_eq!(result.account_id.as_deref(), Some("octocat"));
        assert_eq!(result.auth_method, AuthMethod::OAuth);
        assert_eq!(result.scopes.len(), 2);
    }

    #[test]
    fn acquisition_result_manual_fields() {
        let result = AcquisitionResult {
            credential: "user-token".into(),
            account_id: None,
            auth_method: AuthMethod::Manual,
            scopes: vec![],
            expires_at: Some("2027-01-01T00:00:00Z".into()),
        };
        assert_eq!(result.auth_method, AuthMethod::Manual);
        assert!(result.account_id.is_none());
        assert!(result.expires_at.is_some());
    }
}
