//! DuckLake child — autonomous DuckDB + DuckLake data lake.
//!
//! First child with agency. Mother grants two typed toys (connector +
//! storage) via pipe/initialize, child drives everything: spawns
//! connector, fetches data, stores results, tracks cursors.
//! Per [[children-have-agency-toys-are-capabilities]].

use std::collections::HashMap;
use std::path::PathBuf;

use duckdb::Connection;
use patina_pipe::harness::ChildConnection;
use patina_pipe::http_proxy::{build_http_proxy, HttpProxyConfig, ProxyCredential, ProxyInjection};
use patina_pipe::{run, Capabilities, Child, InitializeParams, PipeError, RunResult, TypeReport};
use patina_pipe_types::config::{ConnectorToy, DuckLakeGrant, Escalation, GrantInjection};

struct DuckLakeChild {
    db: Option<Connection>,
    grant: Option<DuckLakeGrant>,
}

impl DuckLakeChild {
    fn new() -> Self {
        Self {
            db: None,
            grant: None,
        }
    }
}

impl Child for DuckLakeChild {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            provider: "ducklake".to_string(),
            data_types: vec![],
            supports_incremental: true,
        }
    }

    fn initialize(&mut self, params: &InitializeParams) -> Result<(), PipeError> {
        let grant = params.ducklake.as_ref().ok_or_else(|| PipeError::Fatal {
            message: "ducklake child requires a DuckLakeGrant in pipe/initialize".into(),
        })?;

        let lake_path = PathBuf::from(&grant.storage.lake_path);

        // Ensure lake directory exists
        if !lake_path.exists() {
            return Err(PipeError::Fatal {
                message: format!(
                    "lake path does not exist: {} — run: patina lake create <name>",
                    lake_path.display()
                ),
            });
        }

        let db = Connection::open_in_memory().map_err(|e| PipeError::Fatal {
            message: format!("failed to open DuckDB: {}", e),
        })?;

        // Install and load DuckLake extension
        db.execute_batch("INSTALL ducklake; LOAD ducklake;")
            .map_err(|e| PipeError::Fatal {
                message: format!("failed to install/load DuckLake extension: {}", e),
            })?;

        // Attach the DuckLake catalog
        let catalog = lake_path.join("lake.ducklake");
        let attach_sql = format!(
            "ATTACH 'ducklake:{}' AS lake (DATA_PATH '{}')",
            catalog.display(),
            lake_path.display()
        );
        db.execute_batch(&attach_sql)
            .map_err(|e| PipeError::Fatal {
                message: format!("failed to attach DuckLake catalog: {}", e),
            })?;

        // Create cursor tracking table
        // No PRIMARY KEY or DEFAULT — DuckLake only supports literal defaults
        // and no PK constraints. Upsert via DELETE + INSERT.
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS lake._sync_cursors (
                source_name VARCHAR,
                data_type   VARCHAR,
                cursor      VARCHAR,
                last_run    TIMESTAMP,
                records_written BIGINT,
                status      VARCHAR,
                last_error  VARCHAR
            )",
        )
        .map_err(|e| PipeError::Fatal {
            message: format!("failed to create _sync_cursors: {}", e),
        })?;

        self.db = Some(db);
        self.grant = Some(grant.clone());
        eprintln!(
            "[ducklake] initialized with lake at {}",
            lake_path.display()
        );
        Ok(())
    }

    fn run(&mut self) -> Result<RunResult, PipeError> {
        let grant = self.grant.as_ref().ok_or_else(|| PipeError::Fatal {
            message: "run() called before initialize".into(),
        })?;

        // Spawn connector via the boundary-preserving path
        let mut connector = use_connector_toy(&grant.connector).map_err(|e| {
            // Check if this is a connector-not-found error (escalate)
            let msg = e.to_string();
            if msg.contains("not found") {
                PipeError::Fatal {
                    message: format!("connector binary '{}' not found", grant.connector.binary),
                }
            } else {
                PipeError::Fatal {
                    message: format!("failed to spawn connector: {}", msg),
                }
            }
        })?;

        let mut types_report: HashMap<String, TypeReport> = HashMap::new();
        let mut escalation: Option<Escalation> = None;

        // Fetch each type independently
        for data_type in &grant.connector.types {
            let cursor = self.load_cursor(data_type);

            match self.fetch_and_store(&mut connector, data_type, cursor.as_deref()) {
                Ok(result) => {
                    self.advance_cursor(data_type, &result.cursor, result.written);
                    eprintln!(
                        "[ducklake] {}: {} written{}",
                        data_type,
                        result.written,
                        result
                            .cursor
                            .as_ref()
                            .map(|c| format!(", cursor: {}", c))
                            .unwrap_or_default()
                    );
                    types_report.insert(
                        data_type.clone(),
                        TypeReport {
                            status: "ok".to_string(),
                            written: result.written,
                            cursor: result.cursor,
                            error: None,
                        },
                    );
                }
                Err(ref e) if is_auth_error(e.as_ref()) => {
                    let msg = format!("{}", e);
                    eprintln!(
                        "[ducklake] {}: auth error — escalating to Mother",
                        data_type
                    );
                    escalation = Some(Escalation {
                        escalation_type: "auth_failed".to_string(),
                        message: msg,
                        action: Some(format!(
                            "patina connect refresh {}",
                            grant.connector.binary.replace("-connector", "")
                        )),
                    });
                    // Auth errors affect all types — stop fetching
                    break;
                }
                Err(e) => {
                    let msg = format!("{}", e);
                    eprintln!("[ducklake] {}: error — {}", data_type, msg);
                    self.record_error(data_type, &msg);
                    types_report.insert(
                        data_type.clone(),
                        TypeReport {
                            status: "failed".to_string(),
                            written: 0,
                            cursor: None,
                            error: Some(msg),
                        },
                    );
                }
            }
        }

        // Shutdown connector
        if let Err(e) = connector.request("pipe/shutdown", serde_json::json!({})) {
            eprintln!("[ducklake] connector shutdown warning: {}", e);
        }

        Ok(RunResult {
            types: types_report,
            escalation,
        })
    }
}

impl DuckLakeChild {
    fn load_cursor(&self, data_type: &str) -> Option<String> {
        let db = self.db.as_ref()?;
        let grant = self.grant.as_ref()?;
        db.query_row(
            "SELECT cursor FROM lake._sync_cursors WHERE source_name = ? AND data_type = ?",
            duckdb::params![grant.connector.binary, data_type],
            |row| row.get(0),
        )
        .ok()
    }

    fn advance_cursor(&self, data_type: &str, cursor: &Option<String>, written: u64) {
        let Some(db) = &self.db else { return };
        let Some(grant) = &self.grant else { return };
        let cursor_val = cursor.as_deref().unwrap_or("");

        // DELETE + INSERT (DuckLake doesn't support INSERT OR REPLACE)
        let _ = db.execute(
            "DELETE FROM lake._sync_cursors WHERE source_name = ? AND data_type = ?",
            duckdb::params![grant.connector.binary, data_type],
        );
        let _ = db.execute(
            "INSERT INTO lake._sync_cursors
             (source_name, data_type, cursor, last_run, records_written, status)
             VALUES (?, ?, ?, current_timestamp, ?, 'ok')",
            duckdb::params![
                grant.connector.binary,
                data_type,
                cursor_val,
                written as i64
            ],
        );
    }

    fn record_error(&self, data_type: &str, error: &str) {
        let Some(db) = &self.db else { return };
        let Some(grant) = &self.grant else { return };

        // Preserve existing cursor value
        let existing_cursor: String = db
            .query_row(
                "SELECT cursor FROM lake._sync_cursors WHERE source_name = ? AND data_type = ?",
                duckdb::params![grant.connector.binary, data_type],
                |row| row.get(0),
            )
            .unwrap_or_default();

        // DELETE + INSERT (DuckLake doesn't support INSERT OR REPLACE)
        let _ = db.execute(
            "DELETE FROM lake._sync_cursors WHERE source_name = ? AND data_type = ?",
            duckdb::params![grant.connector.binary, data_type],
        );
        let _ = db.execute(
            "INSERT INTO lake._sync_cursors
             (source_name, data_type, cursor, last_run, status, last_error)
             VALUES (?, ?, ?, current_timestamp, 'error', ?)",
            duckdb::params![grant.connector.binary, data_type, existing_cursor, error],
        );
    }

    fn fetch_and_store(
        &self,
        connector: &mut ChildConnection,
        data_type: &str,
        cursor: Option<&str>,
    ) -> Result<StoreResult, Box<dyn std::error::Error>> {
        let grant = self.grant.as_ref().unwrap();
        let db = self.db.as_ref().unwrap();

        // Send pipe/fetch for this type
        // cursor is Option<&str> — serialize as null when None (not empty string)
        let fetch_params = serde_json::json!({
            "types": [data_type],
            "since": cursor.filter(|s| !s.is_empty()),
            "limit": 10000,
            "params": grant.connector.params,
        });

        let (facts, response) = connector.request("pipe/fetch", fetch_params)?;

        // Check for error response
        if let Some(error) = response.get("error") {
            let msg = error["message"]
                .as_str()
                .unwrap_or("unknown error")
                .to_string();
            // Check for auth-related HTTP errors
            if msg.contains("401") || msg.contains("403") || msg.contains("auth") {
                return Err(format!("auth error from connector: {}", msg).into());
            }
            return Err(format!("connector error: {}", msg).into());
        }

        // Parse fetch result from response
        let fetch_cursor = response
            .get("result")
            .and_then(|r| r.get("cursor"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        // Auto-create table
        // No DEFAULT expressions — DuckLake only supports literal defaults.
        let table = event_type_to_table(data_type);
        db.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS lake.{} (
                _ingested_at TIMESTAMP,
                _source_id VARCHAR,
                _content_hash VARCHAR,
                data JSON
            )",
            table
        ))?;

        // Batch insert from fact notifications (supply timestamp explicitly)
        let mut written = 0u64;
        let mut stmt = db.prepare(&format!(
            "INSERT INTO lake.{} (_ingested_at, _source_id, _content_hash, data) VALUES (current_timestamp, ?, ?, ?)",
            table
        ))?;

        for fact in &facts {
            let params = fact.get("params").unwrap_or(fact);
            let content_hash = params
                .get("content_hash")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let data = params
                .get("data")
                .map(|v| v.to_string())
                .unwrap_or_else(|| "{}".to_string());

            stmt.execute(duckdb::params![grant.connector.binary, content_hash, data])?;
            written += 1;
        }

        Ok(StoreResult {
            written,
            cursor: fetch_cursor,
        })
    }
}

struct StoreResult {
    written: u64,
    cursor: Option<String>,
}

/// Derive table name from event type.
/// "issues" → "issues", "prs" → "prs", "github.issue" → "issues"
fn event_type_to_table(data_type: &str) -> String {
    // Strip provider prefix if present
    let base = data_type.rsplit('.').next().unwrap_or(data_type);

    // Pluralize if needed
    if base.ends_with('s') {
        base.to_string()
    } else {
        format!("{}s", base)
    }
}

/// Spawn the approved connector toy with derived HTTP enforcement.
///
/// The child derives HTTP proxy from the connector grant — policy
/// boundary + secret material → enforcement mechanism. The child
/// cannot have connector access without going through this step.
/// Per [[connector-toy-is-indivisible-authority]].
fn use_connector_toy(
    connector: &ConnectorToy,
) -> Result<ChildConnection, Box<dyn std::error::Error>> {
    // Resolve approved binary
    let path = resolve_child_binary(&connector.binary)?;

    // Derive HTTP enforcement from the connector grant
    let http_handler = build_http_proxy(HttpProxyConfig {
        allowed_domains: connector.allowed_domains.clone(),
        credential: connector.credential.as_ref().map(|cred| ProxyCredential {
            value: cred.clone(),
            injection: match &connector.injection {
                GrantInjection::Bearer => ProxyInjection::Bearer,
                GrantInjection::Header { name } => ProxyInjection::Header { name: name.clone() },
                GrantInjection::InProcess => ProxyInjection::InProcess,
            },
        }),
    })?;

    // Spawn connector with derived enforcement
    let mut conn = ChildConnection::spawn_with_http(&path, http_handler)?;

    // Initialize connector — InProcess credential delivery if applicable
    let auth = match &connector.injection {
        GrantInjection::InProcess => connector.credential.as_ref().map(|c| {
            serde_json::json!({
                "token": c, "provider": "oauth"
            })
        }),
        _ => None,
    };
    let init_params = serde_json::json!({
        "protocol_version": "1.0",
        "auth": auth,
    });

    let (_notifs, response) = conn.request("pipe/initialize", init_params)?;

    // Check for init error
    if let Some(error) = response.get("error") {
        return Err(format!(
            "connector initialization failed: {}",
            error["message"].as_str().unwrap_or("unknown error")
        )
        .into());
    }

    eprintln!(
        "[ducklake] connector '{}' spawned and initialized",
        connector.binary
    );
    Ok(conn)
}

/// Resolve a child binary by name.
///
/// Search order mirrors broker/spawn.rs:
/// 1. ~/.patina/children/{name}/{name}
/// 2. PATH
/// 3. ./target/release/{name}
fn resolve_child_binary(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    // 1. ~/.patina/children/{name}/{name}
    if let Some(home) = dirs::home_dir() {
        let installed = home.join(".patina").join("children").join(name).join(name);
        if installed.exists() {
            return Ok(installed.to_string_lossy().to_string());
        }
    }

    // 2. PATH
    if let Ok(path) = which::which(name) {
        return Ok(path.to_string_lossy().to_string());
    }

    // 3. ./target/release/{name}
    let dev_build = PathBuf::from("target").join("release").join(name);
    if dev_build.exists() {
        return Ok(dev_build.to_string_lossy().to_string());
    }

    Err(format!(
        "child binary '{}' not found. Searched:\n  \
         1. ~/.patina/children/{0}/{0}\n  \
         2. PATH\n  \
         3. ./target/release/{0}",
        name
    )
    .into())
}

/// Check if an error is auth-related (should escalate to Mother).
fn is_auth_error(e: &dyn std::error::Error) -> bool {
    let msg = e.to_string();
    msg.contains("auth error") || msg.contains("401") || msg.contains("403")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(DuckLakeChild::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_to_table_simple() {
        assert_eq!(event_type_to_table("issues"), "issues");
        assert_eq!(event_type_to_table("prs"), "prs");
    }

    #[test]
    fn event_type_to_table_with_provider_prefix() {
        assert_eq!(event_type_to_table("github.issue"), "issues");
        assert_eq!(event_type_to_table("github.pr"), "prs");
    }

    #[test]
    fn event_type_to_table_pluralization() {
        assert_eq!(event_type_to_table("issue"), "issues");
        assert_eq!(event_type_to_table("pr"), "prs");
        assert_eq!(event_type_to_table("status"), "status"); // already ends in 's'
    }

    #[test]
    fn resolve_nonexistent_binary() {
        let result = resolve_child_binary("nonexistent-child-binary-xyz");
        assert!(result.is_err());
    }

    #[test]
    fn ducklake_child_capabilities() {
        let child = DuckLakeChild::new();
        let caps = child.capabilities();
        assert_eq!(caps.provider, "ducklake");
        assert!(caps.supports_incremental);
    }

    #[test]
    fn ducklake_child_init_fails_without_grant() {
        let mut child = DuckLakeChild::new();
        let params = InitializeParams {
            protocol_version: "1.0".to_string(),
            auth: None,
            ducklake: None,
        };
        let result = child.initialize(&params);
        assert!(result.is_err());
        match result.unwrap_err() {
            PipeError::Fatal { message } => {
                assert!(message.contains("DuckLakeGrant"));
            }
            _ => panic!("expected Fatal error"),
        }
    }

    #[test]
    fn ducklake_child_init_fails_with_bad_path() {
        let mut child = DuckLakeChild::new();
        let params = InitializeParams {
            protocol_version: "1.0".to_string(),
            auth: None,
            ducklake: Some(DuckLakeGrant {
                connector: ConnectorToy {
                    binary: "github-connector".to_string(),
                    credential: None,
                    injection: GrantInjection::Bearer,
                    allowed_domains: vec![],
                    params: serde_json::json!({}),
                    types: vec![],
                },
                storage: patina_pipe_types::config::StorageToy {
                    lake_path: "/nonexistent/path/xyz".to_string(),
                },
            }),
        };
        let result = child.initialize(&params);
        assert!(result.is_err());
        match result.unwrap_err() {
            PipeError::Fatal { message } => {
                assert!(message.contains("does not exist"));
            }
            _ => panic!("expected Fatal error"),
        }
    }

    #[test]
    fn ducklake_child_run_fails_before_init() {
        let mut child = DuckLakeChild::new();
        let result = child.run();
        assert!(result.is_err());
        match result.unwrap_err() {
            PipeError::Fatal { message } => {
                assert!(message.contains("before initialize"));
            }
            _ => panic!("expected Fatal error"),
        }
    }

    #[test]
    fn grant_injection_bearer_to_proxy() {
        // Verify the mapping from GrantInjection to ProxyInjection
        let connector = ConnectorToy {
            binary: "test".to_string(),
            credential: Some("secret".to_string()),
            injection: GrantInjection::Bearer,
            allowed_domains: vec!["api.test.com".to_string()],
            params: serde_json::json!({}),
            types: vec![],
        };

        let config = HttpProxyConfig {
            allowed_domains: connector.allowed_domains.clone(),
            credential: connector.credential.as_ref().map(|cred| ProxyCredential {
                value: cred.clone(),
                injection: match &connector.injection {
                    GrantInjection::Bearer => ProxyInjection::Bearer,
                    GrantInjection::Header { name } => {
                        ProxyInjection::Header { name: name.clone() }
                    }
                    GrantInjection::InProcess => ProxyInjection::InProcess,
                },
            }),
        };

        // Build succeeds — validates the mapping works
        assert!(build_http_proxy(config).is_ok());
    }
}
