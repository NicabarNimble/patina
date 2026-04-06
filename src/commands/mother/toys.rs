use anyhow::{bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
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

fn fetch_upstream_pinned(client: &reqwest::blocking::Client, entry: &ToyEntry) -> Result<Vec<u8>> {
    require_upstream(entry)?;

    let mut last_error: Option<anyhow::Error> = None;
    for git_ref in candidate_refs(entry) {
        for url in candidate_urls(entry, &git_ref) {
            match client.get(&url).send() {
                Ok(response) => {
                    if response.status().is_success() {
                        return response
                            .bytes()
                            .map(|b| b.to_vec())
                            .context("reading upstream response body");
                    }
                    last_error = Some(anyhow::anyhow!(
                        "{} returned HTTP {}",
                        url,
                        response.status()
                    ));
                }
                Err(error) => {
                    last_error = Some(anyhow::anyhow!("request failed for {}: {}", url, error));
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("no upstream URL candidates")))
}

fn fetch_upstream_latest(client: &reqwest::blocking::Client, entry: &ToyEntry) -> Result<Vec<u8>> {
    require_upstream(entry)?;

    let mut last_error: Option<anyhow::Error> = None;
    for git_ref in ["main", "master"] {
        for url in candidate_urls(entry, git_ref) {
            match client.get(&url).send() {
                Ok(response) => {
                    if response.status().is_success() {
                        return response
                            .bytes()
                            .map(|b| b.to_vec())
                            .context("reading upstream latest response body");
                    }
                    last_error = Some(anyhow::anyhow!(
                        "{} returned HTTP {}",
                        url,
                        response.status()
                    ));
                }
                Err(error) => {
                    last_error = Some(anyhow::anyhow!("request failed for {}: {}", url, error));
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("no latest URL candidates")))
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

fn candidate_urls(entry: &ToyEntry, git_ref: &str) -> Vec<String> {
    let repo = entry.source.trim_start_matches("https://github.com/");
    let mut urls = Vec::new();
    for file_name in candidate_file_names(entry) {
        urls.push(format!(
            "https://raw.githubusercontent.com/{}/{}/wit/{}",
            repo, git_ref, file_name
        ));
        urls.push(format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            repo, git_ref, file_name
        ));
    }
    urls
}

fn candidate_refs(entry: &ToyEntry) -> Vec<String> {
    vec![
        format!("v{}", entry.version),
        format!("v{}-draft", entry.version),
        "main".to_string(),
    ]
}

fn candidate_file_names(entry: &ToyEntry) -> Vec<String> {
    let mut names = vec![entry.file.clone()];
    let extra = match entry.id.as_str() {
        "wasi-keyvalue" => Some("store.wit"),
        "wasi-filesystem" => Some("types.wit"),
        "wasi-http" => Some("types.wit"),
        "wasi-sql" => Some("readwrite.wit"),
        _ => None,
    };
    if let Some(extra_name) = extra {
        if !names.iter().any(|n| n == extra_name) {
            names.push(extra_name.to_string());
        }
    }
    names
}

fn short_hash(hash: &str) -> &str {
    let len = hash.len().min(8);
    &hash[..len]
}
