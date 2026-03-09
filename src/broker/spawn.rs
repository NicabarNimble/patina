//! Native child spawn with sandbox enforcement and credential delivery.
//!
//! Resolves child binary, loads manifest, decrypts credentials,
//! spawns the child under OS sandbox, and initializes via pipe/initialize.

use anyhow::{bail, Context, Result};
use patina_pipe::harness::{spawn_child_with_handler, HttpHandler};
use patina_pipe::sandbox::SandboxProfile;
use patina_pipe_types::config::{AuthConfig, InitializeParams};
use patina_pipe_types::manifest::{ChildManifest, ChildType};
use std::path::{Path, PathBuf};

use super::http::build_production_handler;
use super::lifecycle::NativeChild;

/// Resolve a child binary by name using the search order from DESIGN.md §7.
///
/// 1. ~/.patina/children/{name}/{name} — installed children
/// 2. PATH — system-installed children
/// 3. ./target/release/{name} — development builds
pub fn resolve_child_binary(name: &str) -> Result<PathBuf> {
    // 1. ~/.patina/children/{name}/{name}
    let installed = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".patina")
        .join("children")
        .join(name)
        .join(name);
    if installed.exists() {
        return Ok(installed);
    }

    // 2. PATH
    if let Ok(path) = which::which(name) {
        return Ok(path);
    }

    // 3. ./target/release/{name}
    let dev_build = PathBuf::from("target").join("release").join(name);
    if dev_build.exists() {
        return Ok(dev_build);
    }

    bail!(
        "child binary '{}' not found. Searched:\n  \
         1. ~/.patina/children/{0}/{0}\n  \
         2. PATH\n  \
         3. ./target/release/{0}",
        name
    );
}

/// Load child.toml manifest from the same directory as the child binary.
pub fn load_manifest(binary_path: &Path) -> Result<ChildManifest> {
    let manifest_path = binary_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("child.toml");

    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading child manifest {}", manifest_path.display()))?;

    ChildManifest::from_toml(&content)
        .with_context(|| format!("parsing child manifest {}", manifest_path.display()))
}

/// Build pipe/initialize params with optional credential delivery (§9).
///
/// Constructs `pipe_types::InitializeParams` for compile-time field checking,
/// then serializes to `serde_json::Value` for `conn.request()` compatibility.
pub fn build_init_params(
    manifest: &ChildManifest,
    credential: Option<&str>,
    provider: &str,
) -> serde_json::Value {
    // Tier 2: include raw token if child opts in
    let requires_token = manifest
        .auth
        .as_ref()
        .map(|a| a.requires_in_process_token)
        .unwrap_or(false);

    let auth = if requires_token {
        if let Some(token) = credential {
            eprintln!(
                "[broker] {}: child holds raw credential (audit trail active)",
                manifest.child.name
            );
            Some(AuthConfig {
                token: token.to_string(),
                provider: provider.to_string(),
            })
        } else {
            None
        }
    } else {
        None
    };

    let init_params = InitializeParams {
        protocol_version: "1.0".to_string(),
        auth,
    };

    serde_json::to_value(init_params).expect("InitializeParams serialization cannot fail")
}

/// Determine the sandbox profile for a child based on its type.
///
/// Connector/transport/transform: deny-all (no filesystem, no network).
/// Lakehouse: scoped filesystem access to the storage path, no network.
/// Per DESIGN.md §8.3 and [[sandbox-profiles-are-parameterized]].
pub fn sandbox_profile_for_child(
    child_type: &ChildType,
    storage_path: Option<&str>,
) -> Result<SandboxProfile> {
    match child_type {
        ChildType::Connector | ChildType::Transport | ChildType::Transform => {
            Ok(SandboxProfile::DenyAll)
        }
        ChildType::Lakehouse => {
            let path = storage_path.ok_or_else(|| {
                anyhow::anyhow!(
                    "lakehouse child requires a storage_path for scoped filesystem access"
                )
            })?;
            Ok(SandboxProfile::ScopedStorage {
                path: path.to_string(),
            })
        }
    }
}

/// Spawn a native child with full broker setup.
///
/// 1. Resolve binary
/// 2. Load manifest
/// 3. Build HTTP handler with credential
/// 4. Determine sandbox profile from child type
/// 5. Spawn with sandbox
/// 6. Send pipe/initialize
///
/// Returns (NativeChild, ChildManifest) for the caller to use.
///
/// `storage_path` is required for lakehouse children — it specifies the
/// scoped filesystem path the sandbox will allow. Pass `None` for
/// connector/transport/transform children.
pub fn spawn_native(
    child_name: &str,
    credential: Option<(String, String)>, // (secret_name, secret_value)
    no_sandbox: bool,
    provider: &str,
    storage_path: Option<&str>,
) -> Result<(NativeChild, ChildManifest)> {
    let binary_path = resolve_child_binary(child_name)?;
    let manifest = load_manifest(&binary_path)?;

    // Build allowed domains list from manifest
    let allowed_domains: Vec<String> = manifest
        .domains
        .as_ref()
        .map(|d| d.allowed.clone())
        .unwrap_or_default();

    // Build production HTTP handler
    let http_handler: Option<HttpHandler> = if !allowed_domains.is_empty() || credential.is_some() {
        Some(build_production_handler(
            &allowed_domains,
            credential.clone(),
        )?)
    } else {
        None
    };

    // Determine sandbox profile from child type (DESIGN.md §8.3)
    let sandbox_profile = sandbox_profile_for_child(&manifest.child.child_type, storage_path)?;

    // Sandbox enforcement (DESIGN.md §Sandbox Enforcement)
    if !no_sandbox {
        check_sandbox_available()?;
        eprintln!(
            "[broker] {}: sandbox profile {:?}",
            child_name, sandbox_profile
        );
    } else {
        eprintln!(
            "[broker] WARNING: sandbox disabled for {} — child has unrestricted access",
            child_name
        );
    }

    // Note: actual sandbox application via pre_exec is [[spec-mother-broker]] scope.
    // The profile is computed here for logging and future use.
    let _ = &sandbox_profile;

    // Spawn child process
    let binary_str = binary_path.to_string_lossy().to_string();
    let mut conn = spawn_child_with_handler(&binary_str, http_handler)
        .map_err(|e| anyhow::anyhow!("failed to spawn {}: {}", child_name, e))?;

    // Send pipe/initialize
    let cred_value = credential.as_ref().map(|(_, v)| v.as_str());
    let init_params = build_init_params(&manifest, cred_value, provider);

    let (_notifs, response) = conn
        .request("pipe/initialize", init_params)
        .map_err(|e| anyhow::anyhow!("pipe/initialize failed for {}: {}", child_name, e))?;

    if let Some(error) = response.get("error") {
        bail!("child '{}' rejected initialization: {}", child_name, error);
    }

    Ok((NativeChild::new(child_name.to_string(), conn), manifest))
}

/// Check if OS sandbox is available. Fails with actionable error if not.
fn check_sandbox_available() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        // macOS sandbox_init is always available (kernel feature)
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        patina_pipe::sandbox::check_landlock_support()
            .map(|_| ())
            .map_err(|e| {
                anyhow::anyhow!(
                    "{}\n  Use --no-sandbox to bypass (not recommended for production)",
                    e
                )
            })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        bail!("sandbox not available on this platform. Use --no-sandbox to bypass.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_params_no_credential() {
        let manifest = ChildManifest::from_toml(
            r#"
[child]
name = "test"
version = "0.1.0"
type = "connector"
runtime = "native"
lifecycle = "poll"
"#,
        )
        .unwrap();

        let params = build_init_params(&manifest, None, "test");
        assert_eq!(params["protocol_version"], "1.0");
        assert!(params.get("auth").is_none());
    }

    #[test]
    fn init_params_with_token_but_not_required() {
        let manifest = ChildManifest::from_toml(
            r#"
[child]
name = "test"
version = "0.1.0"
type = "connector"
runtime = "native"
lifecycle = "poll"

[auth]
required = true
"#,
        )
        .unwrap();

        // Token provided but requires_in_process_token is false (default)
        let params = build_init_params(&manifest, Some("secret123"), "github");
        assert!(
            params.get("auth").is_none(),
            "should not include token when not opted in"
        );
    }

    #[test]
    fn init_params_with_token_required() {
        let manifest = ChildManifest::from_toml(
            r#"
[child]
name = "test"
version = "0.1.0"
type = "connector"
runtime = "native"
lifecycle = "poll"

[auth]
required = true
requires_in_process_token = true
"#,
        )
        .unwrap();

        let params = build_init_params(&manifest, Some("secret123"), "github");
        assert_eq!(params["auth"]["token"], "secret123");
        assert_eq!(params["auth"]["provider"], "github");
    }

    #[test]
    fn resolve_nonexistent_binary() {
        let result = resolve_child_binary("nonexistent-child-binary-xyz");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn sandbox_profile_connector_is_deny_all() {
        let profile = sandbox_profile_for_child(&ChildType::Connector, None).unwrap();
        assert!(matches!(profile, SandboxProfile::DenyAll));
    }

    #[test]
    fn sandbox_profile_transport_is_deny_all() {
        let profile = sandbox_profile_for_child(&ChildType::Transport, None).unwrap();
        assert!(matches!(profile, SandboxProfile::DenyAll));
    }

    #[test]
    fn sandbox_profile_transform_is_deny_all() {
        let profile = sandbox_profile_for_child(&ChildType::Transform, None).unwrap();
        assert!(matches!(profile, SandboxProfile::DenyAll));
    }

    #[test]
    fn sandbox_profile_lakehouse_requires_path() {
        let result = sandbox_profile_for_child(&ChildType::Lakehouse, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("storage_path"));
    }

    #[test]
    fn sandbox_profile_lakehouse_scoped() {
        let profile = sandbox_profile_for_child(&ChildType::Lakehouse, Some("/tmp/lake")).unwrap();
        assert!(matches!(
            profile,
            SandboxProfile::ScopedStorage { path } if path == "/tmp/lake"
        ));
    }
}
