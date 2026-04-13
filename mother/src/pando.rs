use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PandoArgType {
    String,
    Flag,
    Int,
    Strings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PandoCommandArg {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: PandoArgType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub positional: bool,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PandoCommand {
    pub description: String,
    pub child: String,
    pub action: String,
    #[serde(default)]
    pub args: Vec<PandoCommandArg>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PandoSection {
    pub name: String,
    pub description: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PandoChild {
    pub name: String,
    /// Instance identifier for multi-instance compositions.
    /// Defaults to `name` if not set.
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PandoDeliveryPolicy {
    Required,
    BestEffort,
    DeadLetter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PandoTypedWiring {
    pub from: String,
    pub to: String,
    pub toy: String,
    #[serde(default)]
    pub delivery: Option<PandoDeliveryPolicy>,
}

impl PandoTypedWiring {
    pub fn delivery_policy(&self) -> PandoDeliveryPolicy {
        self.delivery.unwrap_or(PandoDeliveryPolicy::Required)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum PandoWiring {
    /// Legacy string wiring for handle-based pandos.
    Legacy(String),
    /// Typed toy wiring for composed pandos.
    Typed(PandoTypedWiring),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PandoEntry {
    pub child: String,
    pub toy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PandoDeadLetter {
    pub child: String,
    #[serde(default)]
    pub toy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PandoComposition {
    /// Supports both legacy string wiring and typed wiring rules.
    ///
    /// Supported shapes:
    /// - `[composition] wiring = ["a.out -> b.in"]` (legacy)
    /// - `[[composition.wiring]] from = "a"; to = "b"; toy = "patina:record/x"` (typed)
    /// - `[[composition.typed]] ...` (temporary compatibility alias)
    #[serde(default, alias = "typed")]
    pub wiring: Vec<PandoWiring>,
    /// Entry point for the composed component.
    #[serde(default)]
    pub entry: Option<PandoEntry>,
    /// Optional dead-letter destination used by `delivery = "dead-letter"` routes.
    #[serde(default, rename = "dead-letter", alias = "dead_letter")]
    pub dead_letter: Option<PandoDeadLetter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PandoManifest {
    pub pando: PandoSection,
    #[serde(default)]
    pub children: Vec<PandoChild>,
    #[serde(default)]
    pub commands: BTreeMap<String, PandoCommand>,
    #[serde(default)]
    pub composition: Option<PandoComposition>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PandoLifecycleStatus {
    Registered,
    Ready,
    Live,
    Degraded,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PandoRegistryEntry {
    pub name: String,
    pub status: PandoLifecycleStatus,
    pub commands: Vec<String>,
    pub aliases: Vec<String>,
    pub child_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PandoRegistry {
    pub pandos: Vec<PandoRegistryEntry>,
}

pub fn parse_manifest_path(path: &Path) -> Result<PandoManifest> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading pando manifest {}", path.display()))?;
    parse_manifest_str(&raw)
}

pub fn parse_manifest_str(raw: &str) -> Result<PandoManifest> {
    let manifest: PandoManifest =
        toml::from_str(raw).context("invalid pando.toml (schema mismatch)")?;

    if manifest.children.is_empty() {
        anyhow::bail!("invalid pando.toml: at least one [[children]] entry is required");
    }

    for (command_name, command) in &manifest.commands {
        let positional_count = command.args.iter().filter(|arg| arg.positional).count();
        if positional_count > 1 {
            anyhow::bail!(
                "invalid pando.toml: command '{}' has {} positional args (max 1)",
                command_name,
                positional_count
            );
        }
    }

    Ok(manifest)
}

pub fn build_registry(
    pandos_root: &Path,
    native_commands: &HashSet<String>,
    aliases: &HashMap<String, String>,
    installed_children: &HashSet<String>,
    live_children: &HashSet<String>,
) -> Result<PandoRegistry> {
    let mut entries = Vec::new();
    let mut claimed_namespaces: HashMap<String, String> = HashMap::new();

    if !pandos_root.exists() {
        return Ok(PandoRegistry::default());
    }

    let mut dirs = std::fs::read_dir(pandos_root)
        .with_context(|| format!("reading pando root {}", pandos_root.display()))?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .collect::<Vec<_>>();
    dirs.sort_by_key(|entry| entry.file_name());

    for entry in dirs {
        let dir_name = entry.file_name().to_string_lossy().to_string();
        let manifest_path = entry.path().join("pando.toml");
        if !manifest_path.exists() {
            continue;
        }

        let manifest = match parse_manifest_path(&manifest_path) {
            Ok(manifest) => manifest,
            Err(error) => {
                entries.push(PandoRegistryEntry {
                    name: dir_name,
                    status: PandoLifecycleStatus::Error,
                    commands: Vec::new(),
                    aliases: Vec::new(),
                    child_count: 0,
                    error: Some(error.to_string()),
                });
                continue;
            }
        };

        let namespace = manifest.pando.name.clone();
        if native_commands.contains(&namespace) {
            entries.push(PandoRegistryEntry {
                name: namespace.clone(),
                status: PandoLifecycleStatus::Error,
                commands: Vec::new(),
                aliases: Vec::new(),
                child_count: manifest.children.len(),
                error: Some(format!("namespace '{}' is a native command", namespace)),
            });
            continue;
        }

        if let Some(owner) = aliases.get(&namespace) {
            entries.push(PandoRegistryEntry {
                name: namespace.clone(),
                status: PandoLifecycleStatus::Error,
                commands: Vec::new(),
                aliases: Vec::new(),
                child_count: manifest.children.len(),
                error: Some(format!(
                    "namespace '{}' is an alias for pando '{}'",
                    namespace, owner
                )),
            });
            continue;
        }

        if let Some(existing_owner) = claimed_namespaces.get(&namespace) {
            entries.push(PandoRegistryEntry {
                name: namespace.clone(),
                status: PandoLifecycleStatus::Error,
                commands: Vec::new(),
                aliases: Vec::new(),
                child_count: manifest.children.len(),
                error: Some(format!(
                    "namespace '{}' already registered by pando '{}'",
                    namespace, existing_owner
                )),
            });
            continue;
        }

        claimed_namespaces.insert(namespace.clone(), namespace.clone());

        let mut commands = manifest.commands.keys().cloned().collect::<Vec<_>>();
        commands.sort();

        let installed_count = manifest
            .children
            .iter()
            .filter(|child| installed_children.contains(&child.name))
            .count();
        let live_count = manifest
            .children
            .iter()
            .filter(|child| live_children.contains(&child.name))
            .count();
        let child_count = manifest.children.len();

        let (status, error) = if installed_count == child_count {
            if live_count == child_count {
                (PandoLifecycleStatus::Live, None)
            } else if live_count > 0 {
                (
                    PandoLifecycleStatus::Degraded,
                    Some("partial child activity".to_string()),
                )
            } else {
                (PandoLifecycleStatus::Ready, None)
            }
        } else if installed_count > 0 {
            (
                PandoLifecycleStatus::Degraded,
                Some("partial child install".to_string()),
            )
        } else {
            (PandoLifecycleStatus::Registered, None)
        };

        entries.push(PandoRegistryEntry {
            name: namespace,
            status,
            commands,
            aliases: Vec::new(),
            child_count,
            error,
        });
    }

    Ok(PandoRegistry { pandos: entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_manifest() {
        let raw = r#"
[pando]
name = "slate"
description = "Spec workflow"
version = "0.1.0"

[[children]]
name = "slate-manager"

[commands.list]
description = "List specs"
child = "slate-manager"
action = "list"
args = [
  { name = "status", type = "string", required = false, description = "Filter by status" },
  { name = "json", type = "flag", description = "Output as JSON" },
]

[commands.check]
description = "Check exit criteria"
child = "slate-manager"
action = "check"
args = [
  { name = "id", type = "string", required = true, positional = true, description = "Spec ID" }
]

[composition]
wiring = ["a.out -> b.in"]
"#;

        let manifest = parse_manifest_str(raw).unwrap();
        assert_eq!(manifest.pando.name, "slate");
        assert_eq!(manifest.children.len(), 1);
        assert!(manifest.commands.contains_key("list"));
        assert_eq!(
            manifest.commands["list"].args[0].arg_type,
            PandoArgType::String
        );
        assert_eq!(
            manifest.commands["check"].args[0].arg_type,
            PandoArgType::String
        );
        assert_eq!(
            manifest.composition.unwrap().wiring,
            vec![PandoWiring::Legacy("a.out -> b.in".to_string())]
        );
    }

    #[test]
    fn parses_typed_wiring_under_composition_wiring() {
        let raw = r#"
[pando]
name = "typed"
description = "Typed composition"
version = "0.2.0"

[[children]]
name = "schema-enforcer"

[[children]]
name = "dedup-filter"

[composition]
entry = { child = "schema-enforcer", toy = "patina:record/transform" }

[[composition.wiring]]
from = "schema-enforcer"
to = "dedup-filter"
toy = "patina:record/transform"
"#;

        let manifest = parse_manifest_str(raw).unwrap();
        let composition = manifest.composition.unwrap();
        assert_eq!(
            composition.entry,
            Some(PandoEntry {
                child: "schema-enforcer".to_string(),
                toy: "patina:record/transform".to_string(),
            })
        );
        assert_eq!(
            composition.wiring,
            vec![PandoWiring::Typed(PandoTypedWiring {
                from: "schema-enforcer".to_string(),
                to: "dedup-filter".to_string(),
                toy: "patina:record/transform".to_string(),
                delivery: None,
            })]
        );
    }

    #[test]
    fn parses_typed_wiring_under_composition_typed_alias() {
        let raw = r#"
[pando]
name = "typed-alias"
description = "Typed composition alias"
version = "0.2.0"

[[children]]
name = "schema-enforcer"

[[children]]
name = "dedup-filter"

[composition]

[[composition.typed]]
from = "schema-enforcer"
to = "dedup-filter"
toy = "patina:record/transform"
"#;

        let manifest = parse_manifest_str(raw).unwrap();
        assert_eq!(
            manifest.composition.unwrap().wiring,
            vec![PandoWiring::Typed(PandoTypedWiring {
                from: "schema-enforcer".to_string(),
                to: "dedup-filter".to_string(),
                toy: "patina:record/transform".to_string(),
                delivery: None,
            })]
        );
    }

    #[test]
    fn parses_typed_wiring_delivery_policy() {
        let raw = r#"
[pando]
name = "typed-delivery"
description = "Typed composition with delivery policy"
version = "0.2.0"

[[children]]
name = "schema-enforcer"

[[children]]
name = "dedup-filter"

[composition]

[[composition.wiring]]
from = "schema-enforcer"
to = "dedup-filter"
toy = "patina:record/transform"
delivery = "best-effort"
"#;

        let manifest = parse_manifest_str(raw).unwrap();
        let composition = manifest.composition.unwrap();
        let typed = match &composition.wiring[0] {
            PandoWiring::Typed(typed) => typed,
            PandoWiring::Legacy(_) => panic!("expected typed wiring"),
        };
        assert_eq!(
            typed.delivery,
            Some(PandoDeliveryPolicy::BestEffort),
            "delivery policy should parse from wiring"
        );
        assert_eq!(typed.delivery_policy(), PandoDeliveryPolicy::BestEffort);
    }

    #[test]
    fn parses_composition_dead_letter_target() {
        let raw = r#"
[pando]
name = "typed-dead-letter"
description = "Typed composition with dead-letter"
version = "0.2.0"

[[children]]
name = "schema-enforcer"
id = "se"

[[children]]
name = "watch-null-sink"
id = "dlq"

[composition]

[composition.dead-letter]
child = "dlq"
toy = "patina:watch/events"
"#;

        let manifest = parse_manifest_str(raw).unwrap();
        let composition = manifest.composition.unwrap();
        let dead_letter = composition.dead_letter.expect("dead-letter should parse");
        assert_eq!(dead_letter.child, "dlq");
        assert_eq!(dead_letter.toy.as_deref(), Some("patina:watch/events"));
    }

    #[test]
    fn rejects_missing_required_fields() {
        let raw = r#"
[pando]
description = "Missing name"
version = "0.1.0"

[[children]]
name = "slate-manager"
"#;

        let err = parse_manifest_str(raw).unwrap_err();
        assert!(err.to_string().contains("schema mismatch"));
    }

    #[test]
    fn rejects_unknown_fields() {
        let raw = r#"
[pando]
name = "slate"
description = "Spec workflow"
version = "0.1.0"
owner = "team"

[[children]]
name = "slate-manager"
"#;

        let err = parse_manifest_str(raw).unwrap_err();
        assert!(err
            .chain()
            .any(|cause| cause.to_string().contains("unknown field")));
    }

    #[test]
    fn registry_rejects_native_and_alias_collisions() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        std::fs::create_dir_all(root.join("native-collision")).unwrap();
        std::fs::write(
            root.join("native-collision/pando.toml"),
            r#"
[pando]
name = "init"
description = "collides with native"
version = "0.1.0"

[[children]]
name = "slate-manager"
"#,
        )
        .unwrap();

        std::fs::create_dir_all(root.join("alias-collision")).unwrap();
        std::fs::write(
            root.join("alias-collision/pando.toml"),
            r#"
[pando]
name = "spec"
description = "collides with alias"
version = "0.1.0"

[[children]]
name = "slate-manager"
"#,
        )
        .unwrap();

        let native = HashSet::from(["init".to_string()]);
        let aliases = HashMap::from([("spec".to_string(), "slate".to_string())]);
        let installed = HashSet::from(["slate-manager".to_string()]);

        let registry = build_registry(root, &native, &aliases, &installed, &installed).unwrap();
        assert_eq!(registry.pandos.len(), 2);
        assert!(registry
            .pandos
            .iter()
            .any(|entry| entry.error.as_deref() == Some("namespace 'init' is a native command")));
        assert!(registry.pandos.iter().any(|entry| {
            entry.error.as_deref() == Some("namespace 'spec' is an alias for pando 'slate'")
        }));
    }

    #[test]
    fn registry_tracks_lifecycle_states() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        std::fs::create_dir_all(root.join("live")).unwrap();
        std::fs::write(
            root.join("live/pando.toml"),
            r#"
[pando]
name = "live"
description = "all children available"
version = "0.1.0"

[[children]]
name = "a"
"#,
        )
        .unwrap();

        std::fs::create_dir_all(root.join("ready")).unwrap();
        std::fs::write(
            root.join("ready/pando.toml"),
            r#"
[pando]
name = "ready"
description = "children installed but not active"
version = "0.1.0"

[[children]]
name = "ready-a"
"#,
        )
        .unwrap();

        std::fs::create_dir_all(root.join("degraded")).unwrap();
        std::fs::write(
            root.join("degraded/pando.toml"),
            r#"
[pando]
name = "degraded"
description = "partial child availability"
version = "0.1.0"

[[children]]
name = "a"

[[children]]
name = "b"
"#,
        )
        .unwrap();

        std::fs::create_dir_all(root.join("registered")).unwrap();
        std::fs::write(
            root.join("registered/pando.toml"),
            r#"
[pando]
name = "registered"
description = "no children available"
version = "0.1.0"

[[children]]
name = "missing"
"#,
        )
        .unwrap();

        let registry = build_registry(
            root,
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::from(["a".to_string(), "ready-a".to_string()]),
            &HashSet::from(["a".to_string()]),
        )
        .unwrap();

        let live = registry
            .pandos
            .iter()
            .find(|entry| entry.name == "live")
            .unwrap();
        assert_eq!(live.status, PandoLifecycleStatus::Live);

        let ready = registry
            .pandos
            .iter()
            .find(|entry| entry.name == "ready")
            .unwrap();
        assert_eq!(ready.status, PandoLifecycleStatus::Ready);

        let degraded = registry
            .pandos
            .iter()
            .find(|entry| entry.name == "degraded")
            .unwrap();
        assert_eq!(degraded.status, PandoLifecycleStatus::Degraded);

        let registered = registry
            .pandos
            .iter()
            .find(|entry| entry.name == "registered")
            .unwrap();
        assert_eq!(registered.status, PandoLifecycleStatus::Registered);
    }
}
