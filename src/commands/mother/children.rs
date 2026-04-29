use anyhow::{bail, Context, Result};
use serde_json::json;

use super::{ChildrenCommands, ChildrenSourceProviderCommands, ChildrenSourcesCommands};

pub(super) fn execute_children(command: ChildrenCommands) -> Result<()> {
    match command {
        ChildrenCommands::Sources { command, json } => match command {
            None => list_sources_cli(json),
            Some(ChildrenSourcesCommands::Add { provider }) => add_source_cli(provider, json),
        },
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

fn add_source_cli(provider: ChildrenSourceProviderCommands, as_json: bool) -> Result<()> {
    match provider {
        ChildrenSourceProviderCommands::Github {
            repo,
            source_id,
            child_name,
            disabled,
        } => add_github_source_cli(
            &repo,
            source_id.as_deref(),
            child_name.as_deref(),
            disabled,
            as_json,
        ),
    }
}

fn add_github_source_cli(
    repo: &str,
    source_id_override: Option<&str>,
    child_name: Option<&str>,
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

    let mut config = json!({
        "owner": owner,
        "repo": repo_name,
    });
    if let Some(child_name) = child_name {
        config["child_name"] = json!(child_name);
    }

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
    fn default_github_source_id_is_stable_slug() {
        assert_eq!(
            default_github_source_id("NicabarNimble", "patina-child-slate"),
            "src_github_nicabarnimble_patina_child_slate"
        );
    }
}
