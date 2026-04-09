use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

const TOYS_REGISTRY_PATH: &str = "wit/toys/deps/toys-registry.toml";
const TOYS_DEPS_PATH: &str = "wit/toys/deps";
const TOYS_WASI_P2_PATH: &str = "wit/toys/wasi-p2";
const TOYS_PATINA_PATH: &str = "wit/toys/patina";
const PREVIEW2_PROPOSALS: [&str; 7] = [
    "io",
    "clocks",
    "random",
    "filesystem",
    "sockets",
    "cli",
    "http",
];

#[derive(Debug, Clone)]
pub struct ToyEntry {
    pub id: String,
    pub source: String,
    pub version: String,
    pub file: String,
    pub upstream_files: Vec<String>,
    pub hash: Option<String>,
    pub path: Option<String>,
    pub registry_key: String,
    pub tier: ToyTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToyTier {
    Preview2,
    Patina,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Semver {
    major: u64,
    minor: u64,
    patch: u64,
}

#[derive(Debug, Clone, Copy)]
enum PullStatus {
    Pulled,
    Unchanged,
    Failed,
    Reverted,
}

#[derive(Debug, Clone)]
struct PullReport {
    id: String,
    status: PullStatus,
    detail: String,
}

#[derive(Debug, Clone)]
pub struct RegistryData {
    pub preview2: Preview2Meta,
    pub entries: Vec<ToyEntry>,
}

#[derive(Debug, Clone)]
pub struct Preview2Meta {
    pub source: String,
    pub version: String,
    pub proposals: Vec<String>,
}

#[derive(Debug, Clone)]
struct Preview2ReleaseStatus {
    stable_version: String,
    stable_semver: Semver,
    rc_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct GithubTag {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GithubRepo {
    default_branch: String,
}

enum FetchRefError {
    NotFound,
    Fatal(anyhow::Error),
}

#[derive(Debug, Clone)]
struct UpstreamFile {
    relative_path: String,
    content: String,
}

impl ToyEntry {
    pub fn tier_label(&self) -> &'static str {
        match self.tier {
            ToyTier::Preview2 => "wasi-preview2",
            ToyTier::Patina => "patina",
        }
    }
}

pub fn toys_status(project_root: &Path) -> Result<()> {
    let registry = load_registry(project_root)?;
    let entries = registry.entries;
    let preview2_count = entries
        .iter()
        .filter(|entry| entry.tier == ToyTier::Preview2)
        .count();

    println!(
        "Toy Registry: {}",
        project_root.join(TOYS_REGISTRY_PATH).display()
    );
    println!(
        "WASI Preview 2: v{} ({} proposal(s))",
        registry.preview2.version,
        registry.preview2.proposals.len()
    );
    println!("Preview 2 source: {}", registry.preview2.source);
    println!(
        "Patina toys: {}",
        entries.len().saturating_sub(preview2_count)
    );
    println!();
    println!(
        "{:<22} {:<8} {:<10} {:<14}",
        "name", "version", "source", "tier"
    );
    println!("{:-<22} {:-<8} {:-<10} {:-<14}", "", "", "", "");

    for entry in entries {
        println!(
            "{:<22} {:<8} {:<10} {:<14}",
            entry.id,
            entry.version,
            if entry.tier == ToyTier::Preview2 {
                "preview2"
            } else {
                "patina"
            },
            entry.tier_label()
        );
    }

    Ok(())
}

pub fn toys_check(project_root: &Path) -> Result<()> {
    let entries = load_registry(project_root)?.entries;

    let mut failures = Vec::new();
    println!("Checking toy WIT files against pinned versions:\n");

    for entry in entries {
        let local_hash = if entry.tier == ToyTier::Patina {
            let local_path = local_wit_path(project_root, &entry);
            hash_file(&local_path)?
        } else {
            hash_bytes(compose_local_preview2_wit(project_root, &entry)?.as_bytes())
        };

        if entry.tier == ToyTier::Patina {
            println!("ok   {:<22} local-only (patina delta)", entry.id);
            continue;
        }

        let Some(expected_hash) = entry.hash.as_deref() else {
            failures.push(format!("{}: missing pinned hash in registry", entry.id));
            println!("fail {:<22} missing hash field in registry", entry.id);
            continue;
        };
        let expected_hash = expected_hash
            .strip_prefix("sha256:")
            .unwrap_or(expected_hash);

        if expected_hash == local_hash {
            println!("ok   {:<22} matches pinned {}", entry.id, entry.version);
        } else {
            failures.push(format!(
                "{}: local hash {} does not match pinned hash {}",
                entry.id, local_hash, expected_hash
            ));
            println!(
                "fail {:<22} hash mismatch vs pinned {}",
                entry.id, entry.version
            );
        }
    }

    if failures.is_empty() {
        println!("\nAll toy WIT files match pinned versions.");
        Ok(())
    } else {
        println!("\nMismatches:");
        for failure in &failures {
            println!("- {}", failure);
        }
        bail!("toy registry check failed with {} issue(s)", failures.len())
    }
}

pub fn toys_sync(project_root: &Path) -> Result<()> {
    let registry = load_registry(project_root)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("building HTTP client")?;

    let latest = fetch_preview2_release_status(&client)?;
    let pinned = &registry.preview2.version;

    println!("Sync report against WASI Preview 2 monorepo:\n");
    println!(
        "{:<16} {:<10} {:<10} {:<10} {:<16}",
        "target", "pinned", "stable", "latest-rc", "age"
    );
    println!(
        "{:-<16} {:-<10} {:-<10} {:-<10} {:-<16}",
        "", "", "", "", ""
    );

    let age = version_age(pinned, &latest.stable_semver)?;
    println!(
        "{:<16} {:<10} {:<10} {:<10} {:<16}",
        "wasi-preview2",
        pinned,
        latest.stable_version,
        latest.rc_version.as_deref().unwrap_or("-"),
        age
    );

    if latest.stable_version == *pinned {
        println!("\nPreview 2 pin is on latest stable release.");
    } else {
        println!(
            "\nPreview 2 pin is behind latest stable release; update [preview2].version before pull."
        );
    }

    Ok(())
}

pub fn toys_pull(project_root: &Path, toy_name: &str) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("building HTTP client")?;
    toys_pull_with(
        project_root,
        toy_name,
        &client,
        pull_entry_without_compile,
        run_sdk_check,
    )
}

pub fn toys_pull_preview2(project_root: &Path) -> Result<()> {
    toys_pull_all(project_root)
}

fn toys_pull_with<P, C>(
    project_root: &Path,
    toy_name: &str,
    client: &reqwest::blocking::Client,
    pull_entry: P,
    run_check: C,
) -> Result<()>
where
    P: Fn(&Path, &reqwest::blocking::Client, &ToyEntry) -> Result<bool>,
    C: Fn(&Path) -> Result<()>,
{
    let entries = load_registry(project_root)?.entries;
    let entry = entries
        .into_iter()
        .find(|entry| entry.id == toy_name)
        .ok_or_else(|| anyhow::anyhow!("unknown toy '{}'", toy_name))?;
    require_upstream(&entry)?;

    let mut snapshot_targets = vec![project_root.join(TOYS_REGISTRY_PATH)];
    if entry.tier == ToyTier::Preview2 {
        snapshot_targets.push(preview2_local_deps_dir(project_root, &entry));
        snapshot_targets.push(preview2_legacy_flat_path(project_root, &entry));
    } else {
        snapshot_targets.push(local_wit_path(project_root, &entry));
    }
    let raw_dir = wasi_p2_raw_dir(project_root, &entry);
    let had_raw_dir = raw_dir.exists();
    if entry.tier == ToyTier::Preview2 {
        snapshot_targets.push(raw_dir.clone());
    }
    let snapshot = create_snapshot(project_root, &snapshot_targets)?;

    let changed = pull_entry(project_root, client, &entry)?;
    if !changed {
        cleanup_snapshot(&snapshot);
        println!(
            "unchanged {:<22} hash matches pinned {}",
            entry.id, entry.version
        );
        return Ok(());
    }

    match run_check(project_root) {
        Ok(()) => {
            cleanup_snapshot(&snapshot);
            let location = if entry.tier == ToyTier::Preview2 {
                preview2_local_deps_dir(project_root, &entry)
                    .display()
                    .to_string()
            } else {
                local_wit_path(project_root, &entry).display().to_string()
            };
            println!(
                "pulled {:<22} updated {} and registry hash",
                entry.id, location
            );
            Ok(())
        }
        Err(error) => {
            restore_snapshot(project_root, &snapshot)?;
            cleanup_snapshot(&snapshot);
            if entry.tier == ToyTier::Preview2 && !had_raw_dir && raw_dir.exists() {
                let _ = std::fs::remove_dir_all(&raw_dir);
            }
            bail!(
                "pull reverted for '{}' after compile failure: {}. Run `cargo check -q -p patina-sdk --features child` for details.",
                entry.id,
                error
            )
        }
    }
}

pub fn toys_pull_all(project_root: &Path) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("building HTTP client")?;
    toys_pull_all_with(
        project_root,
        &client,
        pull_entry_without_compile,
        run_sdk_check,
    )
}

fn toys_pull_all_with<P, C>(
    project_root: &Path,
    client: &reqwest::blocking::Client,
    pull_entry: P,
    run_check: C,
) -> Result<()>
where
    P: Fn(&Path, &reqwest::blocking::Client, &ToyEntry) -> Result<bool>,
    C: Fn(&Path) -> Result<()>,
{
    let entries = load_registry(project_root)?.entries;
    let wasi_entries: Vec<ToyEntry> = entries
        .into_iter()
        .filter(|entry| entry.tier == ToyTier::Preview2)
        .collect();

    let mut snapshot_targets = vec![project_root.join(TOYS_REGISTRY_PATH)];
    for entry in &wasi_entries {
        snapshot_targets.push(preview2_local_deps_dir(project_root, entry));
        snapshot_targets.push(preview2_legacy_flat_path(project_root, entry));
    }
    let wasi_p2_root = project_root.join(TOYS_WASI_P2_PATH);
    let had_wasi_p2_root = wasi_p2_root.exists();
    snapshot_targets.push(wasi_p2_root.clone());
    let snapshot = create_snapshot(project_root, &snapshot_targets)?;

    let mut reports = Vec::new();
    for entry in &wasi_entries {
        match pull_entry(project_root, client, entry) {
            Ok(true) => reports.push(PullReport {
                id: entry.id.clone(),
                status: PullStatus::Pulled,
                detail: "file updated".to_string(),
            }),
            Ok(false) => reports.push(PullReport {
                id: entry.id.clone(),
                status: PullStatus::Unchanged,
                detail: "hash already current".to_string(),
            }),
            Err(error) => reports.push(PullReport {
                id: entry.id.clone(),
                status: PullStatus::Failed,
                detail: error.to_string(),
            }),
        }
    }

    let failed_count = reports
        .iter()
        .filter(|report| matches!(report.status, PullStatus::Failed))
        .count();
    if failed_count > 0 {
        restore_snapshot(project_root, &snapshot)?;
        if !had_wasi_p2_root && wasi_p2_root.exists() {
            let _ = std::fs::remove_dir_all(&wasi_p2_root);
        }
        for report in &mut reports {
            if matches!(report.status, PullStatus::Pulled) {
                report.status = PullStatus::Reverted;
                report.detail = "reverted after pull failure".to_string();
            }
        }
        print_pull_report(&reports);
        cleanup_snapshot(&snapshot);
        bail!(
            "pull --preview2 reverted all changes because {} proposal(s) failed to pull",
            failed_count
        );
    }

    if let Err(error) = run_check(project_root) {
        restore_snapshot(project_root, &snapshot)?;
        if !had_wasi_p2_root && wasi_p2_root.exists() {
            let _ = std::fs::remove_dir_all(&wasi_p2_root);
        }
        for report in &mut reports {
            if matches!(report.status, PullStatus::Pulled) {
                report.status = PullStatus::Reverted;
                report.detail = "reverted after compile failure".to_string();
            }
        }
        print_pull_report(&reports);
        cleanup_snapshot(&snapshot);
        bail!(
            "pull --preview2 reverted all changes because compile failed: {}.",
            error
        );
    }

    print_pull_report(&reports);
    cleanup_snapshot(&snapshot);
    Ok(())
}

pub fn load_registry(project_root: &Path) -> Result<RegistryData> {
    let registry_path = project_root.join(TOYS_REGISTRY_PATH);
    let content = std::fs::read_to_string(&registry_path)
        .with_context(|| format!("missing {}", registry_path.display()))?;

    let value: toml::Value =
        toml::from_str(&content).with_context(|| format!("invalid {}", registry_path.display()))?;
    let table = value
        .as_table()
        .ok_or_else(|| anyhow::anyhow!("registry must be a TOML table"))?;

    let preview2 = table
        .get("preview2")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| anyhow::anyhow!("registry missing [preview2] section"))?;
    let preview2_source = string_field(preview2, "preview2", "source")?;
    let preview2_version = string_field(preview2, "preview2", "version")?;
    let preview2_proposals = string_array_field(preview2, "proposals");
    if preview2_proposals.is_empty() {
        bail!("[preview2] must declare proposals");
    }
    for required in PREVIEW2_PROPOSALS {
        if !preview2_proposals
            .iter()
            .any(|proposal| proposal == required)
        {
            bail!(
                "[preview2].proposals missing '{}': expected all 7 Preview 2 proposals",
                required
            );
        }
    }

    let mut entries = Vec::new();

    for proposal in &preview2_proposals {
        let entry_table = preview2
            .get(proposal)
            .and_then(toml::Value::as_table)
            .ok_or_else(|| anyhow::anyhow!("registry missing [preview2.{}] section", proposal))?;

        let file = string_field(entry_table, proposal, "file")?;
        let hash = optional_string_field(entry_table, "hash");
        let upstream_files = string_array_field(entry_table, "upstream_files");
        let path = optional_string_field(entry_table, "path");
        entries.push(ToyEntry {
            id: format!("wasi-{}", proposal),
            source: preview2_source.clone(),
            version: preview2_version.clone(),
            file,
            upstream_files,
            hash,
            path,
            registry_key: proposal.to_string(),
            tier: ToyTier::Preview2,
        });
    }

    for (id, raw_entry) in table {
        if id == "preview2" || id.starts_with("preview2.") {
            continue;
        }
        let entry_table = raw_entry
            .as_table()
            .ok_or_else(|| anyhow::anyhow!("entry '{}' must be a TOML table", id))?;

        let source = string_field(entry_table, id, "source")?;
        let version = string_field(entry_table, id, "version")?;
        let file = string_field(entry_table, id, "file")?;
        let hash = optional_string_field(entry_table, "hash");
        if source != "patina" {
            bail!(
                "entry '{}' must be source='patina' or live under [preview2.*]",
                id
            );
        }

        entries.push(ToyEntry {
            id: id.to_string(),
            source,
            version,
            file,
            upstream_files: string_array_field(entry_table, "upstream_files"),
            hash,
            path: optional_string_field(entry_table, "path"),
            registry_key: id.to_string(),
            tier: ToyTier::Patina,
        });
    }

    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(RegistryData {
        preview2: Preview2Meta {
            source: preview2_source,
            version: preview2_version,
            proposals: preview2_proposals,
        },
        entries,
    })
}

pub fn local_wit_path(project_root: &Path, entry: &ToyEntry) -> PathBuf {
    let base = match entry.tier {
        ToyTier::Preview2 => TOYS_DEPS_PATH,
        ToyTier::Patina => TOYS_PATINA_PATH,
    };
    project_root.join(base).join(&entry.file)
}

fn preview2_local_deps_dir(project_root: &Path, entry: &ToyEntry) -> PathBuf {
    project_root.join(TOYS_DEPS_PATH).join(&entry.registry_key)
}

fn preview2_legacy_flat_path(project_root: &Path, entry: &ToyEntry) -> PathBuf {
    project_root.join(TOYS_DEPS_PATH).join(&entry.file)
}

fn preview2_file_order(entry: &ToyEntry) -> Vec<String> {
    if entry.upstream_files.is_empty() {
        vec![entry.file.clone()]
    } else {
        entry.upstream_files.clone()
    }
}

fn compose_local_preview2_wit(project_root: &Path, entry: &ToyEntry) -> Result<String> {
    let deps_dir = preview2_local_deps_dir(project_root, entry);
    if deps_dir.exists() {
        let mut parts = Vec::new();
        for file in preview2_file_order(entry) {
            let path = deps_dir.join(&file);
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            parts.push(content);
        }
        return compose_upstream_wit(&parts);
    }

    let legacy = preview2_legacy_flat_path(project_root, entry);
    let content = std::fs::read_to_string(&legacy)
        .with_context(|| format!("reading {}", legacy.display()))?;
    compose_upstream_wit(&[content])
}

fn write_preview2_local_deps(
    project_root: &Path,
    entry: &ToyEntry,
    files: &[UpstreamFile],
) -> Result<()> {
    let deps_dir = preview2_local_deps_dir(project_root, entry);
    if deps_dir.exists() {
        std::fs::remove_dir_all(&deps_dir)
            .with_context(|| format!("clearing {}", deps_dir.display()))?;
    }
    std::fs::create_dir_all(&deps_dir)
        .with_context(|| format!("creating {}", deps_dir.display()))?;

    for file in files {
        let target = deps_dir.join(&file.relative_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&target, file.content.as_bytes())
            .with_context(|| format!("writing {}", target.display()))?;
    }

    let legacy = preview2_legacy_flat_path(project_root, entry);
    if legacy.exists() {
        let _ = std::fs::remove_file(&legacy);
    }
    Ok(())
}

fn wasi_p2_raw_dir(project_root: &Path, entry: &ToyEntry) -> PathBuf {
    project_root
        .join(TOYS_WASI_P2_PATH)
        .join(&entry.registry_key)
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn hash_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(hash_bytes(&bytes))
}

fn pull_entry_without_compile(
    project_root: &Path,
    client: &reqwest::blocking::Client,
    entry: &ToyEntry,
) -> Result<bool> {
    let upstream_files = fetch_upstream_pinned(client, entry)?;
    if entry.tier == ToyTier::Preview2 {
        write_wasi_p2_raw_files(project_root, entry, &upstream_files)?;
        write_preview2_local_deps(project_root, entry, &upstream_files)?;
    }
    let composed_inputs: Vec<String> = upstream_files
        .iter()
        .map(|file| file.content.clone())
        .collect();
    let composed = compose_upstream_wit(&composed_inputs)?;
    let composed_hash = hash_bytes(composed.as_bytes());
    if entry
        .hash
        .as_deref()
        .map(|hash| hash.strip_prefix("sha256:").unwrap_or(hash))
        == Some(composed_hash.as_str())
    {
        return Ok(false);
    }

    if entry.tier == ToyTier::Patina {
        let local_path = local_wit_path(project_root, entry);
        std::fs::write(&local_path, composed.as_bytes())
            .with_context(|| format!("writing {}", local_path.display()))?;
    }
    update_registry_hash(project_root, entry, &composed_hash)?;
    Ok(true)
}

fn create_snapshot(project_root: &Path, targets: &[PathBuf]) -> Result<PathBuf> {
    let snapshot_root = std::env::temp_dir().join(format!(
        "patina-toys-snapshot-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&snapshot_root)
        .with_context(|| format!("creating {}", snapshot_root.display()))?;
    for target in targets {
        if !target.exists() {
            continue;
        }
        if target.is_dir() {
            for entry in walkdir::WalkDir::new(target)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_file())
            {
                let path = entry.path();
                let relative = path.strip_prefix(project_root).unwrap_or(path);
                let snapshot_path = snapshot_root.join(relative);
                if let Some(parent) = snapshot_path.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("creating {}", parent.display()))?;
                }
                std::fs::copy(path, &snapshot_path)
                    .with_context(|| format!("snapshot {}", path.display()))?;
            }
            continue;
        }

        let relative = target
            .strip_prefix(project_root)
            .unwrap_or(target.as_path());
        let snapshot_path = snapshot_root.join(relative);
        if let Some(parent) = snapshot_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::copy(target, &snapshot_path)
            .with_context(|| format!("snapshot {}", target.display()))?;
    }
    Ok(snapshot_root)
}

fn restore_snapshot(project_root: &Path, snapshot_root: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(snapshot_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let relative = path
            .strip_prefix(snapshot_root)
            .with_context(|| format!("computing snapshot relative path for {}", path.display()))?;
        let target = project_root.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::copy(path, &target).with_context(|| format!("restoring {}", target.display()))?;
    }
    Ok(())
}

fn cleanup_snapshot(snapshot_root: &Path) {
    let _ = std::fs::remove_dir_all(snapshot_root);
}

fn print_pull_report(reports: &[PullReport]) {
    println!("\nPull report:");
    for report in reports {
        let status = match report.status {
            PullStatus::Pulled => "pulled",
            PullStatus::Unchanged => "unchanged",
            PullStatus::Failed => "failed",
            PullStatus::Reverted => "reverted",
        };
        println!("- {:<22} {:<9} {}", report.id, status, report.detail);
    }
}

fn string_field(table: &toml::value::Table, entry_id: &str, field: &str) -> Result<String> {
    table
        .get(field)
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("entry '{}' missing string field '{}'", entry_id, field))
}

fn optional_string_field(table: &toml::value::Table, field: &str) -> Option<String> {
    table
        .get(field)
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
}

fn string_array_field(table: &toml::value::Table, field: &str) -> Vec<String> {
    table
        .get(field)
        .and_then(toml::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub fn require_upstream(entry: &ToyEntry) -> Result<()> {
    if entry.tier == ToyTier::Patina {
        bail!(
            "toy '{}' is patina-delta and has no upstream WASI source",
            entry.id
        );
    }
    Ok(())
}

fn fetch_upstream_pinned(
    client: &reqwest::blocking::Client,
    entry: &ToyEntry,
) -> Result<Vec<UpstreamFile>> {
    require_upstream(entry)?;
    let repo = repo_slug(entry)?;
    let files: Vec<(String, String)> = if entry.upstream_files.is_empty() {
        vec![(entry.file.clone(), entry.file.clone())]
    } else {
        entry
            .upstream_files
            .iter()
            .map(|upstream_file| {
                let request_path = if let Some(path) = &entry.path {
                    format!("{}/{}", path, upstream_file)
                } else {
                    upstream_file.clone()
                };
                (request_path, upstream_file.clone())
            })
            .collect()
    };
    let refs = candidate_pull_refs(client, &repo, &entry.version, entry.tier);
    let mut attempts = Vec::new();
    for reference in refs {
        match fetch_files_for_ref(client, &repo, &reference, &files) {
            Ok(contents) => return Ok(contents),
            Err(FetchRefError::NotFound) => attempts.push(reference),
            Err(FetchRefError::Fatal(error)) => return Err(error),
        }
    }

    bail!(
        "unable to fetch upstream files for '{}' at pinned version '{}'; tried refs: {}",
        entry.id,
        entry.version,
        attempts.join(", ")
    )
}

fn fetch_files_for_ref(
    client: &reqwest::blocking::Client,
    repo: &str,
    reference: &str,
    files: &[(String, String)],
) -> std::result::Result<Vec<UpstreamFile>, FetchRefError> {
    let mut out = Vec::with_capacity(files.len());
    for (request_path, relative_path) in files {
        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            repo, reference, request_path
        );
        let response = client.get(&url).send().map_err(|error| {
            FetchRefError::Fatal(anyhow::anyhow!("request failed for {}: {}", url, error))
        })?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(FetchRefError::NotFound);
        }
        if !response.status().is_success() {
            return Err(FetchRefError::Fatal(anyhow::anyhow!(
                "{} returned HTTP {}",
                url,
                response.status()
            )));
        }
        let body = response.text().map_err(|error| {
            FetchRefError::Fatal(anyhow::anyhow!("reading upstream response body: {}", error))
        })?;
        out.push(UpstreamFile {
            relative_path: relative_path.clone(),
            content: body,
        });
    }
    Ok(out)
}

fn candidate_pull_refs(
    client: &reqwest::blocking::Client,
    repo: &str,
    version: &str,
    tier: ToyTier,
) -> Vec<String> {
    if tier == ToyTier::Preview2 {
        return vec![format!("v{}", version), version.to_string()];
    }
    let mut refs = Vec::new();
    refs.push(format!("v{}", version));
    refs.push(version.to_string());

    if let Ok(tags) = fetch_matching_tags(client, repo, version) {
        refs.extend(tags);
    }
    if let Ok(default_branch) = fetch_default_branch(client, repo) {
        refs.push(default_branch);
    }

    let mut seen = HashSet::new();
    refs.into_iter()
        .filter(|reference| seen.insert(reference.clone()))
        .collect()
}

fn fetch_matching_tags(
    client: &reqwest::blocking::Client,
    repo: &str,
    version: &str,
) -> Result<Vec<String>> {
    let tags_url = format!("https://api.github.com/repos/{}/tags?per_page=100", repo);
    let response = client
        .get(&tags_url)
        .header(reqwest::header::USER_AGENT, "patina-toys-sync")
        .send()
        .context("requesting repository tags")?;
    if !response.status().is_success() {
        bail!("tags API returned HTTP {}", response.status());
    }
    let tags: Vec<GithubTag> = response.json().context("parsing GitHub tags response")?;
    Ok(tags
        .into_iter()
        .map(|tag| tag.name)
        .filter(|name| tag_matches_version(name, version))
        .collect())
}

fn fetch_default_branch(client: &reqwest::blocking::Client, repo: &str) -> Result<String> {
    let repo_url = format!("https://api.github.com/repos/{}", repo);
    let response = client
        .get(&repo_url)
        .header(reqwest::header::USER_AGENT, "patina-toys-sync")
        .send()
        .context("requesting repository metadata")?;
    if !response.status().is_success() {
        bail!(
            "repository metadata API returned HTTP {}",
            response.status()
        );
    }
    let repo: GithubRepo = response
        .json()
        .context("parsing repository metadata response")?;
    Ok(repo.default_branch)
}

fn tag_matches_version(tag: &str, version: &str) -> bool {
    let normalized = tag.strip_prefix('v').unwrap_or(tag);
    normalized == version || normalized.strip_prefix(version) == Some("-draft")
}

fn compose_upstream_wit(files: &[String]) -> Result<String> {
    if files.is_empty() {
        bail!("cannot compose empty upstream file list");
    }
    if files.len() == 1 {
        return Ok(files[0].clone());
    }

    let mut package_seen = false;
    let mut use_lines: Vec<String> = Vec::new();
    let mut body_lines: Vec<String> = Vec::new();

    for content in files {
        for line in content.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("package ") {
                if package_seen {
                    continue;
                }
                package_seen = true;
                body_lines.push(line.to_string());
                continue;
            }
            if trimmed.starts_with("use ") {
                use_lines.push(line.to_string());
                continue;
            }
            body_lines.push(line.to_string());
        }
    }

    let mut composed = Vec::new();
    let mut inserted_uses = false;
    for line in body_lines {
        if !inserted_uses && !line.trim().is_empty() && !line.trim_start().starts_with("package ") {
            if !use_lines.is_empty() {
                composed.extend(use_lines.iter().cloned());
                composed.push(String::new());
            }
            inserted_uses = true;
        }
        composed.push(line);
    }
    if !inserted_uses && !use_lines.is_empty() {
        composed.extend(use_lines);
    }

    Ok(format!("{}\n", composed.join("\n")))
}

fn write_wasi_p2_raw_files(
    project_root: &Path,
    entry: &ToyEntry,
    files: &[UpstreamFile],
) -> Result<()> {
    let raw_dir = wasi_p2_raw_dir(project_root, entry);
    if raw_dir.exists() {
        std::fs::remove_dir_all(&raw_dir)
            .with_context(|| format!("clearing {}", raw_dir.display()))?;
    }
    std::fs::create_dir_all(&raw_dir).with_context(|| format!("creating {}", raw_dir.display()))?;

    for file in files {
        let target = raw_dir.join(&file.relative_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&target, file.content.as_bytes())
            .with_context(|| format!("writing {}", target.display()))?;
    }
    Ok(())
}

fn update_registry_hash(project_root: &Path, entry: &ToyEntry, hash: &str) -> Result<()> {
    let registry_path = project_root.join(TOYS_REGISTRY_PATH);
    let content = std::fs::read_to_string(&registry_path)
        .with_context(|| format!("reading {}", registry_path.display()))?;
    let mut value: toml::Value =
        toml::from_str(&content).with_context(|| format!("parsing {}", registry_path.display()))?;
    let table = value
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("registry must be a TOML table"))?;
    let entry_table = if entry.tier == ToyTier::Preview2 {
        table
            .get_mut("preview2")
            .and_then(toml::Value::as_table_mut)
            .and_then(|preview2| preview2.get_mut(&entry.registry_key))
            .and_then(toml::Value::as_table_mut)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "toy '[preview2.{}]' missing from registry",
                    entry.registry_key
                )
            })?
    } else {
        table
            .get_mut(&entry.registry_key)
            .and_then(toml::Value::as_table_mut)
            .ok_or_else(|| anyhow::anyhow!("toy '{}' missing from registry", entry.registry_key))?
    };
    entry_table.insert(
        "hash".to_string(),
        toml::Value::String(format!("sha256:{}", hash)),
    );
    let rendered = toml::to_string_pretty(&value).context("rendering updated registry")?;
    std::fs::write(&registry_path, rendered)
        .with_context(|| format!("writing {}", registry_path.display()))?;
    Ok(())
}

fn run_sdk_check(project_root: &Path) -> Result<()> {
    let status = std::process::Command::new("cargo")
        .args(["check", "-q", "-p", "patina-sdk", "--features", "child"])
        .current_dir(project_root)
        .status()
        .context("running cargo check for patina-sdk")?;
    if status.success() {
        Ok(())
    } else {
        bail!("cargo check failed with status {}", status)
    }
}

fn fetch_preview2_release_status(
    client: &reqwest::blocking::Client,
) -> Result<Preview2ReleaseStatus> {
    let release_url = "https://api.github.com/repos/WebAssembly/WASI/releases";
    let response = client
        .get(release_url)
        .header(reqwest::header::USER_AGENT, "patina-toys-sync")
        .send()
        .context("requesting WASI monorepo releases")?;
    if !response.status().is_success() {
        bail!("WASI releases API returned HTTP {}", response.status());
    }
    let releases: Vec<GithubRelease> = response.json().context("parsing WASI releases response")?;
    let (stable_version, stable_semver) = latest_from_releases(&releases)
        .ok_or_else(|| anyhow::anyhow!("no stable WASI releases found"))?;
    let rc_version = latest_rc_from_releases(&releases);
    Ok(Preview2ReleaseStatus {
        stable_version,
        stable_semver,
        rc_version,
    })
}

fn latest_from_releases(releases: &[GithubRelease]) -> Option<(String, Semver)> {
    releases
        .iter()
        .filter(|release| !release.draft && !release.prerelease)
        .filter_map(|release| normalized_semver(&release.tag_name))
        .max_by(|(_, left), (_, right)| left.cmp(right))
}

fn latest_rc_from_releases(releases: &[GithubRelease]) -> Option<String> {
    releases
        .iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let tag = release
                .tag_name
                .strip_prefix('v')
                .unwrap_or(&release.tag_name);
            if !tag.contains("-rc") {
                return None;
            }
            Some(tag.to_string())
        })
        .max()
}

fn normalized_semver(raw: &str) -> Option<(String, Semver)> {
    let normalized = raw.strip_prefix('v').unwrap_or(raw).to_string();
    if normalized.contains('-') || normalized.contains('+') {
        return None;
    }
    let mut parts = normalized.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((
        normalized,
        Semver {
            major,
            minor,
            patch,
        },
    ))
}

fn version_age(pinned: &str, latest: &Semver) -> Result<String> {
    let (_, pinned_semver) = normalized_semver(pinned)
        .ok_or_else(|| anyhow::anyhow!("pinned version '{}' is not semver", pinned))?;
    if pinned_semver == *latest {
        return Ok("current".to_string());
    }
    if pinned_semver > *latest {
        return Ok("ahead".to_string());
    }
    Ok(format!(
        "behind {}.{}.{}",
        latest.major.saturating_sub(pinned_semver.major),
        latest.minor.saturating_sub(pinned_semver.minor),
        latest.patch.saturating_sub(pinned_semver.patch)
    ))
}

fn repo_slug(entry: &ToyEntry) -> Result<String> {
    require_upstream(entry)?;
    let slug = entry
        .source
        .trim_start_matches("https://github.com/")
        .trim_end_matches('/')
        .to_string();
    if slug.split('/').count() != 2 {
        bail!("invalid GitHub source URL '{}'", entry.source);
    }
    Ok(slug)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn latest_from_releases_filters_unstable_and_normalizes_v_prefix() {
        let releases = vec![
            GithubRelease {
                tag_name: "v0.2.7".to_string(),
                draft: false,
                prerelease: false,
            },
            GithubRelease {
                tag_name: "v0.2.8-rc.1".to_string(),
                draft: false,
                prerelease: false,
            },
            GithubRelease {
                tag_name: "0.2.9".to_string(),
                draft: true,
                prerelease: false,
            },
            GithubRelease {
                tag_name: "v0.2.8".to_string(),
                draft: false,
                prerelease: false,
            },
        ];

        let (version, semver) = latest_from_releases(&releases).expect("stable release");
        assert_eq!(version, "0.2.8");
        assert_eq!(semver.major, 0);
        assert_eq!(semver.minor, 2);
        assert_eq!(semver.patch, 8);
    }

    #[test]
    fn tag_matches_version_accepts_draft_variant() {
        assert!(tag_matches_version("v0.2.0-draft", "0.2.0"));
        assert!(tag_matches_version("0.2.0", "0.2.0"));
        assert!(!tag_matches_version("v0.2.1", "0.2.0"));
    }

    #[test]
    fn require_upstream_rejects_patina_entries() {
        let entry = ToyEntry {
            id: "patina-connect".to_string(),
            source: "patina".to_string(),
            version: "0.1.0".to_string(),
            file: "patina-connect.wit".to_string(),
            upstream_files: Vec::new(),
            hash: None,
            path: None,
            registry_key: "patina-connect".to_string(),
            tier: ToyTier::Patina,
        };

        assert!(require_upstream(&entry).is_err());
    }

    #[test]
    fn compose_multi_file_wit_keeps_single_package_and_moves_use_lines() {
        let files = vec![
            "package wasi:demo@0.1.0;\n\ninterface first {\n  a: func();\n}\n".to_string(),
            "package wasi:demo@0.1.0;\nuse dep.{x};\n\ninterface second {\n  b: func(v: x);\n}\n"
                .to_string(),
        ];

        let composed = compose_upstream_wit(&files).expect("compose success");
        assert_eq!(composed.matches("package wasi:demo@0.1.0;").count(), 1);
        let use_pos = composed.find("use dep.{x};").expect("use line present");
        let second_pos = composed
            .find("interface second")
            .expect("second interface present");
        assert!(use_pos < second_pos);
    }

    fn test_http_client() -> reqwest::blocking::Client {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("test http client")
    }

    fn setup_project(registry: &str, files: &[(&str, &str)]) -> TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let deps = root.join(TOYS_DEPS_PATH);
        std::fs::create_dir_all(&deps).expect("create deps dir");
        std::fs::write(root.join(TOYS_REGISTRY_PATH), registry).expect("write registry");
        for (file, content) in files {
            std::fs::write(deps.join(file), content).expect("write toy file");
        }
        temp
    }

    #[test]
    fn pull_reverts_local_file_and_registry_hash_on_compile_failure() {
        let old = "old-http";
        let old_hash = hash_bytes(old.as_bytes());
        let registry = format!(
            "[preview2]\nsource = \"https://github.com/WebAssembly/WASI\"\nversion = \"0.2.8\"\nproposals = [\"io\", \"clocks\", \"random\", \"filesystem\", \"sockets\", \"cli\", \"http\"]\n\n[preview2.io]\npath = \"wasip2/io\"\nupstream_files = [\"error.wit\", \"poll.wit\", \"streams.wit\"]\nfile = \"io.wit\"\n\n[preview2.clocks]\npath = \"wasip2/clocks\"\nupstream_files = [\"monotonic-clock.wit\", \"wall-clock.wit\", \"timezone.wit\"]\nfile = \"clocks.wit\"\n\n[preview2.random]\npath = \"wasip2/random\"\nupstream_files = [\"random.wit\", \"insecure.wit\", \"insecure-seed.wit\"]\nfile = \"random.wit\"\n\n[preview2.filesystem]\npath = \"wasip2/filesystem\"\nupstream_files = [\"types.wit\", \"preopens.wit\"]\nfile = \"filesystem.wit\"\n\n[preview2.sockets]\npath = \"wasip2/sockets\"\nupstream_files = [\"network.wit\", \"tcp.wit\", \"udp.wit\", \"ip-name-lookup.wit\"]\nfile = \"sockets.wit\"\n\n[preview2.cli]\npath = \"wasip2/cli\"\nupstream_files = [\"command.wit\", \"environment.wit\", \"exit.wit\", \"run.wit\", \"stdio.wit\", \"terminal.wit\"]\nfile = \"cli.wit\"\n\n[preview2.http]\npath = \"wasip2/http\"\nupstream_files = [\"types.wit\", \"handler.wit\"]\nfile = \"http.wit\"\nhash = \"sha256:{old_hash}\"\n"
        );
        let temp = setup_project(&registry, &[("http.wit", old)]);
        let root = temp.path();
        let client = test_http_client();

        let result = toys_pull_with(
            root,
            "wasi-http",
            &client,
            |project_root, _, entry| {
                let local_path = local_wit_path(project_root, entry);
                let new = "new-http";
                std::fs::write(&local_path, new).context("write updated local http")?;
                update_registry_hash(project_root, entry, &hash_bytes(new.as_bytes()))?;
                Ok(true)
            },
            |_| bail!("forced compile failure"),
        );

        assert!(result.is_err());
        let err = result.expect_err("expected rollback error").to_string();
        assert!(err.contains("pull reverted"));

        let local = std::fs::read_to_string(root.join(TOYS_DEPS_PATH).join("http.wit"))
            .expect("read reverted http file");
        assert_eq!(local, old);

        let entry = load_registry(root)
            .expect("load registry")
            .entries
            .into_iter()
            .find(|entry| entry.id == "wasi-http")
            .expect("wasi-http registry entry");
        let expected = format!("sha256:{old_hash}");
        assert_eq!(entry.hash.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn pull_all_reverts_all_files_and_registry_hashes_on_compile_failure() {
        let old_http = "old-http";
        let old_io = "old-io";
        let old_http_hash = hash_bytes(old_http.as_bytes());
        let old_io_hash = hash_bytes(old_io.as_bytes());
        let registry = format!(
            "[preview2]\nsource = \"https://github.com/WebAssembly/WASI\"\nversion = \"0.2.8\"\nproposals = [\"io\", \"clocks\", \"random\", \"filesystem\", \"sockets\", \"cli\", \"http\"]\n\n[preview2.io]\npath = \"wasip2/io\"\nupstream_files = [\"error.wit\", \"poll.wit\", \"streams.wit\"]\nfile = \"io.wit\"\nhash = \"sha256:{old_io_hash}\"\n\n[preview2.clocks]\npath = \"wasip2/clocks\"\nupstream_files = [\"monotonic-clock.wit\", \"wall-clock.wit\", \"timezone.wit\"]\nfile = \"clocks.wit\"\n\n[preview2.random]\npath = \"wasip2/random\"\nupstream_files = [\"random.wit\", \"insecure.wit\", \"insecure-seed.wit\"]\nfile = \"random.wit\"\n\n[preview2.filesystem]\npath = \"wasip2/filesystem\"\nupstream_files = [\"types.wit\", \"preopens.wit\"]\nfile = \"filesystem.wit\"\n\n[preview2.sockets]\npath = \"wasip2/sockets\"\nupstream_files = [\"network.wit\", \"tcp.wit\", \"udp.wit\", \"ip-name-lookup.wit\"]\nfile = \"sockets.wit\"\n\n[preview2.cli]\npath = \"wasip2/cli\"\nupstream_files = [\"command.wit\", \"environment.wit\", \"exit.wit\", \"run.wit\", \"stdio.wit\", \"terminal.wit\"]\nfile = \"cli.wit\"\n\n[preview2.http]\npath = \"wasip2/http\"\nupstream_files = [\"types.wit\", \"handler.wit\"]\nfile = \"http.wit\"\nhash = \"sha256:{old_http_hash}\"\n\n[patina-connect]\nsource = \"patina\"\nversion = \"0.2.0\"\nfile = \"patina-connect.wit\"\n"
        );
        let temp = setup_project(
            &registry,
            &[
                ("http.wit", old_http),
                ("io.wit", old_io),
                ("patina-connect.wit", "delta"),
            ],
        );
        let root = temp.path();
        let client = test_http_client();

        let result = toys_pull_all_with(
            root,
            &client,
            |project_root, _, entry| {
                let new_content = format!("new-{}", entry.id);
                let local_path = local_wit_path(project_root, entry);
                std::fs::write(&local_path, &new_content)
                    .with_context(|| format!("write updated {}", entry.id))?;
                update_registry_hash(project_root, entry, &hash_bytes(new_content.as_bytes()))?;
                Ok(true)
            },
            |_| bail!("forced compile failure"),
        );

        assert!(result.is_err());
        let err = result.expect_err("expected rollback error").to_string();
        assert!(err.contains("reverted all changes"));

        let http_local = std::fs::read_to_string(root.join(TOYS_DEPS_PATH).join("http.wit"))
            .expect("read reverted http file");
        let io_local = std::fs::read_to_string(root.join(TOYS_DEPS_PATH).join("io.wit"))
            .expect("read reverted io file");
        assert_eq!(http_local, old_http);
        assert_eq!(io_local, old_io);

        let entries = load_registry(root).expect("load reverted registry").entries;
        let http_entry = entries
            .iter()
            .find(|entry| entry.id == "wasi-http")
            .expect("wasi-http entry");
        let io_entry = entries
            .iter()
            .find(|entry| entry.id == "wasi-io")
            .expect("wasi-io entry");
        let expected_http_hash = format!("sha256:{old_http_hash}");
        let expected_io_hash = format!("sha256:{old_io_hash}");
        assert_eq!(
            http_entry.hash.as_deref(),
            Some(expected_http_hash.as_str())
        );
        assert_eq!(io_entry.hash.as_deref(), Some(expected_io_hash.as_str()));
    }

    #[test]
    fn pull_rejects_patina_delta_toy_by_name() {
        let registry = "[preview2]\nsource = \"https://github.com/WebAssembly/WASI\"\nversion = \"0.2.8\"\nproposals = [\"io\", \"clocks\", \"random\", \"filesystem\", \"sockets\", \"cli\", \"http\"]\n\n[preview2.io]\npath = \"wasip2/io\"\nupstream_files = [\"error.wit\", \"poll.wit\", \"streams.wit\"]\nfile = \"io.wit\"\n\n[preview2.clocks]\npath = \"wasip2/clocks\"\nupstream_files = [\"monotonic-clock.wit\", \"wall-clock.wit\", \"timezone.wit\"]\nfile = \"clocks.wit\"\n\n[preview2.random]\npath = \"wasip2/random\"\nupstream_files = [\"random.wit\", \"insecure.wit\", \"insecure-seed.wit\"]\nfile = \"random.wit\"\n\n[preview2.filesystem]\npath = \"wasip2/filesystem\"\nupstream_files = [\"types.wit\", \"preopens.wit\"]\nfile = \"filesystem.wit\"\n\n[preview2.sockets]\npath = \"wasip2/sockets\"\nupstream_files = [\"network.wit\", \"tcp.wit\", \"udp.wit\", \"ip-name-lookup.wit\"]\nfile = \"sockets.wit\"\n\n[preview2.cli]\npath = \"wasip2/cli\"\nupstream_files = [\"command.wit\", \"environment.wit\", \"exit.wit\", \"run.wit\", \"stdio.wit\", \"terminal.wit\"]\nfile = \"cli.wit\"\n\n[preview2.http]\npath = \"wasip2/http\"\nupstream_files = [\"types.wit\", \"handler.wit\"]\nfile = \"http.wit\"\n\n[patina-connect]\nsource = \"patina\"\nversion = \"0.2.0\"\nfile = \"patina-connect.wit\"\n";
        let temp = setup_project(registry, &[("patina-connect.wit", "delta")]);
        let root = temp.path();
        let client = test_http_client();

        let result = toys_pull_with(
            root,
            "patina-connect",
            &client,
            |_, _, _| bail!("puller should not run for patina delta toy"),
            |_| Ok(()),
        );

        assert!(result.is_err());
        let err = result.expect_err("expected patina rejection").to_string();
        assert!(err.contains("patina-delta"));
    }
}
