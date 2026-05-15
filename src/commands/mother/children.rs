use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{ChildrenCommands, ChildrenSourceProviderCommands, ChildrenSourcesCommands};

pub(super) fn execute_children(command: ChildrenCommands) -> Result<()> {
    match command {
        ChildrenCommands::Sources { command, json } => match command {
            None => list_sources_cli(json),
            Some(ChildrenSourcesCommands::Add { provider }) => add_source_cli(provider, json),
            Some(ChildrenSourcesCommands::Disable { source_id }) => {
                set_source_enabled_cli(&source_id, false, json)
            }
            Some(ChildrenSourcesCommands::Enable { source_id }) => {
                set_source_enabled_cli(&source_id, true, json)
            }
        },
        ChildrenCommands::Sync { source, json } => sync_sources_cli(source.as_deref(), json),
        ChildrenCommands::Show { target, json } => show_entry_cli(&target, json),
        ChildrenCommands::Search {
            child,
            state,
            source,
            json,
        } => search_entries_cli(child.as_deref(), state.as_deref(), source.as_deref(), json),
        ChildrenCommands::Approve {
            target,
            reason,
            force,
            json,
        } => transition_entry_cli(&target, "approved", reason.as_deref(), force, json),
        ChildrenCommands::Block {
            target,
            reason,
            json,
        } => transition_entry_cli(&target, "blocked", reason.as_deref(), false, json),
        ChildrenCommands::Deprecate {
            target,
            reason,
            json,
        } => transition_entry_cli(&target, "deprecated", reason.as_deref(), false, json),
        ChildrenCommands::Install {
            target,
            installed_by,
            json,
        } => install_entry_cli(&target, installed_by.as_deref(), json),
        ChildrenCommands::Assign {
            target,
            project,
            reason,
            json,
        } => assign_entry_cli(&target, &project, reason.as_deref(), json),
        ChildrenCommands::Unassign {
            child,
            project,
            reason,
            json,
        } => unassign_entry_cli(&child, &project, reason.as_deref(), json),
        ChildrenCommands::Status { project, json } => status_cli(project.as_deref(), json),
    }
}

fn list_sources_cli(as_json: bool) -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let mut sources = store.list_child_registry_sources()?;
    sources.sort_by(|a, b| a.source_id.cmp(&b.source_id));

    let entries = store.list_child_registry_entries(None)?;
    let mut entries_by_source = std::collections::HashMap::<String, usize>::new();
    for entry in entries {
        *entries_by_source.entry(entry.source_id).or_insert(0) += 1;
    }

    let rows = sources
        .into_iter()
        .map(|source| {
            let entry_count = entries_by_source
                .get(&source.source_id)
                .copied()
                .unwrap_or(0);
            json!({
                "source_id": source.source_id,
                "provider_kind": source.provider_kind,
                "enabled": source.enabled,
                "entry_count": entry_count,
                "last_sync_at": source.last_sync_at,
                "last_sync_status": source.last_sync_status,
                "last_error": source.last_error,
            })
        })
        .collect::<Vec<_>>();

    if as_json {
        let payload = json!({
            "total": rows.len(),
            "sources": rows,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!("Child registry sources: {}", rows.len());
    if rows.is_empty() {
        return Ok(());
    }

    for row in rows {
        println!(
            "- {} kind={} enabled={} entries={} last_sync={} status={} error={}",
            row["source_id"].as_str().unwrap_or("<unknown>"),
            row["provider_kind"].as_str().unwrap_or("<unknown>"),
            row["enabled"].as_bool().unwrap_or(false),
            row["entry_count"].as_u64().unwrap_or(0),
            row["last_sync_at"].as_str().unwrap_or("<never>"),
            row["last_sync_status"].as_str().unwrap_or("<none>"),
            row["last_error"].as_str().unwrap_or("<none>"),
        );
    }

    Ok(())
}

fn add_source_cli(provider: ChildrenSourceProviderCommands, as_json: bool) -> Result<()> {
    match provider {
        ChildrenSourceProviderCommands::Github {
            repo,
            source_id,
            child_name,
            tag_prefix,
            asset_name_wasm,
            asset_name_manifest,
            asset_name_checksums,
            include_prerelease,
            patina_min,
            disabled,
        } => add_github_source_cli(
            &repo,
            source_id.as_deref(),
            GitHubSourceOptions {
                child_name: child_name.as_deref(),
                tag_prefix: tag_prefix.as_deref(),
                asset_name_wasm: asset_name_wasm.as_deref(),
                asset_name_manifest: asset_name_manifest.as_deref(),
                asset_name_checksums: asset_name_checksums.as_deref(),
                include_prerelease,
                patina_min: patina_min.as_deref(),
            },
            disabled,
            as_json,
        ),
    }
}

fn set_source_enabled_cli(source_id: &str, enabled: bool, as_json: bool) -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let source = store
        .get_child_registry_source(source_id)?
        .ok_or_else(|| anyhow::anyhow!("unknown child registry source '{}'", source_id))?;

    store.upsert_child_registry_source(&patina::mother::ChildRegistrySourceUpdate {
        source_id: source.source_id.clone(),
        provider_kind: source.provider_kind.clone(),
        provider_config_json: source.provider_config_json.clone(),
        enabled,
    })?;

    if as_json {
        let payload = json!({
            "ok": true,
            "source": {
                "source_id": source.source_id,
                "provider_kind": source.provider_kind,
                "enabled": enabled,
            }
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "✓ {} child source {}",
            if enabled { "Enabled" } else { "Disabled" },
            source_id
        );
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, Default)]
struct GitHubSourceOptions<'a> {
    child_name: Option<&'a str>,
    tag_prefix: Option<&'a str>,
    asset_name_wasm: Option<&'a str>,
    asset_name_manifest: Option<&'a str>,
    asset_name_checksums: Option<&'a str>,
    include_prerelease: bool,
    patina_min: Option<&'a str>,
}

fn add_github_source_cli(
    repo: &str,
    source_id_override: Option<&str>,
    options: GitHubSourceOptions<'_>,
    disabled: bool,
    as_json: bool,
) -> Result<()> {
    let (owner, repo_name) = parse_owner_repo(repo)?;
    let source_id = source_id_override
        .map(|v| v.to_string())
        .unwrap_or_else(|| default_github_source_id(&owner, &repo_name));

    let store = patina::mother::MotherRuntimeStore::default();
    if store.get_child_registry_source(&source_id)?.is_some() {
        bail!(
            "child registry source '{}' already exists; choose --source-id or sync existing source",
            source_id
        );
    }

    let config = github_source_config_json(&owner, &repo_name, options);

    store.upsert_child_registry_source(&patina::mother::ChildRegistrySourceUpdate {
        source_id: source_id.clone(),
        provider_kind: "github".to_string(),
        provider_config_json: serde_json::to_string(&config)?,
        enabled: !disabled,
    })?;

    if as_json {
        let payload = json!({
            "ok": true,
            "source": {
                "source_id": source_id,
                "provider_kind": "github",
                "enabled": !disabled,
                "provider_config": config,
            }
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "✓ Added child source {} (kind=github, enabled={}, repo={}/{})",
            source_id,
            !disabled,
            config["owner"].as_str().unwrap_or(""),
            config["repo"].as_str().unwrap_or("")
        );
    }

    Ok(())
}

fn github_source_config_json(
    owner: &str,
    repo_name: &str,
    options: GitHubSourceOptions<'_>,
) -> serde_json::Value {
    let mut config = json!({
        "owner": owner,
        "repo": repo_name,
    });
    if let Some(child_name) = options.child_name {
        config["child_name"] = json!(child_name);
    }
    if let Some(tag_prefix) = options.tag_prefix {
        config["tag_prefix"] = json!(tag_prefix);
    }
    if let Some(asset_name_wasm) = options.asset_name_wasm {
        config["asset_name_wasm"] = json!(asset_name_wasm);
    }
    if let Some(asset_name_manifest) = options.asset_name_manifest {
        config["asset_name_manifest"] = json!(asset_name_manifest);
    }
    if let Some(asset_name_checksums) = options.asset_name_checksums {
        config["asset_name_checksums"] = json!(asset_name_checksums);
    }
    if options.include_prerelease {
        config["include_prerelease"] = json!(true);
    }
    if let Some(patina_min) = options.patina_min {
        config["patina_min"] = json!(patina_min);
    }
    config
}

fn sync_sources_cli(source_id: Option<&str>, as_json: bool) -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let engine = patina::mother::ChildRegistrySyncEngine::new(store.clone());

    if let Some(source_id) = source_id {
        let source = store
            .get_child_registry_source(source_id)?
            .ok_or_else(|| anyhow::anyhow!("unknown child registry source '{}'", source_id))?;
        let provider = provider_for_kind(&source.provider_kind)?;
        let report = engine.sync_source(&source.source_id, provider.as_ref())?;

        if as_json {
            let payload = json!({
                "ok": true,
                "succeeded": 1,
                "failed": 0,
                "reports": [report_to_json(&report)],
                "errors": []
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            print_sync_report(&report);
        }
        return Ok(());
    }

    let sources = store.list_child_registry_sources()?;
    if sources.is_empty() {
        if as_json {
            let payload = json!({
                "ok": true,
                "succeeded": 0,
                "failed": 0,
                "reports": [],
                "errors": []
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!("No child registry sources configured.");
        }
        return Ok(());
    }

    let mut ok = 0usize;
    let mut failed = 0usize;
    let mut reports = Vec::new();
    let mut errors = Vec::new();

    for source in sources {
        let provider = match provider_for_kind(&source.provider_kind) {
            Ok(provider) => provider,
            Err(error) => {
                failed += 1;
                let message = error.to_string();
                if !as_json {
                    println!("✗ {} ({})", source.source_id, message);
                }
                errors.push(json!({ "source_id": source.source_id, "error": message }));
                continue;
            }
        };

        match engine.sync_source(&source.source_id, provider.as_ref()) {
            Ok(report) => {
                ok += 1;
                if !as_json {
                    print_sync_report(&report);
                }
                reports.push(report_to_json(&report));
            }
            Err(error) => {
                failed += 1;
                let message = format!("{:#}", error);
                if !as_json {
                    println!("✗ {} ({})", source.source_id, message);
                }
                errors.push(json!({ "source_id": source.source_id, "error": message }));
            }
        }
    }

    if as_json {
        let payload = json!({
            "ok": failed == 0,
            "succeeded": ok,
            "failed": failed,
            "reports": reports,
            "errors": errors,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!();
        println!(
            "Child source sync complete: {} succeeded, {} failed",
            ok, failed
        );
    }

    if failed > 0 {
        bail!("one or more child sources failed to sync");
    }
    Ok(())
}

fn show_entry_cli(target: &str, as_json: bool) -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let entry = resolve_entry_target(&store, target)?;
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&entry_record_json(&entry))?
        );
    } else {
        print_entry_record(&entry);
    }
    Ok(())
}

fn search_entries_cli(
    child: Option<&str>,
    state: Option<&str>,
    source: Option<&str>,
    as_json: bool,
) -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let entries = store.list_child_registry_entries(child)?;
    let rows = entries
        .into_iter()
        .filter(|entry| match state {
            Some(state) => entry.state == state,
            None => true,
        })
        .filter(|entry| match source {
            Some(source) => entry.source_id == source,
            None => true,
        })
        .collect::<Vec<_>>();

    if as_json {
        let payload = json!({
            "total": rows.len(),
            "entries": rows.iter().map(entry_record_json).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("Child registry entries: {}", rows.len());
        for entry in rows {
            print_entry_record(&entry);
        }
    }

    Ok(())
}

fn transition_entry_cli(
    target: &str,
    next_state: &str,
    reason: Option<&str>,
    force: bool,
    as_json: bool,
) -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let entry = resolve_entry_target(&store, target)?;

    let (from_state, to_state) =
        store.transition_child_registry_entry_state(&entry.entry_id, next_state, reason, force)?;

    let updated = store
        .get_child_registry_entry_by_id(&entry.entry_id)?
        .ok_or_else(|| anyhow::anyhow!("entry disappeared after state transition"))?;

    let outcome = if from_state == to_state {
        "no-op"
    } else {
        "grant"
    };
    append_audit(
        &store,
        "entry.state.transition",
        outcome,
        None,
        Some(updated.child_name.as_str()),
        Some(updated.entry_id.as_str()),
        reason,
        json!({
            "from": from_state,
            "to": to_state,
            "force": force,
            "target": target,
        }),
    )?;

    if as_json {
        let payload = json!({
            "ok": true,
            "entry": entry_record_json(&updated),
            "transition": {
                "from": from_state,
                "to": to_state,
                "force": force,
            }
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "✓ Transitioned {}: {} -> {}",
            updated.entry_id, from_state, to_state
        );
    }

    Ok(())
}

fn install_entry_cli(target: &str, installed_by: Option<&str>, as_json: bool) -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let entry = resolve_entry_target(&store, target)?;

    if entry.state != "approved" {
        append_audit(
            &store,
            "entry.install",
            "deny",
            None,
            Some(entry.child_name.as_str()),
            Some(entry.entry_id.as_str()),
            Some("entry is not approved"),
            json!({ "target": target, "state": entry.state }),
        )?;
        bail!(
            "entry '{}' is in state '{}' (install requires approved)",
            entry.entry_id,
            entry.state
        );
    }

    let artifact = fetch_url_bytes(&entry.artifact_url)
        .with_context(|| format!("downloading artifact {}", entry.artifact_url))?;
    let manifest = fetch_url_bytes(&entry.manifest_url)
        .with_context(|| format!("downloading manifest {}", entry.manifest_url))?;

    let artifact_sha = sha256_hex(&artifact);
    let manifest_sha = sha256_hex(&manifest);

    if artifact_sha != entry.artifact_sha256 || manifest_sha != entry.manifest_sha256 {
        let reason = "sha256 mismatch during install verification";
        append_audit(
            &store,
            "entry.install",
            "deny",
            None,
            Some(entry.child_name.as_str()),
            Some(entry.entry_id.as_str()),
            Some(reason),
            json!({
                "target": target,
                "artifact_expected": entry.artifact_sha256,
                "artifact_actual": artifact_sha,
                "manifest_expected": entry.manifest_sha256,
                "manifest_actual": manifest_sha,
            }),
        )?;
        bail!(reason);
    }

    let install_paths = install_entry_atomically(&entry.child_name, &artifact, &manifest)?;
    let install_id = format!("inst_{}", uuid::Uuid::new_v4().simple());
    let installer = installed_by
        .map(|v| v.to_string())
        .or_else(|| std::env::var("USER").ok());

    store.upsert_child_install(&patina::mother::ChildInstallUpdate {
        install_id: install_id.clone(),
        entry_id: entry.entry_id.clone(),
        installed_name: entry.child_name.clone(),
        installed_version: entry.version.clone(),
        wasm_path: install_paths.wasm_path.to_string_lossy().to_string(),
        manifest_path: install_paths.manifest_path.to_string_lossy().to_string(),
        artifact_sha256_verified: artifact_sha.clone(),
        manifest_sha256_verified: manifest_sha.clone(),
        installed_by: installer.clone(),
        status: "installed".to_string(),
        last_error: None,
    })?;

    append_audit(
        &store,
        "entry.install",
        "grant",
        None,
        Some(entry.child_name.as_str()),
        Some(entry.entry_id.as_str()),
        Some("install verified and staged atomically"),
        json!({
            "install_id": install_id,
            "wasm_path": install_paths.wasm_path,
            "manifest_path": install_paths.manifest_path,
            "installed_by": installer,
        }),
    )?;

    if as_json {
        let payload = json!({
            "ok": true,
            "entry": entry_record_json(&entry),
            "install": {
                "install_id": install_id,
                "wasm_path": install_paths.wasm_path,
                "manifest_path": install_paths.manifest_path,
                "artifact_sha256_verified": artifact_sha,
                "manifest_sha256_verified": manifest_sha,
            }
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "✓ Installed {}@{} to {}",
            entry.child_name,
            entry.version,
            install_paths.wasm_path.display()
        );
    }

    Ok(())
}

fn assign_entry_cli(
    target: &str,
    project: &str,
    reason: Option<&str>,
    as_json: bool,
) -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let entry = resolve_entry_target(&store, target)?;
    let project_uid = resolve_project_uid(project)?;

    if entry.state != "approved" {
        append_audit(
            &store,
            "assignment.grant",
            "deny",
            Some(project_uid.as_str()),
            Some(entry.child_name.as_str()),
            Some(entry.entry_id.as_str()),
            Some("entry is not approved"),
            json!({ "target": target, "state": entry.state }),
        )?;
        bail!(
            "entry '{}' is in state '{}' (assignment requires approved)",
            entry.entry_id,
            entry.state
        );
    }

    if let Some(existing) =
        store.get_active_project_child_assignment(&project_uid, &entry.child_name)?
    {
        if existing.entry_id != entry.entry_id {
            store.set_project_child_assignment_status(
                &existing.assignment_id,
                "revoked",
                Some("superseded by new assignment"),
            )?;
            append_audit(
                &store,
                "assignment.revoke",
                "grant",
                Some(project_uid.as_str()),
                Some(entry.child_name.as_str()),
                Some(existing.entry_id.as_str()),
                Some("superseded by new assignment"),
                json!({
                    "previous_assignment_id": existing.assignment_id,
                    "new_entry_id": entry.entry_id,
                }),
            )?;
        } else {
            if as_json {
                let payload = json!({
                    "ok": true,
                    "assignment": {
                        "assignment_id": existing.assignment_id,
                        "project_uid": existing.project_uid,
                        "child_name": existing.child_name,
                        "entry_id": existing.entry_id,
                        "pinned_version": existing.pinned_version,
                        "status": existing.status,
                    },
                    "note": "already assigned"
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!(
                    "✓ Assignment already active for {} on {}",
                    entry.child_name, project_uid
                );
            }
            return Ok(());
        }
    }

    let assignment_id = assignment_id_for(&project_uid, &entry.child_name);
    store.upsert_project_child_assignment(&patina::mother::ProjectChildAssignmentUpdate {
        assignment_id: assignment_id.clone(),
        project_uid: project_uid.clone(),
        project_id: None,
        child_name: entry.child_name.clone(),
        entry_id: entry.entry_id.clone(),
        pinned_version: entry.version.clone(),
        status: "active".to_string(),
        reason: reason.map(|v| v.to_string()),
    })?;

    append_audit(
        &store,
        "assignment.grant",
        "grant",
        Some(project_uid.as_str()),
        Some(entry.child_name.as_str()),
        Some(entry.entry_id.as_str()),
        reason,
        json!({ "assignment_id": assignment_id, "target": target }),
    )?;

    if as_json {
        let payload = json!({
            "ok": true,
            "assignment": {
                "assignment_id": assignment_id,
                "project_uid": project_uid,
                "child_name": entry.child_name,
                "entry_id": entry.entry_id,
                "pinned_version": entry.version,
                "status": "active",
            }
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "✓ Assigned {}@{} to project {}",
            entry.child_name, entry.version, project_uid
        );
    }

    Ok(())
}

fn unassign_entry_cli(
    child: &str,
    project: &str,
    reason: Option<&str>,
    as_json: bool,
) -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let project_uid = resolve_project_uid(project)?;

    let existing = store
        .get_active_project_child_assignment(&project_uid, child)?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no active assignment found for child '{}' in project '{}'",
                child,
                project_uid
            )
        })?;

    store.set_project_child_assignment_status(
        &existing.assignment_id,
        "revoked",
        reason.or(Some("operator requested revoke")),
    )?;

    append_audit(
        &store,
        "assignment.revoke",
        "grant",
        Some(project_uid.as_str()),
        Some(child),
        Some(existing.entry_id.as_str()),
        reason,
        json!({ "assignment_id": existing.assignment_id }),
    )?;

    if as_json {
        let payload = json!({
            "ok": true,
            "assignment_id": existing.assignment_id,
            "project_uid": project_uid,
            "child_name": child,
            "status": "revoked",
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "✓ Revoked assignment {} for {}",
            existing.assignment_id, child
        );
    }

    Ok(())
}

fn status_cli(project: Option<&str>, as_json: bool) -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let sources = store.list_child_registry_sources()?;
    let entries = store.list_child_registry_entries(None)?;
    let installs = store.list_child_installs(None)?;

    let project_uid_filter = match project {
        Some(value) => Some(resolve_project_uid(value)?),
        None => None,
    };

    let assignments = store.list_project_child_assignments(project_uid_filter.as_deref())?;
    let active_assignments = assignments
        .iter()
        .filter(|assignment| assignment.status == "active")
        .count();

    let mut by_state = std::collections::BTreeMap::<String, usize>::new();
    for entry in &entries {
        *by_state.entry(entry.state.clone()).or_insert(0) += 1;
    }

    let audits = store.list_child_registry_audit_events(20)?;

    if as_json {
        let payload = json!({
            "ok": true,
            "sources": sources.len(),
            "entries": entries.len(),
            "entries_by_state": by_state,
            "installs": installs.len(),
            "assignments_total": assignments.len(),
            "assignments_active": active_assignments,
            "project_filter": project_uid_filter,
            "recent_audit": audits.into_iter().map(|event| json!({
                "id": event.id,
                "event_kind": event.event_kind,
                "outcome": event.outcome,
                "project_uid": event.project_uid,
                "child_name": event.child_name,
                "entry_id": event.entry_id,
                "reason": event.reason,
                "created_at": event.created_at,
            })).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("Child registry status");
        println!("- sources: {}", sources.len());
        println!("- entries: {}", entries.len());
        println!("- installs: {}", installs.len());
        println!(
            "- assignments: {} total ({} active)",
            assignments.len(),
            active_assignments
        );
        if let Some(project_uid_filter) = project_uid_filter {
            println!("- project filter: {}", project_uid_filter);
        }
        println!("- entry states:");
        for (state, count) in by_state {
            println!("  - {}: {}", state, count);
        }
    }

    Ok(())
}

fn provider_for_kind(kind: &str) -> Result<Box<dyn patina::mother::ChildRegistryProvider>> {
    match kind {
        "github" => Ok(Box::new(
            patina::mother::GitHubChildRegistryProvider::new()
                .context("initializing GitHub child registry provider")?,
        )),
        "gitea" => bail!("gitea child registry provider not implemented yet"),
        "custom" => bail!("custom child registry provider not implemented yet"),
        other => bail!("unsupported child registry provider kind '{}'", other),
    }
}

fn parse_owner_repo(input: &str) -> Result<(String, String)> {
    let value = input
        .trim()
        .trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/")
        .trim_start_matches("git@github.com:")
        .trim_end_matches(".git")
        .trim_matches('/');

    let (owner, repo) = value
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("expected owner/repo, got '{}'", input))?;

    if owner.trim().is_empty() || repo.trim().is_empty() || repo.contains('/') {
        bail!("expected owner/repo, got '{}'", input);
    }

    Ok((owner.to_string(), repo.to_string()))
}

fn default_github_source_id(owner: &str, repo: &str) -> String {
    format!("src_github_{}_{}", slug(owner), slug(repo))
}

fn slug(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn report_to_json(report: &patina::mother::SourceSyncReport) -> serde_json::Value {
    json!({
        "source_id": report.source_id,
        "provider_kind": report.provider_kind,
        "status": report.status,
        "discovered_count": report.discovered_count,
        "upserted_count": report.upserted_count,
        "skipped_count": report.skipped_count,
    })
}

fn print_sync_report(report: &patina::mother::SourceSyncReport) {
    println!(
        "✓ {} kind={} status={} discovered={} upserted={} skipped={}",
        report.source_id,
        report.provider_kind,
        report.status,
        report.discovered_count,
        report.upserted_count,
        report.skipped_count,
    );
}

fn entry_record_json(entry: &patina::mother::ChildRegistryEntryRecord) -> serde_json::Value {
    json!({
        "entry_id": entry.entry_id,
        "child_name": entry.child_name,
        "version": entry.version,
        "source_id": entry.source_id,
        "source_release_ref": entry.source_release_ref,
        "artifact_url": entry.artifact_url,
        "manifest_url": entry.manifest_url,
        "checksums_url": entry.checksums_url,
        "artifact_sha256": entry.artifact_sha256,
        "manifest_sha256": entry.manifest_sha256,
        "signature_ref": entry.signature_ref,
        "patina_min": entry.patina_min,
        "state": entry.state,
        "state_reason": entry.state_reason,
        "updated_at": entry.updated_at,
    })
}

fn print_entry_record(entry: &patina::mother::ChildRegistryEntryRecord) {
    println!(
        "- {} {}@{} state={} source={} updated={}",
        entry.entry_id,
        entry.child_name,
        entry.version,
        entry.state,
        entry.source_id,
        entry.updated_at
    );
}

fn resolve_entry_target(
    store: &patina::mother::MotherRuntimeStore,
    target: &str,
) -> Result<patina::mother::ChildRegistryEntryRecord> {
    if let Some((child, version)) = target.split_once('@') {
        return store
            .get_child_registry_entry_by_child_version(child, version)?
            .ok_or_else(|| anyhow::anyhow!("unknown child entry '{}@{}'", child, version));
    }

    store
        .get_child_registry_entry_by_id(target)?
        .ok_or_else(|| anyhow::anyhow!("unknown child entry id '{}'", target))
}

fn resolve_project_uid(project_target: &str) -> Result<String> {
    let target_path = PathBuf::from(project_target);
    if target_path.exists() {
        let canonical = std::fs::canonicalize(&target_path)
            .with_context(|| format!("canonicalizing {}", target_path.display()))?;
        if !patina::project::is_patina_project(&canonical) {
            bail!(
                "target path is not a Patina project: {}",
                canonical.display()
            );
        }
        return patina::project::register_with_mother(&canonical);
    }

    // treat as uid; validate via paths helper and ensure registered
    patina::paths::mother::projects::project_dir(project_target)
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("invalid project uid '{}'", project_target))?;

    let store = patina::mother::MotherRuntimeStore::default();
    let exists = store
        .list_registered_projects()?
        .into_iter()
        .any(|project| project.project_uid == project_target);
    if !exists {
        bail!(
            "project uid '{}' is not registered; pass a project path or check-in first",
            project_target
        );
    }

    Ok(project_target.to_string())
}

fn assignment_id_for(project_uid: &str, child_name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_uid.as_bytes());
    hasher.update(b"::");
    hasher.update(child_name.as_bytes());
    let digest = hasher.finalize();
    format!("asg_{}", hex_lower(&digest[..12]))
}

#[allow(clippy::too_many_arguments)]
fn append_audit(
    store: &patina::mother::MotherRuntimeStore,
    event_kind: &str,
    outcome: &str,
    project_uid: Option<&str>,
    child_name: Option<&str>,
    entry_id: Option<&str>,
    reason: Option<&str>,
    payload: serde_json::Value,
) -> Result<()> {
    store.append_child_registry_audit_event(&patina::mother::ChildRegistryAuditEventUpdate {
        event_kind: event_kind.to_string(),
        outcome: outcome.to_string(),
        project_uid: project_uid.map(|v| v.to_string()),
        child_name: child_name.map(|v| v.to_string()),
        entry_id: entry_id.map(|v| v.to_string()),
        reason: reason.map(|v| v.to_string()),
        payload_json: payload.to_string(),
    })?;
    Ok(())
}

fn fetch_url_bytes(url: &str) -> Result<Vec<u8>> {
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
    let response = client.get(url).send()?;
    if !response.status().is_success() {
        bail!("HTTP {} while requesting {}", response.status(), url);
    }
    Ok(response.bytes()?.to_vec())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    hex_lower(&digest)
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

struct InstallPaths {
    wasm_path: PathBuf,
    manifest_path: PathBuf,
}

fn install_entry_atomically(
    child_name: &str,
    artifact: &[u8],
    manifest: &[u8],
) -> Result<InstallPaths> {
    let children_dir = patina::paths::child::children_dir();
    std::fs::create_dir_all(&children_dir)
        .with_context(|| format!("creating {}", children_dir.display()))?;

    let wasm_final = children_dir.join(format!("{}.wasm", child_name));
    let manifest_final = children_dir.join(format!("{}.toml", child_name));

    let stage_id = uuid::Uuid::new_v4().simple().to_string();
    let wasm_stage = children_dir.join(format!(".{}.{}.wasm.staged", child_name, stage_id));
    let manifest_stage = children_dir.join(format!(".{}.{}.toml.staged", child_name, stage_id));

    std::fs::write(&wasm_stage, artifact)
        .with_context(|| format!("writing staged wasm {}", wasm_stage.display()))?;
    std::fs::write(&manifest_stage, manifest)
        .with_context(|| format!("writing staged manifest {}", manifest_stage.display()))?;

    atomic_replace(&wasm_stage, &wasm_final)?;
    atomic_replace(&manifest_stage, &manifest_final)?;

    Ok(InstallPaths {
        wasm_path: wasm_final,
        manifest_path: manifest_final,
    })
}

fn atomic_replace(from: &Path, to: &Path) -> Result<()> {
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            let bytes = std::fs::read(from)
                .with_context(|| format!("reading staged file {}", from.display()))?;
            std::fs::write(to, bytes)
                .with_context(|| format!("writing destination file {}", to.display()))?;
            let _ = std::fs::remove_file(from);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_resolution_is_fail_closed_for_unknown_kind() {
        let error = match provider_for_kind("forgejo") {
            Ok(_) => panic!("expected unsupported provider kind to fail"),
            Err(error) => error,
        };
        assert!(error
            .to_string()
            .contains("unsupported child registry provider kind"));
    }

    #[test]
    fn sync_report_json_contains_expected_fields() {
        let value = report_to_json(&patina::mother::SourceSyncReport {
            source_id: "src_github_slate".to_string(),
            provider_kind: "github".to_string(),
            discovered_count: 4,
            upserted_count: 3,
            skipped_count: 1,
            status: "success".to_string(),
        });

        assert_eq!(value["source_id"].as_str(), Some("src_github_slate"));
        assert_eq!(value["provider_kind"].as_str(), Some("github"));
        assert_eq!(value["discovered_count"].as_u64(), Some(4));
        assert_eq!(value["upserted_count"].as_u64(), Some(3));
        assert_eq!(value["skipped_count"].as_u64(), Some(1));
        assert_eq!(value["status"].as_str(), Some("success"));
    }

    #[test]
    fn parse_owner_repo_accepts_common_github_forms() {
        assert_eq!(
            parse_owner_repo("NicabarNimble/patina").unwrap(),
            ("NicabarNimble".to_string(), "patina".to_string())
        );
        assert_eq!(
            parse_owner_repo("https://github.com/NicabarNimble/patina").unwrap(),
            ("NicabarNimble".to_string(), "patina".to_string())
        );
        assert_eq!(
            parse_owner_repo("git@github.com:NicabarNimble/patina.git").unwrap(),
            ("NicabarNimble".to_string(), "patina".to_string())
        );
    }

    #[test]
    fn github_source_config_records_release_selectors() {
        let config = github_source_config_json(
            "NicabarNimble",
            "patina-child-watcher-system",
            GitHubSourceOptions {
                child_name: Some("demo-child"),
                tag_prefix: Some("demo-child-v"),
                asset_name_wasm: Some("patina_ai_child_demo_child.wasm"),
                asset_name_manifest: Some("child.toml"),
                asset_name_checksums: Some("checksums.txt"),
                include_prerelease: true,
                patina_min: Some("0.71.0"),
            },
        );

        assert_eq!(config["owner"].as_str(), Some("NicabarNimble"));
        assert_eq!(config["repo"].as_str(), Some("patina-child-watcher-system"));
        assert_eq!(config["child_name"].as_str(), Some("demo-child"));
        assert_eq!(config["tag_prefix"].as_str(), Some("demo-child-v"));
        assert_eq!(
            config["asset_name_wasm"].as_str(),
            Some("patina_ai_child_demo_child.wasm")
        );
        assert_eq!(config["asset_name_manifest"].as_str(), Some("child.toml"));
        assert_eq!(
            config["asset_name_checksums"].as_str(),
            Some("checksums.txt")
        );
        assert_eq!(config["include_prerelease"].as_bool(), Some(true));
        assert_eq!(config["patina_min"].as_str(), Some("0.71.0"));
    }

    #[test]
    fn default_github_source_id_is_stable_slug() {
        assert_eq!(
            default_github_source_id("NicabarNimble", "patina-child-slate"),
            "src_github_nicabarnimble_patina_child_slate"
        );
    }

    #[test]
    fn assignment_id_is_stable() {
        let one = assignment_id_for("2bdc808e", "slate-manager");
        let two = assignment_id_for("2bdc808e", "slate-manager");
        assert_eq!(one, two);
    }

    #[test]
    fn resolve_entry_target_parses_child_version() {
        assert_eq!(
            "slate-manager@0.1.0".split_once('@').unwrap().0,
            "slate-manager"
        );
    }
}
