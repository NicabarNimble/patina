//! Sources.toml reader — declarative source configuration per project.
//!
//! Each project may have a `.patina/sources.toml` declaring what external
//! data it wants. Mother reads these across all registered projects.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Where a source's facts are routed after validation.
///
/// Sources without a destination default to Project (backward compat).
/// Lake destinations route facts to a lakehouse child via pipe/ingest.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Destination {
    /// Write to the project's events.db (current behavior, default).
    #[default]
    Project,
    /// Route to a named lake via pipe/ingest to lakehouse child.
    Lake { name: String },
}

/// A single source entry from sources.toml.
#[derive(Debug, Clone)]
pub struct SourceEntry {
    pub name: String,
    pub connection: String,
    pub params: HashMap<String, toml::Value>,
    pub types: Vec<String>,
    pub schedule: String,
    pub destination: Destination,
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
    #[serde(default)]
    destination: Option<RawDestination>,
}

#[derive(Deserialize)]
struct RawDestination {
    #[serde(rename = "type")]
    dest_type: String,
    /// Lake name (required when type = "lake").
    lake: Option<String>,
}

fn default_schedule() -> String {
    "manual".to_string()
}

fn parse_destination(raw: Option<RawDestination>, source_name: &str) -> Result<Destination> {
    match raw {
        None => Ok(Destination::Project),
        Some(dest) => match dest.dest_type.as_str() {
            "project" => Ok(Destination::Project),
            "lake" => {
                let name = dest.lake.ok_or_else(|| {
                    anyhow::anyhow!(
                        "source '{}': destination type 'lake' requires a 'lake' field",
                        source_name
                    )
                })?;
                Ok(Destination::Lake { name })
            }
            other => {
                anyhow::bail!(
                    "source '{}': unknown destination type '{}' (expected 'project' or 'lake')",
                    source_name,
                    other
                )
            }
        },
    }
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
pub fn scan_all_sources(registry_path: &Path) -> Result<Vec<ProjectSources>> {
    let registry_path = registry_path.to_path_buf();

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

    let mut entries: Vec<SourceEntry> = Vec::new();
    for (name, raw) in raw.sources {
        let destination = parse_destination(raw.destination, &name)?;
        entries.push(SourceEntry {
            name,
            connection: raw.connection,
            params: raw.params,
            types: raw.types,
            schedule: raw.schedule,
            destination,
        });
    }

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

    #[test]
    fn parse_source_no_destination_defaults_to_project() {
        let toml = r#"
[sources.github]
connection = "github"
types = ["issues"]
"#;
        let entries = parse_sources(toml).unwrap();
        assert_eq!(entries[0].destination, Destination::Project);
    }

    #[test]
    fn parse_source_explicit_project_destination() {
        let toml = r#"
[sources.github]
connection = "github"
types = ["issues"]

[sources.github.destination]
type = "project"
"#;
        let entries = parse_sources(toml).unwrap();
        assert_eq!(entries[0].destination, Destination::Project);
    }

    #[test]
    fn parse_source_lake_destination() {
        let toml = r#"
[sources.github-lake]
connection = "github"
types = ["issues", "prs"]
schedule = "hourly"

[sources.github-lake.destination]
type = "lake"
lake = "github-data"
"#;
        let entries = parse_sources(toml).unwrap();
        assert_eq!(
            entries[0].destination,
            Destination::Lake {
                name: "github-data".to_string()
            }
        );
    }

    #[test]
    fn parse_lake_destination_missing_name_errors() {
        let toml = r#"
[sources.bad]
connection = "github"

[sources.bad.destination]
type = "lake"
"#;
        let result = parse_sources(toml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("lake"));
    }

    #[test]
    fn parse_unknown_destination_type_errors() {
        let toml = r#"
[sources.bad]
connection = "github"

[sources.bad.destination]
type = "bucket"
"#;
        let result = parse_sources(toml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unknown destination type"));
    }

    #[test]
    fn parse_mixed_destinations() {
        let toml = r#"
[sources.github]
connection = "github"
types = ["issues"]

[sources.github-lake]
connection = "github"
types = ["issues", "prs"]

[sources.github-lake.destination]
type = "lake"
lake = "org-lake"
"#;
        let entries = parse_sources(toml).unwrap();
        assert_eq!(entries.len(), 2);

        let project = entries.iter().find(|e| e.name == "github").unwrap();
        assert_eq!(project.destination, Destination::Project);

        let lake = entries.iter().find(|e| e.name == "github-lake").unwrap();
        assert_eq!(
            lake.destination,
            Destination::Lake {
                name: "org-lake".to_string()
            }
        );
    }
}
