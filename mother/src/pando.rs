use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PandoComposition {
    #[serde(default)]
    pub wiring: Vec<String>,
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
}
