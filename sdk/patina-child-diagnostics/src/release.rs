use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use toml::Table;

use crate::manifest::ManifestInfo;
use crate::report::{DiagnosticFinding, DiagnosticPhase};

pub(crate) fn check_release(
    package_root: &Path,
    release_path: Option<&Path>,
    manifest: Option<&ManifestInfo>,
    component_path: Option<&Path>,
    release_tag: Option<&str>,
) -> Result<Vec<DiagnosticFinding>> {
    let mut findings = Vec::new();
    let Some(release_path) = release_path else {
        findings.push(missing_release_bundle(package_root.to_path_buf()));
        return Ok(findings);
    };

    if !release_path.exists() || !release_path.is_dir() {
        findings.push(missing_release_bundle(release_path.to_path_buf()));
        return Ok(findings);
    }

    let release_dir = release_path.to_path_buf();
    let wasm_asset = select_wasm_asset(
        &release_dir,
        manifest.and_then(|manifest| manifest.artifact_wasm.as_deref()),
        component_path,
    )?;
    let manifest_asset = release_dir.join("child.toml");
    let checksums_asset = release_dir.join("checksums.txt");

    if !wasm_asset.as_ref().is_some_and(|path| path.exists()) {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Release,
            "PTN-RELEASE-002",
            wasm_asset.clone().or_else(|| Some(release_dir.clone())),
            "release bundle is missing the WASM component asset",
            Some("copy the manifest-declared .wasm component into the release bundle".to_string()),
        ));
    }

    if !manifest_asset.exists() {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Release,
            "PTN-RELEASE-003",
            Some(manifest_asset.clone()),
            "release bundle is missing child.toml",
            Some(
                "attach child.toml so registry discovery and install review can read package identity and needs"
                    .to_string(),
            ),
        ));
    }

    let existing_wasm_asset = wasm_asset.as_deref().filter(|path| path.exists());
    check_release_wasm_matches_component(existing_wasm_asset, component_path, &mut findings)?;
    check_manifest_sidecar(&manifest_asset, &mut findings)?;
    check_checksums(
        &checksums_asset,
        existing_wasm_asset,
        &manifest_asset,
        &mut findings,
    )?;
    check_manifest_version(manifest, &manifest_asset, release_tag, &mut findings)?;

    Ok(findings)
}

fn missing_release_bundle(location: PathBuf) -> DiagnosticFinding {
    DiagnosticFinding::error(
        DiagnosticPhase::Release,
        "PTN-RELEASE-001",
        Some(location),
        "release candidate has no release bundle evidence",
        Some(
            "prepare a release bundle directory with .wasm, child.toml, child.toml.sha256, and checksums.txt assets"
                .to_string(),
        ),
    )
}

fn select_wasm_asset(
    release_dir: &Path,
    manifest_artifact: Option<&Path>,
    component_path: Option<&Path>,
) -> Result<Option<PathBuf>> {
    if let Some(manifest_artifact) = manifest_artifact {
        return Ok(Some(release_dir.join(manifest_artifact)));
    }

    let mut wasm_assets = Vec::new();
    for entry in std::fs::read_dir(release_dir)
        .with_context(|| format!("reading release bundle {}", release_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "wasm")
        {
            wasm_assets.push(path);
        }
    }

    let Some(component_file_name) = component_path.and_then(Path::file_name) else {
        return Ok(single_or_none(wasm_assets));
    };

    let matching_component = wasm_assets
        .iter()
        .find(|path| path.file_name() == Some(component_file_name))
        .cloned();
    Ok(matching_component.or_else(|| single_or_none(wasm_assets)))
}

fn single_or_none(mut paths: Vec<PathBuf>) -> Option<PathBuf> {
    if paths.len() == 1 {
        paths.pop()
    } else {
        None
    }
}

fn check_release_wasm_matches_component(
    wasm_asset: Option<&Path>,
    component_path: Option<&Path>,
    findings: &mut Vec<DiagnosticFinding>,
) -> Result<()> {
    let (Some(wasm_asset), Some(component_path)) = (wasm_asset, component_path) else {
        return Ok(());
    };

    if !wasm_asset.exists() || !component_path.exists() {
        return Ok(());
    }

    let release_hash = sha256_file(wasm_asset)?;
    let component_hash = sha256_file(component_path)?;
    if !hash_eq(&release_hash, &component_hash) {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Release,
            "PTN-RELEASE-009",
            Some(wasm_asset.to_path_buf()),
            "release WASM asset does not match the inspected component artifact",
            Some(
                "copy the same component artifact checked by component-built diagnostics into the release bundle before publishing"
                    .to_string(),
            ),
        ));
    }

    Ok(())
}

fn check_manifest_sidecar(
    manifest_asset: &Path,
    findings: &mut Vec<DiagnosticFinding>,
) -> Result<()> {
    if !manifest_asset.exists() {
        return Ok(());
    }

    let sidecar = manifest_asset.with_file_name("child.toml.sha256");
    if !sidecar.exists() {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Release,
            "PTN-RELEASE-004",
            Some(sidecar),
            "release bundle is missing child.toml.sha256",
            Some("publish a sidecar hash for child.toml".to_string()),
        ));
        return Ok(());
    }

    let sidecar_content = std::fs::read_to_string(&sidecar)
        .with_context(|| format!("reading manifest hash sidecar {}", sidecar.display()))?;
    let Some(expected_hash) = extract_first_sha256(&sidecar_content) else {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Release,
            "PTN-RELEASE-004",
            Some(sidecar),
            "child.toml.sha256 does not contain a valid SHA256 digest",
            Some("write a 64-character hex SHA256 digest for child.toml".to_string()),
        ));
        return Ok(());
    };

    let actual_hash = sha256_file(manifest_asset)?;
    if !hash_eq(&expected_hash, &actual_hash) {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Release,
            "PTN-RELEASE-004",
            Some(sidecar),
            "child.toml.sha256 does not match child.toml",
            Some("regenerate child.toml.sha256 from the child.toml release asset".to_string()),
        ));
    }

    Ok(())
}

fn check_checksums(
    checksums_asset: &Path,
    wasm_asset: Option<&Path>,
    manifest_asset: &Path,
    findings: &mut Vec<DiagnosticFinding>,
) -> Result<()> {
    if !checksums_asset.exists() {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Release,
            "PTN-RELEASE-005",
            Some(checksums_asset.to_path_buf()),
            "release bundle is missing checksums.txt",
            Some(
                "publish checksums.txt covering the installable WASM and manifest assets"
                    .to_string(),
            ),
        ));
        return Ok(());
    }

    let content = std::fs::read_to_string(checksums_asset)
        .with_context(|| format!("reading checksums asset {}", checksums_asset.display()))?;
    let checksums = parse_checksums(&content);

    if let Some(wasm_asset) = wasm_asset {
        check_checksum_entry(
            checksums_asset,
            &checksums,
            wasm_asset,
            "PTN-RELEASE-006",
            "checksums.txt does not cover the WASM component asset",
            "add the WASM asset hash to checksums.txt",
            findings,
        )?;
    }

    if manifest_asset.exists() {
        check_checksum_entry(
            checksums_asset,
            &checksums,
            manifest_asset,
            "PTN-RELEASE-007",
            "checksums.txt does not cover child.toml",
            "add the child.toml hash to checksums.txt",
            findings,
        )?;
    }

    Ok(())
}

fn check_checksum_entry(
    checksums_asset: &Path,
    checksums: &BTreeMap<String, String>,
    asset: &Path,
    code: &'static str,
    missing_message: &'static str,
    missing_help: &'static str,
    findings: &mut Vec<DiagnosticFinding>,
) -> Result<()> {
    let name = checksum_asset_name(checksums_asset, asset);
    let Some(expected_hash) = checksums.get(&name) else {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Release,
            code,
            Some(checksums_asset.to_path_buf()),
            missing_message,
            Some(missing_help.to_string()),
        ));
        return Ok(());
    };

    let actual_hash = sha256_file(asset)?;
    if !hash_eq(expected_hash, &actual_hash) {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Release,
            code,
            Some(checksums_asset.to_path_buf()),
            format!("checksums.txt hash for {name} does not match the release asset"),
            Some(format!(
                "regenerate checksums.txt after copying {name} into the release bundle"
            )),
        ));
    }

    Ok(())
}

fn check_manifest_version(
    manifest: Option<&ManifestInfo>,
    manifest_asset: &Path,
    release_tag: Option<&str>,
    findings: &mut Vec<DiagnosticFinding>,
) -> Result<()> {
    let manifest_version = manifest.and_then(|manifest| manifest.version.as_deref());
    let manifest_name = manifest.and_then(|manifest| manifest.name.as_deref());

    if let (Some(source_version), true) = (manifest_version, manifest_asset.exists()) {
        let release_version = child_manifest_version(manifest_asset)?;
        if let Some(release_version) = release_version {
            if release_version != source_version {
                findings.push(DiagnosticFinding::error(
                    DiagnosticPhase::Release,
                    "PTN-RELEASE-008",
                    Some(manifest_asset.to_path_buf()),
                    "release child.toml version does not match source child.toml version",
                    Some(
                        "copy the current child.toml into the release bundle before publishing"
                            .to_string(),
                    ),
                ));
            }
        }
    }

    if let (Some(tag), Some(version)) = (release_tag, manifest_version) {
        if !release_tag_matches_manifest_version(tag, manifest_name, version) {
            findings.push(DiagnosticFinding::error(
                DiagnosticPhase::Release,
                "PTN-RELEASE-008",
                Some(manifest_asset.to_path_buf()),
                "release tag does not match child.toml version",
                Some("align the release tag, child.toml version, and package version before publication".to_string()),
            ));
        }
    }

    Ok(())
}

fn checksum_asset_name(checksums_asset: &Path, asset: &Path) -> String {
    let bundle_dir = checksums_asset.parent().unwrap_or_else(|| Path::new(""));
    asset
        .strip_prefix(bundle_dir)
        .unwrap_or(asset)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn child_manifest_version(path: &Path) -> Result<Option<String>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading release manifest {}", path.display()))?;
    let table = match content.parse::<Table>() {
        Ok(table) => table,
        Err(_) => return Ok(None),
    };
    Ok(table
        .get("child")
        .and_then(|value| value.as_table())
        .and_then(|child| child.get("version"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned))
}

fn release_tag_matches_manifest_version(
    tag: &str,
    child_name: Option<&str>,
    version: &str,
) -> bool {
    let version_tag = format!("v{version}");
    if tag == version || tag == version_tag {
        return true;
    }

    if let Some(child_name) = child_name {
        tag == format!("{child_name}-{version}") || tag == format!("{child_name}-v{version}")
    } else {
        false
    }
}

fn parse_checksums(content: &str) -> BTreeMap<String, String> {
    let mut checksums = BTreeMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((name, hash)) = parse_bsd_checksum_line(line) {
            checksums.insert(name, hash);
            continue;
        }

        if let Some((name, hash)) = parse_gnu_checksum_line(line) {
            checksums.insert(name, hash);
        }
    }

    checksums
}

fn parse_bsd_checksum_line(line: &str) -> Option<(String, String)> {
    let line = line.strip_prefix("SHA256 (")?;
    let (name, hash) = line.split_once(") = ")?;
    if !looks_like_sha256(hash) {
        return None;
    }
    Some((name.to_string(), hash.to_string()))
}

fn parse_gnu_checksum_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.split_whitespace();
    let hash = parts.next()?;
    if !looks_like_sha256(hash) {
        return None;
    }
    let mut name = parts.next()?.to_string();
    if let Some(stripped) = name.strip_prefix('*') {
        name = stripped.to_string();
    }
    Some((name, hash.to_string()))
}

fn extract_first_sha256(content: &str) -> Option<String> {
    for token in content.split(|c: char| c.is_whitespace()) {
        if looks_like_sha256(token) {
            return Some(token.to_string());
        }
    }
    None
}

fn looks_like_sha256(input: &str) -> bool {
    input.len() == 64 && input.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hash_eq(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{checksum_asset_name, parse_checksums, release_tag_matches_manifest_version};

    #[test]
    fn parse_checksums_supports_common_formats() {
        let input = r#"
8f6f1eb65f53f8f4f8d9f77a0f8c8e2ba3b72f8f2f07c4866b78f72300a4ad20  slate-manager.wasm
SHA256 (child.toml) = c26bcdf6529d8adf4ceac76714566491582f59d0bc889ef9e4d8ce96aa95f4c4
"#;
        let map = parse_checksums(input);
        assert_eq!(
            map.get("slate-manager.wasm").map(String::as_str),
            Some("8f6f1eb65f53f8f4f8d9f77a0f8c8e2ba3b72f8f2f07c4866b78f72300a4ad20")
        );
        assert_eq!(
            map.get("child.toml").map(String::as_str),
            Some("c26bcdf6529d8adf4ceac76714566491582f59d0bc889ef9e4d8ce96aa95f4c4")
        );
    }

    #[test]
    fn checksum_asset_names_preserve_bundle_relative_paths() {
        let checksums = Path::new("release/checksums.txt");
        let asset = Path::new("release/target/wasm32-wasip1/release/slate.wasm");

        assert_eq!(
            checksum_asset_name(checksums, asset),
            "target/wasm32-wasip1/release/slate.wasm"
        );
    }

    #[test]
    fn release_tags_match_single_and_per_child_conventions() {
        assert!(release_tag_matches_manifest_version(
            "0.2.0",
            Some("slate-manager"),
            "0.2.0"
        ));
        assert!(release_tag_matches_manifest_version(
            "v0.2.0",
            Some("slate-manager"),
            "0.2.0"
        ));
        assert!(release_tag_matches_manifest_version(
            "slate-manager-v0.2.0",
            Some("slate-manager"),
            "0.2.0"
        ));
        assert!(!release_tag_matches_manifest_version(
            "other-child-v0.2.0",
            Some("slate-manager"),
            "0.2.0"
        ));
    }
}
