use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManagedPathKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ManagedPathSpec {
    pub relative_path: String,
    pub kind: ManagedPathKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct InterfaceMcpConfig {
    #[serde(default)]
    pub config_candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct InterfaceSkillsConfig {
    #[serde(default)]
    pub include: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BundleTmuxPolicy {
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InterfaceClassification {
    Hitl,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct InterfaceBundle {
    pub name: String,
    pub display_name: String,
    pub classification: InterfaceClassification,
    pub command: String,
    pub detect_commands: Vec<String>,
    #[serde(default)]
    pub vendor_bootstrap: Option<String>,
    #[serde(default)]
    pub mcp: Option<InterfaceMcpConfig>,
    #[serde(default)]
    pub skills: Option<InterfaceSkillsConfig>,
    pub managed_paths: Vec<ManagedPathSpec>,
    pub tmux_policy: BundleTmuxPolicy,
    pub version: String,
}

#[derive(Debug, Deserialize)]
struct InterfaceRegistry {
    #[serde(rename = "interface")]
    interfaces: Vec<InterfaceBundle>,
}

static REGISTRY: OnceLock<Vec<InterfaceBundle>> = OnceLock::new();

const BUILTIN_INTERFACE_REGISTRY: &str = r#"
[[interface]]
name = "claude"
display_name = "Claude Code"
classification = "hitl"
command = "claude"
detect_commands = ["claude --version"]
vendor_bootstrap = "CLAUDE.md"
tmux_policy = "auto"
version = "builtin"
skills = { include = ["session-start", "session-update", "session-note", "session-end", "spec", "epistemic-beliefs"] }
managed_paths = [
  { relative_path = "AGENTS.md", kind = "file" },
  { relative_path = "CLAUDE.md", kind = "file" },
  { relative_path = ".claude", kind = "directory" },
]

[[interface]]
name = "opencode"
display_name = "OpenCode"
classification = "hitl"
command = "opencode"
detect_commands = ["opencode --version"]
tmux_policy = "auto"
version = "builtin"
skills = { include = ["session-start", "session-update", "session-note", "session-end", "spec", "epistemic-beliefs"] }
mcp = { config_candidates = [
  "~/.config/opencode/config.json",
  "~/.config/opencode/opencode.json",
  "~/.config/opencode/config.toml",
] }
managed_paths = [
  { relative_path = "AGENTS.md", kind = "file" },
  { relative_path = ".opencode", kind = "directory" },
]

[[interface]]
name = "gemini"
display_name = "Gemini CLI"
classification = "hitl"
command = "gemini"
detect_commands = ["gemini --version"]
vendor_bootstrap = "GEMINI.md"
tmux_policy = "auto"
version = "builtin"
skills = { include = ["session-start", "session-update", "session-note", "session-end", "spec", "epistemic-beliefs"] }
mcp = { config_candidates = [
  "~/.gemini/settings.json",
  "~/.config/gemini/settings.json",
  "~/.config/google-gemini/settings.json",
] }
managed_paths = [
  { relative_path = "AGENTS.md", kind = "file" },
  { relative_path = "GEMINI.md", kind = "file" },
  { relative_path = ".gemini", kind = "directory" },
]

[[interface]]
name = "pi"
display_name = "PI"
classification = "hitl"
command = "pi"
detect_commands = ["pi --version"]
tmux_policy = "auto"
version = "builtin"
skills = { include = ["session-start", "session-update", "session-note", "session-end", "spec", "epistemic-beliefs"] }
managed_paths = [
  { relative_path = "AGENTS.md", kind = "file" },
  { relative_path = ".pi", kind = "directory" },
]
"#;

pub fn builtin_registry_toml() -> &'static str {
    BUILTIN_INTERFACE_REGISTRY
}

fn registry_path() -> PathBuf {
    paths::interfaces_dir().join("registry.toml")
}

fn parse_registry(content: &str) -> Option<Vec<InterfaceBundle>> {
    let parsed: InterfaceRegistry = toml::from_str(content).ok()?;
    if parsed.interfaces.is_empty() {
        return None;
    }
    Some(parsed.interfaces)
}

fn load_registry() -> Vec<InterfaceBundle> {
    if let Ok(content) = fs::read_to_string(registry_path()) {
        if let Some(registry) = parse_registry(&content) {
            return registry;
        }
    }

    parse_registry(BUILTIN_INTERFACE_REGISTRY).unwrap_or_default()
}

pub fn interface_bundle_catalog() -> &'static [InterfaceBundle] {
    REGISTRY.get_or_init(load_registry).as_slice()
}

pub fn interface_bundle(name: &str) -> Result<&'static InterfaceBundle> {
    let normalized = name.trim().to_ascii_lowercase();
    interface_bundle_catalog()
        .iter()
        .find(|bundle| bundle.name == normalized)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unsupported Patina AI interface '{}'. Choose one of: {}.",
                name,
                supported_ai_interfaces().join(", ")
            )
        })
}

pub fn supported_ai_interfaces() -> Vec<&'static str> {
    interface_bundle_catalog()
        .iter()
        .filter(|bundle| bundle.classification == InterfaceClassification::Hitl)
        .map(|bundle| bundle.name.as_str())
        .collect()
}

pub fn is_supported_ai_interface(name: &str) -> bool {
    interface_bundle(name).is_ok()
}

pub fn canonical_interface_name(name: &str) -> Option<&'static str> {
    let normalized = name.trim().to_ascii_lowercase();
    interface_bundle_catalog()
        .iter()
        .find(|bundle| bundle.name == normalized)
        .map(|bundle| bundle.name.as_str())
}

pub fn canonicalize_required_interface(name: &str) -> Result<&'static str> {
    canonical_interface_name(name).ok_or_else(|| {
        anyhow::anyhow!(
            "Unsupported Patina AI interface '{}'. Choose one of: {}.",
            name,
            supported_ai_interfaces().join(", ")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn includes_wrapper_skills(bundle: &InterfaceBundle) -> bool {
        let Some(skills) = bundle.skills.as_ref() else {
            return false;
        };
        [
            "session-start",
            "session-update",
            "session-note",
            "session-end",
        ]
        .iter()
        .all(|required| skills.include.iter().any(|entry| entry == required))
    }

    #[test]
    fn bundle_catalog_lists_supported_interfaces() {
        let names = supported_ai_interfaces();
        assert_eq!(names, vec!["claude", "opencode", "gemini", "pi"]);
    }

    #[test]
    fn interface_resolution_is_case_insensitive() {
        assert_eq!(canonical_interface_name("Claude"), Some("claude"));
        assert_eq!(canonical_interface_name("OPENCODE"), Some("opencode"));
        assert_eq!(canonical_interface_name("Gemini"), Some("gemini"));
        assert_eq!(canonical_interface_name("PI"), Some("pi"));
        assert!(canonical_interface_name("codex").is_none());
    }

    #[test]
    fn bundle_metadata_tracks_vendor_bootstrap_and_managed_paths() {
        let claude = interface_bundle("claude").unwrap();
        assert_eq!(claude.vendor_bootstrap.as_deref(), Some("CLAUDE.md"));
        assert!(claude
            .managed_paths
            .iter()
            .any(|path| path.relative_path == ".claude"));

        let opencode = interface_bundle("opencode").unwrap();
        assert_eq!(opencode.vendor_bootstrap, None);
        assert!(opencode
            .managed_paths
            .iter()
            .any(|path| path.relative_path == "AGENTS.md"));
        assert!(opencode
            .mcp
            .as_ref()
            .map(|mcp| !mcp.config_candidates.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn builtins_declare_wrapper_skill_includes() {
        for bundle in interface_bundle_catalog() {
            assert!(
                includes_wrapper_skills(bundle),
                "bundle '{}' missing required wrapper skill includes",
                bundle.name
            );
        }
    }
}
