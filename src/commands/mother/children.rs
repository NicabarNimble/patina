use anyhow::{bail, Context, Result};
use serde_json::json;

use super::ChildrenCommands;

pub(super) fn execute_children(command: ChildrenCommands) -> Result<()> {
    match command {
        ChildrenCommands::Sources { json } => list_sources_cli(json),
        ChildrenCommands::Sync { source, json } => sync_sources_cli(source.as_deref(), json),
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
}
