use anyhow::{bail, Context, Result};
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
    pub phase: Option<u32>,
    pub wasi_overlap: Option<String>,
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
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("building HTTP client")?;

    let mut failures = Vec::new();
    println!("Checking toy WIT files against pinned versions:\n");

    for entry in entries {
        let local_path = local_wit_path(project_root, &entry);
        let local_hash = hash_file(&local_path)?;

        if entry.source == "patina" {
            println!("ok   {:<22} local-only (patina delta)", entry.id);
            continue;
        }

        let pinned_bytes = match fetch_upstream_pinned(&client, &entry) {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push(format!("{}: {}", entry.id, error));
                println!("fail {:<22} unable to fetch pinned source", entry.id);
                continue;
            }
        };
        let pinned_hash = hash_bytes(&pinned_bytes);

        if pinned_hash == local_hash {
            println!("ok   {:<22} matches pinned {}", entry.id, entry.version);
        } else {
            failures.push(format!(
                "{}: local hash {} does not match pinned hash {}",
                entry.id, local_hash, pinned_hash
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
    let mut changed = 0usize;

    println!("Sync report against latest upstream WASI refs:\n");

    for entry in entries {
        if entry.source == "patina" {
            println!("skip {:<22} patina delta", entry.id);
            continue;
        }

        let pinned_bytes = match fetch_upstream_pinned(&client, &entry) {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push(format!("{}: pinned fetch failed: {}", entry.id, error));
                println!("fail {:<22} unable to fetch pinned source", entry.id);
                continue;
            }
        };
        let latest_bytes = match fetch_upstream_latest(&client, &entry) {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push(format!("{}: latest fetch failed: {}", entry.id, error));
                println!("fail {:<22} unable to fetch latest source", entry.id);
                continue;
            }
        };

        let pinned_hash = hash_bytes(&pinned_bytes);
        let latest_hash = hash_bytes(&latest_bytes);

        if pinned_hash == latest_hash {
            println!("ok   {:<22} no upstream changes", entry.id);
        } else {
            changed += 1;
            println!(
                "diff {:<22} pinned {} != latest {}",
                entry.id,
                short_hash(&pinned_hash),
                short_hash(&latest_hash)
            );
        }
    }

    println!();
    if changed == 0 {
        println!("No upstream toy changes detected against pinned versions.");
    } else {
        println!(
            "Detected {} toy(s) with upstream changes; review before version bumps.",
            changed
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
        let phase = optional_u32_field(entry_table, "phase");
        let wasi_overlap = optional_string_field(entry_table, "wasi_overlap");

        entries.push(ToyEntry {
            id: id.to_string(),
            source,
            version,
            file,
            phase,
            wasi_overlap,
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
