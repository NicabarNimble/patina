//! Broker — routing engine for Mother.
//!
//! Routes facts from children to destination events.db files based on
//! sources.toml declarations. Manages child lifecycle (spawn, fetch,
//! shutdown) for native children via the pipe protocol.

pub mod connection;
pub mod cursor;
pub mod http;
pub mod lifecycle;
pub mod routing;
pub mod sources;
pub mod spawn;

use anyhow::{Context, Result};
use patina_pipe_types::manifest::ChildManifest;
use std::collections::HashSet;
use std::path::Path;

use self::connection::load_connection;
use self::cursor::{get_cursor, write_facts_with_cursor};
use self::lifecycle::{BrokerChild, FetchParams, NativeChild};
use self::routing::{validate_fact, ValidatedFact, WriteResult};
use self::sources::{Destination, SourceEntry};
use self::spawn::spawn_native;

/// Run a single source: spawn child, fetch facts, validate, route to destination.
///
/// This is the full flow from DESIGN.md — the broker's primary operation.
/// Routes to project events.db or lake based on source.destination.
pub fn run_source(
    source: &SourceEntry,
    project_root: &Path,
    no_sandbox: bool,
) -> Result<WriteResult> {
    // 1. Load connection config
    let conn_config = load_connection(&source.connection)
        .with_context(|| format!("loading connection for source '{}'", source.name))?;

    // 2. Decrypt credential from vault
    let credential = match crate::secrets::get_global_secret(&conn_config.credential) {
        Ok(Some(value)) => Some((conn_config.credential.clone(), value)),
        Ok(None) => {
            eprintln!(
                "[broker] {}: credential '{}' not found in vault, proceeding without auth",
                source.name, conn_config.credential
            );
            None
        }
        Err(e) => {
            eprintln!(
                "[broker] {}: failed to decrypt credential '{}': {}, proceeding without auth",
                source.name, conn_config.credential, e
            );
            None
        }
    };

    // 3. Spawn child with sandbox and credential delivery
    let (mut child, manifest) = spawn_native(
        &conn_config.child,
        credential,
        no_sandbox,
        &conn_config.provider,
        None, // storage_path — project sources use events.db, not lake storage
    )
    .with_context(|| format!("spawning child for source '{}'", source.name))?;

    // Route based on destination
    match &source.destination {
        Destination::Project => write_to_project(source, project_root, &mut child, &manifest),
        Destination::Lake { name } => route_to_lake(source, name, &mut child),
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

    // 6. Build fetch params
    let fetch_params = FetchParams {
        types: source.types.clone(),
        since: stored_cursor,
        params: source.params.clone(),
        limit: None,
    };

    // 7. Fetch facts from child
    let mut validated_facts: Vec<ValidatedFact> = Vec::new();
    let mut warned_schemas: HashSet<String> = HashSet::new();
    let child_name = child.name().to_string();

    let fetch_result = child.fetch(&fetch_params, &mut |fact| {
        match validate_fact(&fact, manifest, &child_name, &mut warned_schemas) {
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

/// Route facts to a named lake via lakehouse child (stub).
///
/// Lake routing requires the lakehouse child process (Seam 3).
/// For now, this logs the intent and returns an error.
fn route_to_lake(
    source: &SourceEntry,
    lake_name: &str,
    child: &mut NativeChild,
) -> Result<WriteResult> {
    eprintln!(
        "[broker] {}: destination is lake '{}' — lake routing not yet implemented (Seam 3)",
        source.name, lake_name
    );

    // Shutdown child cleanly even though we can't route
    if let Err(e) = child.shutdown() {
        eprintln!("[broker] {}: shutdown warning: {}", source.name, e);
    }

    anyhow::bail!(
        "source '{}': lake destination '{}' requires lakehouse child (not yet implemented)",
        source.name,
        lake_name
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
        let conn_config = load_connection(&source.connection).ok();
        let child_name = conn_config
            .as_ref()
            .map(|c| c.child.clone())
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
