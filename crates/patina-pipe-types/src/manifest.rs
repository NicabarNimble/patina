use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed representation of a `child.toml` manifest for native children.
///
/// WASM children use `plugin.toml` (existing loader unchanged). Native
/// children use `child.toml`. Mother's manifest loader reads the top-level
/// section name to determine format: `[child]` = native, `[plugin]` = WASM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildManifest {
    pub child: ChildSection,
    #[serde(default)]
    pub capabilities: Option<CapabilitiesSection>,
    #[serde(default)]
    pub domains: Option<DomainsSection>,
    #[serde(default)]
    pub auth: Option<AuthSection>,
    #[serde(default)]
    pub schemas: HashMap<String, SchemaEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildSection {
    pub name: String,
    pub version: String,
    #[serde(rename = "type")]
    pub child_type: ChildType,
    pub runtime: ChildRuntime,
    pub lifecycle: ChildLifecycle,
    #[serde(default)]
    pub description: Option<String>,
}

/// What kind of child this is — determines routing and lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChildType {
    Connector,
    Transport,
    Lakehouse,
    Transform,
}

/// How the child runs — determines spawn and communication method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChildRuntime {
    Native,
    Wasm,
}

/// How Mother manages the child's lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChildLifecycle {
    Poll,
    Stream,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesSection {
    #[serde(default)]
    pub data_types: Vec<String>,
    #[serde(default)]
    pub supports_incremental: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainsSection {
    #[serde(default)]
    pub allowed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSection {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEntry {
    pub package: String,
}

impl ChildManifest {
    /// Parse a child.toml manifest from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Serialize back to TOML string.
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_MANIFEST: &str = r#"
[child]
name = "github-connector"
version = "0.1.0"
type = "connector"
runtime = "native"
lifecycle = "poll"

[capabilities]
data_types = ["issues", "prs"]
supports_incremental = true

[domains]
allowed = ["api.github.com"]

[auth]
required = true
provider = "github"

[schemas.github]
package = "patina:schema/github@1.0.0"
"#;

    #[test]
    fn parse_valid_manifest() {
        let manifest = ChildManifest::from_toml(VALID_MANIFEST).unwrap();
        assert_eq!(manifest.child.name, "github-connector");
        assert_eq!(manifest.child.version, "0.1.0");
        assert_eq!(manifest.child.child_type, ChildType::Connector);
        assert_eq!(manifest.child.runtime, ChildRuntime::Native);
        assert_eq!(manifest.child.lifecycle, ChildLifecycle::Poll);

        let caps = manifest.capabilities.unwrap();
        assert_eq!(caps.data_types, vec!["issues", "prs"]);
        assert!(caps.supports_incremental);

        let domains = manifest.domains.unwrap();
        assert_eq!(domains.allowed, vec!["api.github.com"]);

        let auth = manifest.auth.unwrap();
        assert!(auth.required);
        assert_eq!(auth.provider.as_deref(), Some("github"));

        assert!(manifest.schemas.contains_key("github"));
        assert_eq!(
            manifest.schemas["github"].package,
            "patina:schema/github@1.0.0"
        );
    }

    #[test]
    fn round_trip_parse_serialize_parse() {
        let manifest1 = ChildManifest::from_toml(VALID_MANIFEST).unwrap();
        let serialized = manifest1.to_toml().unwrap();
        let manifest2 = ChildManifest::from_toml(&serialized).unwrap();

        assert_eq!(manifest1.child.name, manifest2.child.name);
        assert_eq!(manifest1.child.version, manifest2.child.version);
        assert_eq!(manifest1.child.child_type, manifest2.child.child_type);
        assert_eq!(manifest1.child.runtime, manifest2.child.runtime);
        assert_eq!(manifest1.child.lifecycle, manifest2.child.lifecycle);
    }

    #[test]
    fn missing_required_fields_fails() {
        // Missing name
        let bad = r#"
[child]
version = "0.1.0"
type = "connector"
runtime = "native"
lifecycle = "poll"
"#;
        assert!(ChildManifest::from_toml(bad).is_err());
    }

    #[test]
    fn missing_child_section_fails() {
        let bad = r#"
[capabilities]
data_types = ["issues"]
"#;
        assert!(ChildManifest::from_toml(bad).is_err());
    }

    #[test]
    fn invalid_type_fails() {
        let bad = r#"
[child]
name = "test"
version = "0.1.0"
type = "unknown"
runtime = "native"
lifecycle = "poll"
"#;
        assert!(ChildManifest::from_toml(bad).is_err());
    }

    #[test]
    fn invalid_runtime_fails() {
        let bad = r#"
[child]
name = "test"
version = "0.1.0"
type = "connector"
runtime = "docker"
lifecycle = "poll"
"#;
        assert!(ChildManifest::from_toml(bad).is_err());
    }

    #[test]
    fn minimal_manifest() {
        let minimal = r#"
[child]
name = "simple"
version = "0.1.0"
type = "transform"
runtime = "wasm"
lifecycle = "manual"
"#;
        let manifest = ChildManifest::from_toml(minimal).unwrap();
        assert_eq!(manifest.child.name, "simple");
        assert_eq!(manifest.child.child_type, ChildType::Transform);
        assert_eq!(manifest.child.runtime, ChildRuntime::Wasm);
        assert_eq!(manifest.child.lifecycle, ChildLifecycle::Manual);
        assert!(manifest.capabilities.is_none());
        assert!(manifest.domains.is_none());
        assert!(manifest.auth.is_none());
        assert!(manifest.schemas.is_empty());
    }
}
