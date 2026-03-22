//! Broker — routing engine for Mother.
//!
//! Routes facts from children to destination events.db files based on
//! sources.toml declarations. Manages child lifecycle (spawn, fetch,
//! shutdown) for native children via the pipe protocol.

pub mod cursor;
pub mod sources;

use anyhow::{Context, Result};
use std::path::Path;
use std::time::{Duration, Instant};

use self::cursor::get_cursor;
use self::sources::{Destination, SourceEntry};

/// Result of writing a batch of facts.
#[derive(Debug, Clone)]
pub struct WriteResult {
    pub inserted: u64,
    pub dedup_skipped: u64,
    pub cursor: Option<String>,
}

/// Run a single source: resolve auth, spawn child, fetch facts, validate, route to destination.
///
/// This is the full flow from DESIGN.md — the broker's primary operation.
/// Routes to project events.db or lake based on source.destination.
/// Branch happens BEFORE spawn — lake destinations spawn the ducklake
/// child (not the connector). Per [[children-have-agency-toys-are-capabilities]].
///
/// FAIL CLOSED: auth resolution errors propagate — the child is never
/// spawned without a valid AuthPlan.
pub fn run_source(
    source: &SourceEntry,
    project_root: &Path,
    no_sandbox: bool,
) -> Result<WriteResult> {
    if matches!(source.destination, Destination::Project) {
        anyhow::bail!(
            "source '{}' uses destination=project, which is retired. Set [sources.{}.destination] type = 'lake' and lake = '<name>'",
            source.name,
            source.name
        );
    }

    // 1. Load connection record from connect module
    let record = crate::connect::load(&source.connection)
        .map_err(|e| anyhow::anyhow!("{}", e))
        .with_context(|| format!("loading connection for source '{}'", source.name))?;

    // 2. Resolve auth plan — FAIL CLOSED (no "proceeding without auth")
    let auth_plan = crate::connect::resolve_auth(&record)
        .map_err(|e| anyhow::anyhow!("{}", e))
        .with_context(|| format!("resolving auth for source '{}'", source.name))?;

    // 3. Branch BEFORE spawn based on destination
    match &source.destination {
        Destination::Project => unreachable!("project destination is rejected above"),
        Destination::Lake { name } => {
            route_lake_via_knowledge_child(source, project_root, name, &auth_plan, no_sandbox)
        }
    }
}

/// Grant lake capabilities — spawn ducklake child, grant toy approvals, call pipe/run.
///
/// Mother's involvement ends after the capability grant. The child uses its
/// approved toys and reports back. Per [[children-have-agency-toys-are-capabilities]]
/// and [[initialize-is-capability-grant]].
fn route_lake_via_knowledge_child(
    source: &SourceEntry,
    project_root: &Path,
    lake_name: &str,
    auth_plan: &crate::connect::AuthPlan,
    no_sandbox: bool,
) -> Result<WriteResult> {
    const REQUIRED_TYPES: [&str; 2] = ["issues", "prs"];

    crate::connect::require_auth_plan_domain(&source.connection, auth_plan, "api.github.com")
        .map_err(|e| {
            anyhow::anyhow!(
                "source '{}' connection is invalid for GitHub lake route: {}",
                source.name,
                e
            )
        })?;

    let runtime = crate::mother::KnowledgeRuntimeStore::default();
    let plugin_name = "patina-ducklake";

    let owner = source
        .params
        .get("owner")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("source '{}' is missing params.owner", source.name))?;
    let repo = source
        .params
        .get("repo")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("source '{}' is missing params.repo", source.name))?;
    let include_pr_commits = source
        .params
        .get("include_pr_commits")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let contract_types = REQUIRED_TYPES
        .iter()
        .map(|s| (*s).to_string())
        .collect::<Vec<_>>();
    let scope_contract = serde_json::json!({
        "version": 1,
        "owner": owner,
        "repo": repo,
        "issues": {
            "list": true,
            "comments": true,
            "events": true,
        },
        "pulls": {
            "list": true,
            "comments": true,
            "reviews": true,
            "review_comments": true,
            "commits": include_pr_commits,
        },
        "incremental": {
            "watermark_key": "updated_at",
            "tie_breaker": "id",
        },
    });
    runtime.put_state(
        plugin_name,
        &format!("connector:scope-contract:{}", source.name),
        &scope_contract.to_string(),
    )?;

    let binding_id = source.name.clone();
    let binding_json = serde_json::json!({
        "binding_id": binding_id,
        "connection": source.connection,
        "owner": owner,
        "repo": repo,
        "types": contract_types,
        "scope_version": 1,
        "include_pr_commits": include_pr_commits,
    })
    .to_string();
    runtime.put_state(
        plugin_name,
        &format!("connector:binding:{}", source.name),
        &binding_json,
    )?;

    let table_prefix = format!("{}_{}", owner, repo).replace('-', "_");
    let source_json = serde_json::json!({
        "source_id": source.name,
        "table_prefix": table_prefix,
        "binding_id": source.name,
        "data_types": REQUIRED_TYPES,
        "scope_contract_key": format!("connector:scope-contract:{}", source.name),
    })
    .to_string();
    runtime.put_state(
        plugin_name,
        &format!("source:{}", source.name),
        &source_json,
    )?;

    migrate_legacy_cursor(
        project_root,
        &runtime,
        lake_name,
        &source.name,
        &REQUIRED_TYPES
            .iter()
            .map(|s| (*s).to_string())
            .collect::<Vec<_>>(),
    )?;

    let intent = crate::mother::TaskIntent {
        kind: crate::mother::TaskIntentKind::FetchSource,
        payload: serde_json::json!({"source_id": source.name}),
        dedupe_key: Some(format!("ducklake:{}", source.name)),
    };
    let task_id = runtime.enqueue_task(plugin_name, &intent)?;

    let mut child = load_ducklake_knowledge_child(no_sandbox)?;
    struct BrokerHost;
    impl crate::mother::MotherHost for BrokerHost {
        fn log(&self, child: &str, message: &str) {
            eprintln!("[broker:{}] {}", child, message);
        }
    }
    child.on_load(&BrokerHost)?;

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut written_total = None;

    while Instant::now() < deadline {
        if let Some(task) = runtime.lease_next_task(plugin_name, "broker-lake")? {
            runtime.mark_task_running(&task.id)?;
            let request = crate::mother::ChildRequest {
                action: task.kind.as_str().to_string(),
                payload: serde_json::from_str(&task.payload_json)
                    .unwrap_or(serde_json::Value::Null),
            };
            match child.handle(&request) {
                Ok(response) => {
                    runtime.mark_task_succeeded(&task.id)?;
                    written_total = response.payload.get("written").and_then(|v| v.as_u64());
                }
                Err(error) => {
                    runtime.mark_task_failed(&task.id, task.attempts, &error.to_string())?;
                    anyhow::bail!(
                        "ducklake knowledge-child task failed for source '{}' (task {}): {}",
                        source.name,
                        task.id,
                        error
                    );
                }
            }
        }

        if let Some(checkpoint) = runtime.load_checkpoint(plugin_name, "ducklake.sync")? {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&checkpoint) {
                if value.get("source_id").and_then(|v| v.as_str()) == Some(source.name.as_str()) {
                    let inserted = written_total
                        .or_else(|| value.get("written").and_then(|v| v.as_u64()))
                        .unwrap_or(0);
                    return Ok(WriteResult {
                        inserted,
                        dedup_skipped: 0,
                        cursor: None,
                    });
                }
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    anyhow::bail!(
        "ducklake knowledge-child timed out waiting for '{}' (task {})",
        source.name,
        task_id
    )
}

fn migrate_legacy_cursor(
    project_root: &Path,
    runtime: &crate::mother::KnowledgeRuntimeStore,
    lake_name: &str,
    source_name: &str,
    data_types: &[String],
) -> Result<()> {
    let conn = crate::eventlog::open_events_db_at(project_root)
        .with_context(|| format!("opening events.db for {}", project_root.display()))?;
    let legacy_cursor = cursor::get_cursor(&conn, source_name).unwrap_or(None);
    let Some(cursor) = legacy_cursor else {
        return Ok(());
    };

    for data_type in data_types {
        if runtime
            .load_lake_cursor(lake_name, source_name, data_type)?
            .is_none()
        {
            runtime.save_lake_cursor(&crate::mother::state::LakeCursorUpdate {
                lake_name,
                source_name,
                data_type,
                cursor_value: Some(&cursor),
                records_written: 0,
                status: "migrated",
                last_error: None,
            })?;
        }
    }
    Ok(())
}

fn load_ducklake_knowledge_child(
    no_sandbox: bool,
) -> Result<Box<dyn crate::mother::KnowledgeChild>> {
    let engine = crate::child::engine::KnowledgeChildEngine::new()?;

    let mut candidates: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();
    let installed_dir = crate::paths::child::children_dir();
    if installed_dir.exists() {
        for entry in std::fs::read_dir(&installed_dir).with_context(|| {
            format!("reading installed children dir {}", installed_dir.display())
        })? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let wasm = path.with_extension("wasm");
                if wasm.exists() {
                    candidates.push((wasm, path));
                }
            }
        }
    }
    if let Some(manifest_path) = crate::child::engine::ChildManifest::resolve_child_manifest_path(
        std::path::Path::new("children/ducklake"),
    ) {
        candidates.push((
            std::path::PathBuf::from("target/wasm32-wasip2/release/patina_plugin_ducklake.wasm"),
            manifest_path,
        ));
    }

    for (wasm_path, manifest_path) in candidates {
        if !wasm_path.exists() || !manifest_path.exists() {
            continue;
        }
        let manifest = match crate::child::engine::ChildManifest::from_path(&manifest_path) {
            Ok(manifest) => manifest,
            Err(error) => {
                eprintln!(
                    "[broker] skipping unreadable child manifest {}: {}",
                    manifest_path.display(),
                    error
                );
                continue;
            }
        };
        if manifest.provides.child.as_deref() != Some("ducklake") {
            continue;
        }
        if !no_sandbox {
            eprintln!(
                "[broker] using knowledge-child ducklake component {}",
                wasm_path.display()
            );
        }
        let wasm = std::fs::read(&wasm_path)
            .with_context(|| format!("reading {}", wasm_path.display()))?;
        let component = engine.load_component(&wasm)?;
        return engine.instantiate_child(&component, &manifest, None);
    }

    anyhow::bail!(
        "ducklake knowledge-child component not found. Install one under {} or build children/ducklake for wasm32-wasip2",
        crate::paths::child::children_dir().display()
    )
}

/// Source status information for display.
#[derive(Debug)]
pub struct SourceStatus {
    pub name: String,
    pub last_run: Option<String>,
    pub fact_count: i64,
    pub status: String,
}

/// Get status for all sources in a project.
pub fn status(project_root: &Path) -> Result<Vec<SourceStatus>> {
    let project_sources = sources::load_project_sources(project_root)?;
    let sources_list = match project_sources {
        Some(ps) => ps.sources,
        None => return Ok(vec![]),
    };

    let events_conn = crate::eventlog::open_events_db_at(project_root)?;
    let mut statuses = Vec::new();

    for source in &sources_list {
        let cursor = get_cursor(&events_conn, &source.name)?;

        // Get last run timestamp from cursor's updated_at
        let last_run: Option<String> = events_conn
            .query_row(
                "SELECT updated_at FROM broker_cursors WHERE source_name = ?1",
                [&source.name],
                |row| row.get(0),
            )
            .ok();

        // Count facts from this source
        let source_id = format!("child:{}", source.name);

        let fact_count: i64 = events_conn
            .query_row(
                "SELECT COUNT(*) FROM eventlog WHERE source_id = ?1",
                [&source_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        statuses.push(SourceStatus {
            name: source.name.clone(),
            last_run,
            fact_count,
            status: if cursor.is_some() {
                "ok".to_string()
            } else {
                "never run".to_string()
            },
        });
    }

    Ok(statuses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::connect::{AuthPlan, InjectionStrategy, ResolvedCredential};
    use crate::mother::broker::sources::{Destination, SourceEntry};

    fn temp_runtime() -> crate::mother::KnowledgeRuntimeStore {
        let path =
            std::env::temp_dir().join(format!("patina-broker-runtime-{}.db", uuid::Uuid::new_v4()));
        crate::mother::KnowledgeRuntimeStore::new(path)
    }

    #[test]
    fn migration_copies_legacy_cursor_into_per_type_lake_cursors() {
        let project = tempfile::tempdir().unwrap();
        let conn = crate::eventlog::open_events_db_at(project.path()).unwrap();
        conn.execute(
            "INSERT INTO broker_cursors (source_name, cursor_value, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params!["gh-main", "2026-03-12T00:00:00Z", chrono::Utc::now().to_rfc3339()],
        )
        .unwrap();

        let runtime = temp_runtime();
        migrate_legacy_cursor(
            project.path(),
            &runtime,
            "default",
            "gh-main",
            &["issues".into(), "prs".into()],
        )
        .unwrap();

        assert_eq!(
            runtime
                .load_lake_cursor("default", "gh-main", "issues")
                .unwrap()
                .as_deref(),
            Some("2026-03-12T00:00:00Z")
        );
        assert_eq!(
            runtime
                .load_lake_cursor("default", "gh-main", "prs")
                .unwrap()
                .as_deref(),
            Some("2026-03-12T00:00:00Z")
        );
    }

    #[test]
    fn migration_is_idempotent_and_does_not_overwrite_existing_cursor() {
        let project = tempfile::tempdir().unwrap();
        let conn = crate::eventlog::open_events_db_at(project.path()).unwrap();
        conn.execute(
            "INSERT INTO broker_cursors (source_name, cursor_value, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params!["gh-main", "cursor-v1", chrono::Utc::now().to_rfc3339()],
        )
        .unwrap();

        let runtime = temp_runtime();
        runtime
            .save_lake_cursor(&crate::mother::state::LakeCursorUpdate {
                lake_name: "default",
                source_name: "gh-main",
                data_type: "issues",
                cursor_value: Some("already-set"),
                records_written: 0,
                status: "ok",
                last_error: None,
            })
            .unwrap();

        migrate_legacy_cursor(
            project.path(),
            &runtime,
            "default",
            "gh-main",
            &["issues".into(), "prs".into()],
        )
        .unwrap();

        assert_eq!(
            runtime
                .load_lake_cursor("default", "gh-main", "issues")
                .unwrap()
                .as_deref(),
            Some("already-set")
        );
        assert_eq!(
            runtime
                .load_lake_cursor("default", "gh-main", "prs")
                .unwrap()
                .as_deref(),
            Some("cursor-v1")
        );
    }

    #[test]
    fn lake_route_fails_closed_without_oauth_credential() {
        let project = tempfile::tempdir().unwrap();
        let source = SourceEntry {
            name: "gh-main".to_string(),
            connection: "github".to_string(),
            params: HashMap::from([
                (
                    "owner".to_string(),
                    toml::Value::String("NicabarNimble".into()),
                ),
                ("repo".to_string(), toml::Value::String("patina".into())),
            ]),
            types: vec!["issues".into(), "prs".into()],
            schedule: "manual".to_string(),
            destination: Destination::Lake {
                name: "default".to_string(),
            },
        };
        let auth_plan = AuthPlan {
            credential: None,
            allowed_domains: vec!["api.github.com".to_string()],
        };

        let error =
            route_lake_via_knowledge_child(&source, project.path(), "default", &auth_plan, true)
                .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("connection 'github' has no credential"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn lake_route_fails_closed_without_github_domain_scope() {
        let project = tempfile::tempdir().unwrap();
        let source = SourceEntry {
            name: "gh-main".to_string(),
            connection: "github".to_string(),
            params: HashMap::from([
                (
                    "owner".to_string(),
                    toml::Value::String("NicabarNimble".into()),
                ),
                ("repo".to_string(), toml::Value::String("patina".into())),
            ]),
            types: vec!["issues".into(), "prs".into()],
            schedule: "manual".to_string(),
            destination: Destination::Lake {
                name: "default".to_string(),
            },
        };
        let auth_plan = AuthPlan {
            credential: Some(ResolvedCredential {
                value: "token".to_string(),
                injection: InjectionStrategy::Bearer,
            }),
            allowed_domains: vec!["example.com".to_string()],
        };

        let error =
            route_lake_via_knowledge_child(&source, project.path(), "default", &auth_plan, true)
                .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("connection 'github' is not allowed for api.github.com"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn run_source_rejects_project_destination_path() {
        let project = tempfile::tempdir().unwrap();
        let source = SourceEntry {
            name: "legacy-project-path".to_string(),
            connection: "github".to_string(),
            params: HashMap::new(),
            types: vec!["issues".into()],
            schedule: "manual".to_string(),
            destination: Destination::Project,
        };

        let error = run_source(&source, project.path(), true).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("destination=project, which is retired"),
            "unexpected error: {error}"
        );
    }
}
