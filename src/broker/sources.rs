//! Sources.toml reader — declarative source configuration per project.
//!
//! Each project may have a `.patina/sources.toml` declaring what external
//! data it wants. Mother reads these across all registered projects.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A single source entry from sources.toml.
#[derive(Debug, Clone)]
pub struct SourceEntry {
    pub name: String,
    pub connection: String,
    pub params: HashMap<String, toml::Value>,
    pub types: Vec<String>,
    pub schedule: String,
}

/// All sources for a single project.
#[derive(Debug, Clone)]
pub struct ProjectSources {
    pub project_root: PathBuf,
    pub sources: Vec<SourceEntry>,
}

/// Raw TOML structure.
#[derive(Deserialize)]
struct RawSourcesFile {
    #[serde(default)]
    sources: HashMap<String, RawSourceEntry>,
}

#[derive(Deserialize)]
struct RawSourceEntry {
    connection: String,
    #[serde(default)]
    params: HashMap<String, toml::Value>,
    #[serde(default)]
    types: Vec<String>,
    #[serde(default = "default_schedule")]
    schedule: String,
}

fn default_schedule() -> String {
    "manual".to_string()
}

/// Load sources.toml from a specific project root.
///
/// Returns Ok(None) if the file doesn't exist (project has no sources).
pub fn load_project_sources(project_root: &Path) -> Result<Option<ProjectSources>> {
    let sources_path = project_root.join(".patina").join("sources.toml");

    if !sources_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&sources_path)
        .with_context(|| format!("reading {}", sources_path.display()))?;

    let entries = parse_sources(&content)?;

    Ok(Some(ProjectSources {
        project_root: project_root.to_path_buf(),
        sources: entries,
    }))
}

/// Find a specific source by name across a project's sources.toml.
pub fn find_source(project_root: &Path, source_name: &str) -> Result<Option<SourceEntry>> {
    let project_sources = load_project_sources(project_root)?;
    Ok(project_sources.and_then(|ps| ps.sources.into_iter().find(|s| s.name == source_name)))
}

/// Scan all registered projects for sources.toml entries.
pub fn scan_all_sources() -> Result<Vec<ProjectSources>> {
    let registry_path = crate::paths::registry_path();

    if !registry_path.exists() {
        return Ok(vec![]);
    }

    let content =
        std::fs::read_to_string(&registry_path).with_context(|| "reading registry.yaml")?;

    // Parse registry to get project paths
    let registry: serde_yaml::Value =
        serde_yaml::from_str(&content).with_context(|| "parsing registry.yaml")?;

    let mut all_sources = Vec::new();

    if let Some(projects) = registry.get("projects").and_then(|p| p.as_mapping()) {
        for (_name, entry) in projects {
            if let Some(path_str) = entry.get("path").and_then(|p| p.as_str()) {
                let project_root = PathBuf::from(path_str);
                match load_project_sources(&project_root) {
                    Ok(Some(ps)) if !ps.sources.is_empty() => {
                        all_sources.push(ps);
                    }
                    Ok(_) => {} // No sources.toml or empty
                    Err(e) => {
                        eprintln!(
                            "[broker] warning: failed to read sources from {}: {}",
                            project_root.display(),
                            e
                        );
                    }
                }
            }
        }
    }

    Ok(all_sources)
}

/// Parse sources.toml content into SourceEntry list.
fn parse_sources(content: &str) -> Result<Vec<SourceEntry>> {
    let raw: RawSourcesFile = toml::from_str(content).with_context(|| "parsing sources.toml")?;

    let mut entries: Vec<SourceEntry> = raw
        .sources
        .into_iter()
        .map(|(name, raw)| SourceEntry {
            name,
            connection: raw.connection,
            params: raw.params,
            types: raw.types,
            schedule: raw.schedule,
        })
        .collect();

    // Sort by name for deterministic ordering
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_sources() {
        let toml = r#"
[sources.github]
connection = "github"
params = { owner = "NicabarNimble", repo = "patina" }
types = ["issues", "prs"]
schedule = "on-scrape"

[sources.github-docs]
connection = "github"
params = { owner = "NicabarNimble", repo = "docs" }
types = ["issues"]
schedule = "daily"
"#;
        let entries = parse_sources(toml).unwrap();
        assert_eq!(entries.len(), 2);

        let github = entries.iter().find(|e| e.name == "github").unwrap();
        assert_eq!(github.connection, "github");
        assert_eq!(github.schedule, "on-scrape");
        assert_eq!(github.types, vec!["issues", "prs"]);
        assert_eq!(
            github.params.get("owner").and_then(|v| v.as_str()),
            Some("NicabarNimble")
        );

        let docs = entries.iter().find(|e| e.name == "github-docs").unwrap();
        assert_eq!(docs.connection, "github");
        assert_eq!(docs.schedule, "daily");
    }

    #[test]
    fn parse_minimal_source() {
        let toml = r#"
[sources.test]
connection = "test-conn"
"#;
        let entries = parse_sources(toml).unwrap();
        assert_eq!(entries.len(), 1);

        let test = &entries[0];
        assert_eq!(test.name, "test");
        assert_eq!(test.connection, "test-conn");
        assert_eq!(test.schedule, "manual"); // default
        assert!(test.types.is_empty());
        assert!(test.params.is_empty());
    }

    #[test]
    fn parse_empty_sources() {
        let toml = "";
        let entries = parse_sources(toml).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn load_nonexistent_project() {
        let result = load_project_sources(Path::new("/nonexistent/project")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn parse_missing_connection_errors() {
        let toml = r#"
[sources.bad]
schedule = "daily"
"#;
        let result = parse_sources(toml);
        assert!(result.is_err());
    }
}
