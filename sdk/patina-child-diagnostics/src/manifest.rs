use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use toml::Table;

use crate::report::{DiagnosticFinding, DiagnosticPhase};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestInfo {
    pub path: PathBuf,
    pub declared_toys: BTreeSet<String>,
}

pub fn check_manifest(root: &Path) -> Result<(Option<ManifestInfo>, Vec<DiagnosticFinding>)> {
    let manifest_path = root.join("child.toml");
    let mut findings = Vec::new();

    if !manifest_path.exists() {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Manifest,
            "PTN-MANIFEST-001",
            Some(manifest_path),
            "push-pure child package is missing child.toml",
            Some(
                "add child.toml with [child] identity and [needs].toys policy declarations"
                    .to_string(),
            ),
        ));
        return Ok((None, findings));
    }

    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let table = match content.parse::<Table>() {
        Ok(table) => table,
        Err(error) => {
            findings.push(DiagnosticFinding::error(
                DiagnosticPhase::Manifest,
                "PTN-MANIFEST-000",
                Some(manifest_path),
                "child.toml could not be parsed",
                Some(format!("fix TOML syntax: {error}")),
            ));
            return Ok((None, findings));
        }
    };

    let child = table.get("child").and_then(|value| value.as_table());
    if child.is_none() {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Manifest,
            "PTN-MANIFEST-008",
            Some(manifest_path.clone()),
            "child.toml is missing [child] identity section",
            Some("add [child] with name, version, and kind fields".to_string()),
        ));
    }

    let child_name = child
        .and_then(|child| child.get("name"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if child_name.is_none() {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Manifest,
            "PTN-MANIFEST-002",
            Some(manifest_path.clone()),
            "child.toml is missing child.name",
            Some(
                "set [child].name to the stable package identity used by releases and Mother discovery"
                    .to_string(),
            ),
        ));
    }

    let child_version = child
        .and_then(|child| child.get("version"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if child_version.is_none() {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Manifest,
            "PTN-MANIFEST-003",
            Some(manifest_path.clone()),
            "child.toml is missing child.version",
            Some(
                "set [child].version and keep it aligned with release tags and published assets"
                    .to_string(),
            ),
        ));
    }

    let child_kind = child
        .and_then(|child| child.get("kind"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if child_kind.is_none() {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Manifest,
            "PTN-MANIFEST-004",
            Some(manifest_path.clone()),
            "child.toml is missing child.kind",
            Some(
                "set [child].kind using child terminology; reserve world terminology for WIT composition"
                    .to_string(),
            ),
        ));
    }

    if table.contains_key("capabilities") {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Manifest,
            "PTN-MANIFEST-005",
            Some(manifest_path.clone()),
            "child.toml uses legacy [capabilities] declarations",
            Some("move authority requests to [needs].toys and optional [needs.scopes]".to_string()),
        ));
    }

    if table.contains_key("toys") {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Manifest,
            "PTN-MANIFEST-006",
            Some(manifest_path.clone()),
            "child.toml uses legacy top-level [toys] declarations",
            Some("declare toys under [needs] as toys = [...]".to_string()),
        ));
    }

    let needs = table.get("needs").and_then(|value| value.as_table());
    let needs_toys = needs.and_then(|needs| needs.get("toys"));
    let declared_toys = needs_toys
        .and_then(|value| value.as_array())
        .map(|array| {
            array
                .iter()
                .filter_map(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();

    if needs_toys.is_none() {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Manifest,
            "PTN-MANIFEST-007",
            Some(manifest_path.clone()),
            "child.toml does not declare [needs].toys",
            Some(
                "declare [needs].toys, using an empty list only when the component imports no host toy interfaces"
                    .to_string(),
            ),
        ));
    } else if needs_toys.is_some_and(|value| !value.is_array()) {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Manifest,
            "PTN-MANIFEST-007",
            Some(manifest_path.clone()),
            "child.toml [needs].toys must be an array",
            Some("declare [needs].toys as toys = [\"logging\", ...]".to_string()),
        ));
    }

    Ok((
        Some(ManifestInfo {
            path: manifest_path,
            declared_toys,
        }),
        findings,
    ))
}
