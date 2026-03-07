//! Connection config reader for ~/.patina/connections/{name}.toml
//!
//! Supports both rich (from patina connect) and minimal (hand-authored) formats.
//! Mother reads only the [connection] section; other sections are opaque.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Parsed connection config — the [connection] section only.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub provider: String,
    pub credential: String,
    pub child: String,
    pub name: Option<String>,
}

/// Raw TOML structure for version-aware parsing.
#[derive(Deserialize)]
struct RawConnectionFile {
    schema_version: Option<u64>,
    connection: Option<ConnectionSection>,
}

#[derive(Deserialize)]
struct ConnectionSection {
    #[serde(default)]
    name: Option<String>,
    provider: String,
    credential: String,
    child: String,
}

/// Load a connection config by name from ~/.patina/connections/{name}.toml
pub fn load_connection(name: &str) -> Result<ConnectionConfig> {
    let path = connection_path(name);
    load_connection_from(&path, name)
}

/// Resolve the path to a connection config file.
fn connection_path(name: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".patina")
        .join("connections")
        .join(format!("{}.toml", name))
}

/// Load and parse a connection config from a specific path.
fn load_connection_from(path: &Path, name: &str) -> Result<ConnectionConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading connection config '{}'", name))?;

    parse_connection(&content, name)
}

/// Parse connection config TOML with schema_version validation.
fn parse_connection(content: &str, name: &str) -> Result<ConnectionConfig> {
    let raw: RawConnectionFile =
        toml::from_str(content).with_context(|| format!("parsing connection config '{}'", name))?;

    // Validate schema_version (DESIGN.md §8)
    match raw.schema_version {
        Some(0) => {} // Full support
        None => bail!("connection config '{}' missing schema_version", name),
        Some(v) => {
            eprintln!(
                "[broker] connection '{}': schema_version {} newer than supported (0), attempting read",
                name, v
            );
        }
    }

    let section = raw
        .connection
        .with_context(|| format!("connection config '{}' missing [connection] section", name))?;

    Ok(ConnectionConfig {
        provider: section.provider,
        credential: section.credential,
        child: section.child,
        name: section.name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rich_format() {
        let toml = r#"
schema_version = 0

[connection]
name = "github"
provider = "github"
credential = "github:user"
child = "github-connector"
created = "2026-03-06T00:00:00Z"

[oauth]
client_id = "Iv1.xxxxxxxx"
scopes = ["repo", "read:org"]
"#;
        let config = parse_connection(toml, "github").unwrap();
        assert_eq!(config.provider, "github");
        assert_eq!(config.credential, "github:user");
        assert_eq!(config.child, "github-connector");
        assert_eq!(config.name.as_deref(), Some("github"));
    }

    #[test]
    fn parse_minimal_format() {
        let toml = r#"
schema_version = 0

[connection]
provider = "github"
credential = "github-token"
child = "github-connector"
"#;
        let config = parse_connection(toml, "test").unwrap();
        assert_eq!(config.provider, "github");
        assert_eq!(config.credential, "github-token");
        assert_eq!(config.child, "github-connector");
        assert!(config.name.is_none());
    }

    #[test]
    fn missing_schema_version_errors() {
        let toml = r#"
[connection]
provider = "github"
credential = "github-token"
child = "github-connector"
"#;
        let result = parse_connection(toml, "test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing schema_version"));
    }

    #[test]
    fn unknown_schema_version_warns_and_attempts() {
        let toml = r#"
schema_version = 5

[connection]
provider = "github"
credential = "github-token"
child = "github-connector"
"#;
        // Should succeed with a warning (printed to stderr)
        let config = parse_connection(toml, "test").unwrap();
        assert_eq!(config.provider, "github");
    }

    #[test]
    fn missing_connection_section_errors() {
        let toml = r#"
schema_version = 0

[oauth]
client_id = "test"
"#;
        let result = parse_connection(toml, "test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("[connection] section"));
    }

    #[test]
    fn missing_required_field_errors() {
        let toml = r#"
schema_version = 0

[connection]
provider = "github"
credential = "github-token"
"#;
        // Missing 'child' field
        let result = parse_connection(toml, "test");
        assert!(result.is_err());
    }
}
