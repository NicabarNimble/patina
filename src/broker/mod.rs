//! Broker — routing engine for Mother.
//!
//! Routes facts from children to destination events.db files based on
//! sources.toml declarations. Manages child lifecycle (spawn, fetch,
//! shutdown) for native children via the pipe protocol.

pub mod cursor;
pub mod http;
pub mod lifecycle;
pub mod routing;
pub mod sources;
pub mod spawn;

use anyhow::{Context, Result};
use patina_pipe_types::config::{
    ConnectorToy, DuckLakeGrant, GrantInjection, InitializeParams, StorageToy,
};
use patina_pipe_types::manifest::ChildManifest;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use self::cursor::{get_cursor, write_facts_with_cursor};
use self::lifecycle::{BrokerChild, NativeChild, DEFAULT_MAX_BATCH_SIZE};
use self::routing::{validate_fact, ValidatedFact, WriteResult};
use self::sources::{Destination, SourceEntry};
use self::spawn::spawn_native_with_plan;

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
        Destination::Project => {
            // Current path: Mother spawns connector, drives fetch
            let (mut child, manifest) = spawn_native_with_plan(
                &auth_plan,
                no_sandbox,
                &record.identity.provider,
                None,
            )
            .with_context(|| format!("spawning child for source '{}'", source.name))?;

            write_to_project(source, project_root, &mut child, &manifest)
        }
        Destination::Lake { name } => {
            // New path: Mother spawns lakehouse child, grants toy approvals
            grant_lake_capabilities(source, name, &auth_plan, no_sandbox)
        }
    }
}

/// Write facts to the project's events.db (current default behavior).
///
/// Opens events.db, reads stored cursor, fetches from child, validates,
/// writes facts + cursor transactionally, shuts down child.
fn write_to_project(
    source: &SourceEntry,
    project_root: &Path,
    child: &mut NativeChild,
    manifest: &ChildManifest,
) -> Result<WriteResult> {
    // 4. Open destination events.db
    let events_conn = crate::eventlog::open_events_db_at(project_root)
        .with_context(|| format!("opening events.db for {}", project_root.display()))?;

    // 5. Get stored cursor
    let stored_cursor = get_cursor(&events_conn, &source.name)?;

    // 6. Build fetch params — shared type with boundary conversions
    let fetch_params = patina_pipe_types::FetchParams {
        types: source.types.clone(),
        since: stored_cursor,
        limit: DEFAULT_MAX_BATCH_SIZE as u64,
        params: serde_json::to_value(&source.params)
            .with_context(|| "serializing source params to JSON")?,
    };

    // 7. Fetch facts from child
    let mut validated_facts: Vec<ValidatedFact> = Vec::new();
    let mut schema_cache: HashMap<String, HashSet<String>> = HashMap::new();
    let child_name = child.name().to_string();

    let fetch_result = child.fetch(&fetch_params, &mut |fact| {
        match validate_fact(
            &fact,
            manifest,
            &child_name,
            project_root,
            &mut schema_cache,
        ) {
            Ok(validated) => {
                validated_facts.push(validated);
                Ok(())
            }
            Err(e) => {
                eprintln!("[broker] {}: {}", child_name, e);
                // Validation errors are logged and the fact is dropped,
                // but we don't abort the entire fetch
                Ok(())
            }
        }
    })?;

    // 8. Shutdown child
    if let Err(e) = child.shutdown() {
        eprintln!("[broker] {}: shutdown warning: {}", source.name, e);
    }

    // 9. Write facts + cursor transactionally
    let write_result = write_facts_with_cursor(
        &events_conn,
        &source.name,
        &validated_facts,
        fetch_result.cursor.as_deref(),
    )?;

    // 10. Report
    eprintln!(
        "[broker] {}: {} written, {} dedup{}",
        source.name,
        write_result.inserted,
        write_result.dedup_skipped,
        write_result
            .cursor
            .as_ref()
            .map(|c| format!(", cursor: {}", c))
            .unwrap_or_default()
    );

    Ok(write_result)
}

/// Grant lake capabilities — spawn ducklake child, grant toy approvals, call pipe/run.
///
/// Mother's involvement ends after the capability grant. The child uses its
/// approved toys and reports back. Per [[children-have-agency-toys-are-capabilities]]
/// and [[initialize-is-capability-grant]].
fn grant_lake_capabilities(
    source: &SourceEntry,
    lake_name: &str,
    auth_plan: &crate::connect::AuthPlan,
    no_sandbox: bool,
) -> Result<WriteResult> {
    use crate::connect::InjectionStrategy;
    use patina_pipe::harness::spawn_child;

    // 1. Resolve lake path (storage toy)
    let lake_path = crate::paths::lakes::resolve_lake_path(lake_name)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // 2. Spawn DuckLake child (the child, not the connector)
    let lake_binary = spawn::resolve_child_binary("ducklake")?;
    let mut lake_child = spawn_child(&lake_binary.to_string_lossy())
        .map_err(|e| anyhow::anyhow!("failed to spawn ducklake: {}", e))?;

    if !no_sandbox {
        eprintln!("[broker] ducklake: sandbox profile ScopedStorage");
    }

    // 3. Build typed capability grant — no untyped blobs at trust boundaries
    let grant = DuckLakeGrant {
        connector: ConnectorToy {
            binary: auth_plan.child.clone(),
            credential: auth_plan.credential.as_ref().map(|c| c.value.clone()),
            injection: auth_plan
                .credential
                .as_ref()
                .map(|c| match &c.injection {
                    InjectionStrategy::Bearer => GrantInjection::Bearer,
                    InjectionStrategy::Header { name } => {
                        GrantInjection::Header { name: name.clone() }
                    }
                    InjectionStrategy::InProcess => GrantInjection::InProcess,
                })
                .unwrap_or(GrantInjection::Bearer),
            allowed_domains: auth_plan.allowed_domains.clone(),
            params: serde_json::to_value(&source.params)
                .with_context(|| "serializing source params to JSON")?,
            types: source.types.clone(),
        },
        storage: StorageToy {
            lake_path: lake_path.to_string_lossy().to_string(),
        },
    };

    let init_params = InitializeParams {
        protocol_version: "1.0".to_string(),
        auth: None, // credential is in the grant, not auth
        ducklake: Some(grant),
    };

    eprintln!(
        "[broker] {}: granting lake capabilities to ducklake (lake: {}, types: {:?})",
        source.name, lake_name, source.types
    );

    let (_notifs, response) = lake_child
        .request(
            "pipe/initialize",
            serde_json::to_value(&init_params)
                .with_context(|| "serializing init params")?,
        )
        .map_err(|e| anyhow::anyhow!("ducklake pipe/initialize failed: {}", e))?;

    if let Some(error) = response.get("error") {
        anyhow::bail!(
            "ducklake rejected initialization: {}",
            error["message"].as_str().unwrap_or("unknown error")
        );
    }

    // 4. Tell child to run — child uses toys from here
    let (_notifs, result) = lake_child
        .request("pipe/run", serde_json::json!({}))
        .map_err(|e| anyhow::anyhow!("ducklake pipe/run failed: {}", e))?;

    // 5. Shutdown ducklake child
    if let Err(e) = lake_child.request("pipe/shutdown", serde_json::json!({})) {
        eprintln!("[broker] ducklake: shutdown warning: {}", e);
    }

    // 6. Handle escalation if any
    if let Some(error) = result.get("error") {
        anyhow::bail!(
            "ducklake run error: {}",
            error["message"].as_str().unwrap_or("unknown error")
        );
    }

    let run_result = result.get("result").cloned().unwrap_or(serde_json::json!({}));

    if let Some(escalation) = run_result.get("escalation") {
        if !escalation.is_null() {
            let msg = escalation["message"].as_str().unwrap_or("unknown");
            let action = escalation["action"].as_str().unwrap_or("");
            eprintln!(
                "[broker] {}: lake escalation: {}\n  Action: {}",
                source.name, msg, action
            );
        }
    }

    // 7. Build WriteResult from run result
    let types = run_result.get("types").and_then(|t| t.as_object());
    let total_written: u64 = types
        .map(|t| {
            t.values()
                .filter_map(|v| v.get("written").and_then(|w| w.as_u64()))
                .sum()
        })
        .unwrap_or(0);

    let type_summaries: Vec<String> = types
        .map(|t| {
            t.iter()
                .map(|(k, v)| {
                    let status = v["status"].as_str().unwrap_or("?");
                    let written = v["written"].as_u64().unwrap_or(0);
                    format!("{}: {} ({})", k, written, status)
                })
                .collect()
        })
        .unwrap_or_default();

    eprintln!(
        "[broker] {}: lake '{}' — {}",
        source.name,
        lake_name,
        type_summaries.join(", ")
    );

    Ok(WriteResult {
        inserted: total_written,
        dedup_skipped: 0,
        cursor: None,
    })
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

        // Count facts from this source — use connection's child name
        let child_name = crate::connect::load(&source.connection)
            .ok()
            .map(|r| r.auth.child.clone())
            .unwrap_or_else(|| source.name.clone());
        let source_id = format!("child:{}", child_name);

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
