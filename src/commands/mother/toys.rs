use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

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
