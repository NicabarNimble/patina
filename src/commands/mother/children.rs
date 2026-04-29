use anyhow::{bail, Context, Result};

use super::ChildrenCommands;

pub(super) fn execute_children(command: ChildrenCommands) -> Result<()> {
    match command {
        ChildrenCommands::Sources => list_sources_cli(),
        ChildrenCommands::Sync { source } => sync_sources_cli(source.as_deref()),
    }
}

fn list_sources_cli() -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let mut sources = store.list_child_registry_sources()?;
    sources.sort_by(|a, b| a.source_id.cmp(&b.source_id));

    let entries = store.list_child_registry_entries(None)?;
    let mut entries_by_source = std::collections::HashMap::<String, usize>::new();
    for entry in entries {
        *entries_by_source.entry(entry.source_id).or_insert(0) += 1;
    }

    println!("Child registry sources: {}", sources.len());
    if sources.is_empty() {
        return Ok(());
    }

    for source in sources {
        let entry_count = entries_by_source
            .get(&source.source_id)
            .copied()
            .unwrap_or(0);
        println!(
            "- {} kind={} enabled={} entries={} last_sync={} status={} error={}",
            source.source_id,
            source.provider_kind,
            source.enabled,
            entry_count,
            source.last_sync_at.as_deref().unwrap_or("<never>"),
            source.last_sync_status.as_deref().unwrap_or("<none>"),
            source.last_error.as_deref().unwrap_or("<none>"),
        );
    }

    Ok(())
}

fn sync_sources_cli(source_id: Option<&str>) -> Result<()> {
    let store = patina::mother::MotherRuntimeStore::default();
    let engine = patina::mother::ChildRegistrySyncEngine::new(store.clone());

    if let Some(source_id) = source_id {
        let source = store
            .get_child_registry_source(source_id)?
            .ok_or_else(|| anyhow::anyhow!("unknown child registry source '{}'", source_id))?;
        let provider = provider_for_kind(&source.provider_kind)?;
        let report = engine.sync_source(&source.source_id, provider.as_ref())?;
        print_sync_report(&report);
        return Ok(());
    }

    let sources = store.list_child_registry_sources()?;
    if sources.is_empty() {
        println!("No child registry sources configured.");
        return Ok(());
    }

    let mut ok = 0usize;
    let mut failed = 0usize;

    for source in sources {
        let provider = match provider_for_kind(&source.provider_kind) {
            Ok(provider) => provider,
            Err(error) => {
                failed += 1;
                println!("✗ {} ({})", source.source_id, error);
                continue;
            }
        };

        match engine.sync_source(&source.source_id, provider.as_ref()) {
            Ok(report) => {
                ok += 1;
                print_sync_report(&report);
            }
            Err(error) => {
                failed += 1;
                println!("✗ {} ({:#})", source.source_id, error);
            }
        }
    }

    println!();
    println!(
        "Child source sync complete: {} succeeded, {} failed",
        ok, failed
    );
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
}
