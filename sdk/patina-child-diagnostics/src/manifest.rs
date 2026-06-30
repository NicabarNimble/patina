use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use patina_sdk::manifest::{ChildManifest, ChildManifestError, CHILD_MANIFEST_FILE};
use toml::Table;

use crate::report::{DiagnosticFinding, DiagnosticPhase};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestInfo {
    pub path: PathBuf,
    pub name: Option<String>,
    pub version: Option<String>,
    pub declared_toys: BTreeSet<String>,
}

pub fn check_manifest(root: &Path) -> Result<(Option<ManifestInfo>, Vec<DiagnosticFinding>)> {
    let manifest_path = root.join(CHILD_MANIFEST_FILE);
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

    let sdk_manifest = match ChildManifest::from_toml_str(&content) {
        Ok(manifest) => Some(manifest),
        Err(error) => {
            push_sdk_manifest_error(&manifest_path, &error, &mut findings);
            None
        }
    };

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
    let declared_toys = sdk_manifest
        .as_ref()
        .map(|manifest| manifest.needs.toys.iter().cloned().collect::<BTreeSet<_>>())
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

    check_filesystem_scope_policy(needs, &manifest_path, &mut findings);

    Ok((
        Some(ManifestInfo {
            path: manifest_path,
            name: sdk_manifest.as_ref().map(|manifest| manifest.name.clone()),
            version: sdk_manifest
                .as_ref()
                .map(|manifest| manifest.version.clone()),
            declared_toys,
        }),
        findings,
    ))
}

fn push_sdk_manifest_error(
    manifest_path: &Path,
    error: &ChildManifestError,
    findings: &mut Vec<DiagnosticFinding>,
) {
    let (code, message, remediation) = match error {
        ChildManifestError::ParseToml(error) => (
            "PTN-MANIFEST-000",
            "child.toml could not be parsed".to_string(),
            format!("fix TOML syntax: {error}"),
        ),
        ChildManifestError::MissingSection("child") => (
            "PTN-MANIFEST-008",
            "child.toml is missing [child] identity section".to_string(),
            "add [child] with name, version, and kind fields".to_string(),
        ),
        ChildManifestError::MissingRequiredField("child.name")
        | ChildManifestError::EmptyStringField("child.name") => (
            "PTN-MANIFEST-002",
            "child.toml is missing child.name".to_string(),
            "set [child].name to the stable package identity used by releases and Mother discovery"
                .to_string(),
        ),
        ChildManifestError::MissingRequiredField("child.version")
        | ChildManifestError::EmptyStringField("child.version") => (
            "PTN-MANIFEST-003",
            "child.toml is missing child.version".to_string(),
            "set [child].version and keep it aligned with release tags and published assets"
                .to_string(),
        ),
        ChildManifestError::MissingRequiredField("child.kind")
        | ChildManifestError::EmptyStringField("child.kind") => (
            "PTN-MANIFEST-004",
            "child.toml is missing child.kind".to_string(),
            "set [child].kind using child terminology; reserve world terminology for WIT composition"
                .to_string(),
        ),
        ChildManifestError::InvalidIngressMode(_) => (
            "PTN-MANIFEST-011",
            "child.toml has an unsupported child.ingress.mode".to_string(),
            "set [child.ingress].mode to handle, hybrid, or wit-only".to_string(),
        ),
        other => (
            "PTN-MANIFEST-012",
            "child.toml violates the SDK child manifest contract".to_string(),
            format!("fix the manifest contract error: {other}"),
        ),
    };

    findings.push(DiagnosticFinding::error(
        DiagnosticPhase::Manifest,
        code,
        Some(manifest_path.to_path_buf()),
        message,
        Some(remediation),
    ));
}

fn check_filesystem_scope_policy(
    needs: Option<&Table>,
    manifest_path: &Path,
    findings: &mut Vec<DiagnosticFinding>,
) {
    let Some(filesystem) = needs
        .and_then(|needs| needs.get("scopes"))
        .and_then(|value| value.as_table())
        .and_then(|scopes| scopes.get("filesystem"))
        .and_then(|value| value.as_table())
    else {
        return;
    };

    if let Some(path) = filesystem
        .get("path")
        .and_then(|value| value.as_str())
        .map(str::trim)
    {
        warn_on_filesystem_path("filesystem", path, manifest_path, findings);
    }

    for (scope_name, scope_value) in filesystem {
        let Some(scope_table) = scope_value.as_table() else {
            continue;
        };
        let path = scope_table
            .get("path")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .unwrap_or_default();

        if scope_name == "project" {
            findings.push(DiagnosticFinding::warning(
                DiagnosticPhase::Manifest,
                "PTN-MANIFEST-010",
                Some(manifest_path.to_path_buf()),
                "child.toml declares a project filesystem mount scope",
                Some(
                    "do not hard-code project mounts in release child.toml; Patina/Mother resolves the host project and mounts it as `/project` at runtime"
                        .to_string(),
                ),
            ));
        }

        warn_on_filesystem_path(scope_name, path, manifest_path, findings);
    }
}

fn warn_on_filesystem_path(
    scope_name: &str,
    path: &str,
    manifest_path: &Path,
    findings: &mut Vec<DiagnosticFinding>,
) {
    if path == "/" {
        findings.push(DiagnosticFinding::warning(
            DiagnosticPhase::Manifest,
            "PTN-MANIFEST-009",
            Some(manifest_path.to_path_buf()),
            format!("filesystem scope `{scope_name}` grants broad guest root access"),
            Some(
                "avoid broad `path = \"/\"`; projectful child invocations receive the host project mounted as `/project` by Patina/Mother"
                    .to_string(),
            ),
        ));
    } else if path == "/project" {
        findings.push(DiagnosticFinding::warning(
            DiagnosticPhase::Manifest,
            "PTN-MANIFEST-010",
            Some(manifest_path.to_path_buf()),
            format!("filesystem scope `{scope_name}` declares the runtime project mount path"),
            Some(
                "`/project` is runner-owned; request the `filesystem` toy and let Patina/Mother mount the resolved host project at `/project`"
                    .to_string(),
            ),
        ));
    }
}
