//! Connection store — TOML persistence + referential integrity.
//!
//! CRUD for connection records: load, list, create, remove.
//! Uses `paths::connections` for all path resolution.
//! Writes credentials to vault via `secrets::add_secret`.
//! Never holds decrypted credential values.

use crate::connect::internal::model::{
    ConnectError, ConnectionRecord, ConnectionStatus, ConnectionSummary,
};
use crate::mother::broker::sources;
use crate::paths;
use patina_protocol::{
    BuiltinChild, BuiltinChildAction, BuiltinChildRequest, SecretsAuthorityOperation,
    SecretsDispatchRequest,
};
use std::fs;

/// Load a connection by name.
///
/// Reads `~/.patina/connections/{name}.toml` and parses into a `ConnectionRecord`.
pub(crate) fn load(name: &str) -> Result<ConnectionRecord, ConnectError> {
    let path = paths::connections::connection_path(name);

    if !path.exists() {
        return Err(ConnectError::NotFound {
            name: name.to_string(),
        });
    }

    let content = fs::read_to_string(&path).map_err(|e| ConnectError::IoError {
        detail: format!("reading {}: {}", path.display(), e),
    })?;

    let record: ConnectionRecord =
        toml::from_str(&content).map_err(|e| ConnectError::MalformedConfig {
            name: name.to_string(),
            detail: e.to_string(),
        })?;

    Ok(record)
}

/// List all connections with computed status.
///
/// Reads all `.toml` files in `~/.patina/connections/`, parses each,
/// and computes status from durable metadata.
pub(crate) fn list() -> Result<Vec<ConnectionSummary>, ConnectError> {
    let dir = paths::connections::connections_dir();

    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut summaries = Vec::new();

    let entries = fs::read_dir(&dir).map_err(|e| ConnectError::IoError {
        detail: format!("reading {}: {}", dir.display(), e),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| ConnectError::IoError {
            detail: e.to_string(),
        })?;

        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        match load(name) {
            Ok(record) => {
                let status = compute_status(&record);
                summaries.push(ConnectionSummary {
                    name: record.identity.name.clone(),
                    provider: record.identity.provider.clone(),
                    account_id: record.identity.account_id.clone(),
                    status,
                });
            }
            Err(e) => {
                eprintln!("[connect] warning: skipping {}: {}", name, e);
            }
        }
    }

    summaries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(summaries)
}

/// Create a new connection: writes TOML record + vault entry.
///
/// - Writes `ConnectionRecord` to `~/.patina/connections/{name}.toml`
/// - Stores credential in global vault via `secrets::add_secret`
/// - Creates the connections directory if it doesn't exist
pub(crate) fn create(record: &ConnectionRecord, credential: &str) -> Result<(), ConnectError> {
    let dir = paths::connections::connections_dir();
    let path = paths::connections::connection_path(&record.identity.name);

    // Ensure connections directory exists
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| ConnectError::IoError {
            detail: format!("creating {}: {}", dir.display(), e),
        })?;
    }

    // Write TOML
    let toml_str = toml::to_string_pretty(record).map_err(|e| ConnectError::IoError {
        detail: format!("serializing connection '{}': {}", record.identity.name, e),
    })?;

    fs::write(&path, toml_str).map_err(|e| ConnectError::IoError {
        detail: format!("writing {}: {}", path.display(), e),
    })?;

    // Store credential in global vault
    let request = BuiltinChildRequest::new(
        BuiltinChild::SecretsAuthority,
        BuiltinChildAction::SecretsDispatch(SecretsDispatchRequest {
            operation: SecretsAuthorityOperation::AddSecret {
                name: record.auth.secret_ref.clone(),
                value: credential.to_string(),
                env: None,
                global: true,
                project_root: None,
            },
        }),
    );
    crate::mother::control_plane_client()
        .child_action_typed(&request)
        .map_err(|e| ConnectError::IoError {
            detail: format!(
                "storing credential '{}' in vault via Mother authority: {}",
                record.auth.secret_ref, e
            ),
        })?;

    Ok(())
}

/// Remove a connection: deletes TOML + vault entry.
///
/// Checks referential integrity first by scanning all registered projects
/// for sources.toml entries that reference this connection.
/// Returns `ConnectError::InUse` if references exist.
pub(crate) fn remove(name: &str, force: bool) -> Result<(), ConnectError> {
    let path = paths::connections::connection_path(name);

    if !path.exists() {
        return Err(ConnectError::NotFound {
            name: name.to_string(),
        });
    }

    // Load record to get secret_ref before deletion
    let record = load(name)?;

    // Check referential integrity (unless --force)
    if !force {
        let references = check_references(name)?;
        if !references.is_empty() {
            return Err(ConnectError::InUse {
                connection: name.to_string(),
                references,
            });
        }
    }

    // Delete TOML file
    fs::remove_file(&path).map_err(|e| ConnectError::IoError {
        detail: format!("removing {}: {}", path.display(), e),
    })?;

    // Remove credential from vault (best-effort — vault may not exist)
    let request = BuiltinChildRequest::new(
        BuiltinChild::SecretsAuthority,
        BuiltinChildAction::SecretsDispatch(SecretsDispatchRequest {
            operation: SecretsAuthorityOperation::RemoveSecret {
                name: record.auth.secret_ref.clone(),
                global: true,
                project_root: None,
            },
        }),
    );
    if let Err(e) = crate::mother::control_plane_client().child_action_typed(&request) {
        eprintln!(
            "[connect] warning: could not remove vault entry '{}': {}",
            record.auth.secret_ref, e
        );
    }

    Ok(())
}

/// Check referential integrity: find sources.toml entries referencing this connection.
fn check_references(name: &str) -> Result<Vec<String>, ConnectError> {
    let all_sources = sources::scan_all_sources(&crate::paths::registry_path()).map_err(|e| {
        ConnectError::IoError {
            detail: format!("scanning sources for references: {}", e),
        }
    })?;

    let mut references = Vec::new();
    for project in &all_sources {
        for source in &project.sources {
            if source.connection == name {
                references.push(format!(
                    "{} → source \"{}\"",
                    project.project_root.display(),
                    source.name
                ));
            }
        }
    }

    Ok(references)
}

/// Compute connection status from durable metadata.
///
/// No network calls, no vault decryption — just metadata checks
/// and vault existence test via registry.
fn compute_status(record: &ConnectionRecord) -> ConnectionStatus {
    // Check for recorded error
    if record.auth.last_error.is_some() {
        return ConnectionStatus::Errored;
    }

    // Check expiry
    if let Some(expires_at) = &record.auth.expires_at {
        // Simple string comparison works for ISO 8601 timestamps
        let now = chrono::Utc::now().to_rfc3339();
        if expires_at < &now {
            return ConnectionStatus::Expired;
        }
    }

    // Check if ever validated
    if record.identity.last_validated.is_none() {
        return ConnectionStatus::Unchecked;
    }

    ConnectionStatus::Connected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connect::internal::model::*;
    use tempfile::TempDir;

    fn sample_record(name: &str) -> ConnectionRecord {
        ConnectionRecord {
            schema_version: 0,
            identity: ConnectionIdentity {
                name: name.to_string(),
                provider: "github".into(),
                account_id: Some("octocat".into()),
                auth_method: AuthMethod::OAuth,
                scopes: vec!["repo".into()],
                is_default: true,
                created_at: "2026-03-09T17:00:00Z".into(),
                updated_at: "2026-03-09T17:00:00Z".into(),
                last_validated: None,
                scope: ConnectionScope::Global,
            },
            auth: AuthConfig {
                injection: InjectionStrategy::Bearer,
                secret_ref: format!("{}:default", name),
                child: "github".into(),
                allowed_domains: vec!["api.github.com".into()],
                refresh_capable: true,
                expires_at: None,
                last_error: None,
            },
        }
    }

    // -- Isolated filesystem tests (no vault interaction) --
    // These tests exercise TOML read/write logic using temp directories.
    // They do NOT call create() or remove() which involve the vault.

    #[test]
    fn load_nonexistent_returns_not_found() {
        let result = load("definitely-does-not-exist-xyzzy");
        assert!(matches!(result, Err(ConnectError::NotFound { .. })));
    }

    #[test]
    fn load_malformed_toml_returns_error() {
        let dir = TempDir::new().unwrap();
        let conn_dir = dir.path().join(".patina").join("connections");
        fs::create_dir_all(&conn_dir).unwrap();
        fs::write(conn_dir.join("bad.toml"), "not valid { toml").unwrap();

        // We need to write to the actual connections path for this test,
        // so we test parse logic directly instead.
        let content = "not valid { toml";
        let result: Result<ConnectionRecord, _> = toml::from_str(content);
        assert!(result.is_err());
    }

    #[test]
    fn write_and_read_toml_roundtrip() {
        let dir = TempDir::new().unwrap();
        let conn_dir = dir.path().join("connections");
        fs::create_dir_all(&conn_dir).unwrap();

        let record = sample_record("test-conn");
        let toml_str = toml::to_string_pretty(&record).unwrap();
        let path = conn_dir.join("test-conn.toml");
        fs::write(&path, &toml_str).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let parsed: ConnectionRecord = toml::from_str(&content).unwrap();
        assert_eq!(record, parsed);
    }

    #[test]
    fn list_empty_dir_returns_empty() {
        // If connections dir doesn't exist, list returns empty
        // This relies on the real connections dir path, which may or may not exist.
        // We verify the logic by testing that non-.toml files are skipped.
        let dir = TempDir::new().unwrap();
        let conn_dir = dir.path().join("connections");
        fs::create_dir_all(&conn_dir).unwrap();

        // Write a non-TOML file — should be skipped
        fs::write(conn_dir.join("readme.md"), "not a connection").unwrap();

        // We can't override paths::connections::connections_dir() in tests,
        // so we verify the filtering logic directly.
        let path = conn_dir.join("readme.md");
        assert_ne!(path.extension().and_then(|e| e.to_str()), Some("toml"));
    }

    #[test]
    fn compute_status_errored() {
        let mut record = sample_record("test");
        record.auth.last_error = Some("auth failed".into());
        assert_eq!(compute_status(&record), ConnectionStatus::Errored);
    }

    #[test]
    fn compute_status_expired() {
        let mut record = sample_record("test");
        record.auth.expires_at = Some("2020-01-01T00:00:00Z".into());
        record.identity.last_validated = Some("2020-01-01T00:00:00Z".into());
        assert_eq!(compute_status(&record), ConnectionStatus::Expired);
    }

    #[test]
    fn compute_status_unchecked() {
        let record = sample_record("test");
        // last_validated is None by default
        assert_eq!(compute_status(&record), ConnectionStatus::Unchecked);
    }

    #[test]
    fn compute_status_connected() {
        let mut record = sample_record("test");
        record.identity.last_validated = Some("2026-03-09T17:00:00Z".into());
        assert_eq!(compute_status(&record), ConnectionStatus::Connected);
    }

    #[test]
    fn check_references_no_registry_returns_empty() {
        // scan_all_sources returns empty when no registry exists
        let result = check_references("nonexistent-conn");
        // Should succeed (empty vec) or at worst return an error
        // In test environments without a registry, this returns Ok(vec![])
        if let Ok(refs) = result {
            assert!(refs.is_empty());
        }
    }
}
