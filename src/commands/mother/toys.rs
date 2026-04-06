use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

const TOYS_REGISTRY_PATH: &str = "wit/toys/deps/toys-registry.toml";
const TOYS_DEPS_PATH: &str = "wit/toys/deps";

#[derive(Debug, Clone)]
pub struct ToyEntry {
    pub id: String,
    pub source: String,
    pub version: String,
    pub file: String,
    pub upstream_files: Vec<String>,
    pub hash: Option<String>,
    pub phase: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Semver {
    major: u64,
    minor: u64,
    patch: u64,
}

#[derive(Debug, Clone)]
struct LatestStableVersion {
    version: String,
    semver: Semver,
    source: &'static str,
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

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct GithubTagRef {
    #[serde(rename = "ref")]
    ref_name: String,
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

impl ToyEntry {
    pub fn tier(&self) -> &'static str {
        if self.id.starts_with("wasi-") {
            if self.phase.is_some() {
                "wasi"
            } else {
                "wasi-proposal"
            }
        } else {
            "patina-delta"
        }
    }
}

pub fn toys_status(project_root: &Path) -> Result<()> {
    let entries = load_registry(project_root)?;

    println!(
        "Toy Registry: {}",
        project_root.join(TOYS_REGISTRY_PATH).display()
    );
    println!();
    println!(
        "{:<22} {:<8} {:<10} {:<5} {:<14}",
        "name", "version", "source", "phase", "tier"
    );
    println!("{:-<22} {:-<8} {:-<10} {:-<5} {:-<14}", "", "", "", "", "");

    for entry in entries {
        let source = if entry.source == "patina" {
            "patina"
        } else {
            "wasi"
        };
        let phase = entry
            .phase
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:<22} {:<8} {:<10} {:<5} {:<14}",
            entry.id,
            entry.version,
            source,
            phase,
            entry.tier()
        );
    }

    Ok(())
}

pub fn toys_check(project_root: &Path) -> Result<()> {
    let entries = load_registry(project_root)?;

    let mut failures = Vec::new();
    println!("Checking toy WIT files against pinned versions:\n");

    for entry in entries {
        let local_path = local_wit_path(project_root, &entry);
        let local_hash = hash_file(&local_path)?;

        if entry.source == "patina" {
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
    let entries = load_registry(project_root)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("building HTTP client")?;

    let mut failures = Vec::new();
    let mut behind = 0usize;

    println!("Sync report against latest upstream stable releases:\n");
    println!(
        "{:<22} {:<10} {:<10} {:<20} {:<10}",
        "name", "pinned", "latest", "age", "source"
    );
    println!(
        "{:-<22} {:-<10} {:-<10} {:-<20} {:-<10}",
        "", "", "", "", ""
    );

    for entry in entries {
        if entry.source == "patina" {
            println!(
                "{:<22} {:<10} {:<10} {:<20} {:<10}",
                entry.id, entry.version, "-", "patina-delta", "-"
            );
            continue;
        }

        match fetch_latest_stable_version(&client, &entry) {
            Ok(latest) => {
                let age = version_age(&entry.version, &latest.semver)?;
                if latest.version != entry.version {
                    behind += 1;
                }
                println!(
                    "{:<22} {:<10} {:<10} {:<20} {:<10}",
                    entry.id, entry.version, latest.version, age, latest.source
                );
            }
            Err(error) => {
                failures.push(format!("{}: {}", entry.id, error));
                println!(
                    "{:<22} {:<10} {:<10} {:<20} {:<10}",
                    entry.id, entry.version, "error", "unavailable", "error"
                );
            }
        }
    }

    println!();
    if behind == 0 {
        println!("All WASI toy pins are on the latest stable release.");
    } else {
        println!(
            "{} WASI toy(s) are behind latest stable releases; update version pins before pull.",
            behind
        );
    }

    if failures.is_empty() {
        Ok(())
    } else {
        println!("\nSync errors:");
        for failure in &failures {
            println!("- {}", failure);
        }
        bail!("toy sync encountered {} fetch error(s)", failures.len())
    }
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
    let entries = load_registry(project_root)?;
    let entry = entries
        .into_iter()
        .find(|entry| entry.id == toy_name)
        .ok_or_else(|| anyhow::anyhow!("unknown toy '{}'", toy_name))?;
    require_upstream(&entry)?;

    let local_path = local_wit_path(project_root, &entry);
    let old_local =
        std::fs::read(&local_path).with_context(|| format!("reading {}", local_path.display()))?;
    let registry_path = project_root.join(TOYS_REGISTRY_PATH);
    let old_registry = std::fs::read(&registry_path)
        .with_context(|| format!("reading {}", registry_path.display()))?;

    let changed = pull_entry(project_root, client, &entry)?;
    if !changed {
        println!(
            "unchanged {:<22} hash matches pinned {}",
            entry.id, entry.version
        );
        return Ok(());
    }

    match run_check(project_root) {
        Ok(()) => {
            println!(
                "pulled {:<22} updated {} and registry hash",
                entry.id,
                local_path.display()
            );
            Ok(())
        }
        Err(error) => {
            std::fs::write(&local_path, &old_local)
                .with_context(|| format!("restoring {}", local_path.display()))?;
            std::fs::write(&registry_path, &old_registry)
                .with_context(|| format!("restoring {}", registry_path.display()))?;
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
    let entries = load_registry(project_root)?;
    let wasi_entries: Vec<ToyEntry> = entries
        .into_iter()
        .filter(|entry| entry.source != "patina")
        .collect();

    let mut snapshot_targets = vec![project_root.join(TOYS_REGISTRY_PATH)];
    for entry in &wasi_entries {
        snapshot_targets.push(local_wit_path(project_root, entry));
    }
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

    if let Err(error) = run_check(project_root) {
        restore_snapshot(project_root, &snapshot)?;
        for report in &mut reports {
            if matches!(report.status, PullStatus::Pulled) {
                report.status = PullStatus::Reverted;
                report.detail = "reverted after compile failure".to_string();
            }
        }
        print_pull_report(&reports);
        cleanup_snapshot(&snapshot);
        bail!(
            "pull --all reverted all changes because compile failed: {}. Pull individual toys to isolate the breakage.",
            error
        );
    }

    print_pull_report(&reports);
    cleanup_snapshot(&snapshot);
    Ok(())
}

pub fn load_registry(project_root: &Path) -> Result<Vec<ToyEntry>> {
    let registry_path = project_root.join(TOYS_REGISTRY_PATH);
    let content = std::fs::read_to_string(&registry_path)
        .with_context(|| format!("missing {}", registry_path.display()))?;

    let value: toml::Value =
        toml::from_str(&content).with_context(|| format!("invalid {}", registry_path.display()))?;
    let table = value
        .as_table()
        .ok_or_else(|| anyhow::anyhow!("registry must be a TOML table"))?;

    let mut entries = Vec::new();
    for (id, raw_entry) in table {
        let entry_table = raw_entry
            .as_table()
            .ok_or_else(|| anyhow::anyhow!("entry '{}' must be a TOML table", id))?;

        let source = string_field(entry_table, id, "source")?;
        let version = string_field(entry_table, id, "version")?;
        let file = string_field(entry_table, id, "file")?;
        let hash = optional_string_field(entry_table, "hash");
        let phase = optional_u32_field(entry_table, "phase");

        entries.push(ToyEntry {
            id: id.to_string(),
            source,
            version,
            file,
            upstream_files: string_array_field(entry_table, "upstream_files"),
            hash,
            phase,
        });
    }

    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(entries)
}

pub fn local_wit_path(project_root: &Path, entry: &ToyEntry) -> PathBuf {
    project_root.join(TOYS_DEPS_PATH).join(&entry.file)
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
    let upstream_contents = fetch_upstream_pinned(client, entry)?;
    let composed = compose_upstream_wit(&upstream_contents)?;
    let composed_hash = hash_bytes(composed.as_bytes());
    if entry
        .hash
        .as_deref()
        .map(|hash| hash.strip_prefix("sha256:").unwrap_or(hash))
        == Some(composed_hash.as_str())
    {
        return Ok(false);
    }

    let local_path = local_wit_path(project_root, entry);
    std::fs::write(&local_path, composed.as_bytes())
        .with_context(|| format!("writing {}", local_path.display()))?;
    update_registry_hash(project_root, &entry.id, &composed_hash)?;
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

fn optional_u32_field(table: &toml::value::Table, field: &str) -> Option<u32> {
    table
        .get(field)
        .and_then(toml::Value::as_integer)
        .and_then(|v| u32::try_from(v).ok())
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
    if entry.source == "patina" {
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
) -> Result<Vec<String>> {
    require_upstream(entry)?;
    let repo = repo_slug(entry)?;
    let files = if entry.upstream_files.is_empty() {
        vec![entry.file.clone()]
    } else {
        entry.upstream_files.clone()
    };
    let refs = candidate_pull_refs(client, &repo, &entry.version);
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
    files: &[String],
) -> std::result::Result<Vec<String>, FetchRefError> {
    let mut out = Vec::with_capacity(files.len());
    for upstream_file in files {
        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            repo, reference, upstream_file
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
        out.push(body);
    }
    Ok(out)
}

fn candidate_pull_refs(
    client: &reqwest::blocking::Client,
    repo: &str,
    version: &str,
) -> Vec<String> {
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

fn update_registry_hash(project_root: &Path, toy_id: &str, hash: &str) -> Result<()> {
    let registry_path = project_root.join(TOYS_REGISTRY_PATH);
    let content = std::fs::read_to_string(&registry_path)
        .with_context(|| format!("reading {}", registry_path.display()))?;
    let mut value: toml::Value =
        toml::from_str(&content).with_context(|| format!("parsing {}", registry_path.display()))?;
    let table = value
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("registry must be a TOML table"))?;
    let entry = table
        .get_mut(toy_id)
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| anyhow::anyhow!("toy '{}' missing from registry", toy_id))?;
    entry.insert(
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

fn fetch_latest_stable_version(
    client: &reqwest::blocking::Client,
    entry: &ToyEntry,
) -> Result<LatestStableVersion> {
    require_upstream(entry)?;
    let repo = repo_slug(entry)?;

    let release_url = format!("https://api.github.com/repos/{}/releases", repo);
    match client
        .get(&release_url)
        .header(reqwest::header::USER_AGENT, "patina-toys-sync")
        .send()
    {
        Ok(response) if response.status().is_success() => {
            let releases: Vec<GithubRelease> = response
                .json()
                .context("parsing GitHub releases response")?;
            if let Some((version, semver)) = latest_from_releases(&releases) {
                return Ok(LatestStableVersion {
                    version,
                    semver,
                    source: "releases",
                });
            }
        }
        Ok(response) => {
            let status = response.status();
            let tag_fallback = fetch_latest_tag_version(client, &repo)
                .with_context(|| format!("releases API HTTP {} and tag fallback failed", status))?;
            return Ok(tag_fallback);
        }
        Err(error) => {
            let tag_fallback = fetch_latest_tag_version(client, &repo).with_context(|| {
                format!(
                    "releases API request failed ({}) and tag fallback failed",
                    error
                )
            })?;
            return Ok(tag_fallback);
        }
    }

    fetch_latest_tag_version(client, &repo)
        .context("no stable releases found and tag fallback failed")
}

fn fetch_latest_tag_version(
    client: &reqwest::blocking::Client,
    repo: &str,
) -> Result<LatestStableVersion> {
    let tags_url = format!("https://api.github.com/repos/{}/git/refs/tags", repo);
    let response = client
        .get(&tags_url)
        .header(reqwest::header::USER_AGENT, "patina-toys-sync")
        .send()
        .context("requesting GitHub tag refs")?;
    if !response.status().is_success() {
        bail!("tag refs API returned HTTP {}", response.status());
    }
    let tags: Vec<GithubTagRef> = response
        .json()
        .context("parsing GitHub tag refs response")?;
    let (version, semver) = latest_from_tags(&tags)
        .ok_or_else(|| anyhow::anyhow!("no stable semver tags found in git refs"))?;
    Ok(LatestStableVersion {
        version,
        semver,
        source: "tags",
    })
}

fn latest_from_releases(releases: &[GithubRelease]) -> Option<(String, Semver)> {
    releases
        .iter()
        .filter(|release| !release.draft && !release.prerelease)
        .filter_map(|release| normalized_semver(&release.tag_name))
        .max_by(|(_, left), (_, right)| left.cmp(right))
}

fn latest_from_tags(tags: &[GithubTagRef]) -> Option<(String, Semver)> {
    tags.iter()
        .filter_map(|tag| {
            let name = tag
                .ref_name
                .strip_prefix("refs/tags/")
                .unwrap_or(tag.ref_name.as_str())
                .trim_end_matches("^{}");
            normalized_semver(name)
        })
        .max_by(|(_, left), (_, right)| left.cmp(right))
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
    fn latest_from_tags_uses_semver_only() {
        let tags = vec![
            GithubTagRef {
                ref_name: "refs/tags/v0.2.0-draft".to_string(),
            },
            GithubTagRef {
                ref_name: "refs/tags/v0.1.9".to_string(),
            },
            GithubTagRef {
                ref_name: "refs/tags/0.2.1^{}".to_string(),
            },
        ];

        let (version, semver) = latest_from_tags(&tags).expect("stable semver tag");
        assert_eq!(version, "0.2.1");
        assert_eq!(semver.major, 0);
        assert_eq!(semver.minor, 2);
        assert_eq!(semver.patch, 1);
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
            phase: None,
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
        let old = "old-keyvalue";
        let old_hash = hash_bytes(old.as_bytes());
        let registry = format!(
            "[wasi-keyvalue]\nsource = \"https://github.com/WebAssembly/wasi-keyvalue\"\nversion = \"0.2.0\"\nfile = \"keyvalue.wit\"\nhash = \"sha256:{old_hash}\"\n"
        );
        let temp = setup_project(&registry, &[("keyvalue.wit", old)]);
        let root = temp.path();
        let client = test_http_client();

        let result = toys_pull_with(
            root,
            "wasi-keyvalue",
            &client,
            |project_root, _, entry| {
                let local_path = local_wit_path(project_root, entry);
                let new = "new-keyvalue";
                std::fs::write(&local_path, new).context("write updated local keyvalue")?;
                update_registry_hash(project_root, &entry.id, &hash_bytes(new.as_bytes()))?;
                Ok(true)
            },
            |_| bail!("forced compile failure"),
        );

        assert!(result.is_err());
        let err = result.expect_err("expected rollback error").to_string();
        assert!(err.contains("pull reverted"));

        let local = std::fs::read_to_string(root.join(TOYS_DEPS_PATH).join("keyvalue.wit"))
            .expect("read reverted keyvalue file");
        assert_eq!(local, old);

        let entry = load_registry(root)
            .expect("load registry")
            .into_iter()
            .find(|entry| entry.id == "wasi-keyvalue")
            .expect("wasi-keyvalue registry entry");
        let expected = format!("sha256:{old_hash}");
        assert_eq!(entry.hash.as_deref(), Some(expected.as_str()));
    }
}
