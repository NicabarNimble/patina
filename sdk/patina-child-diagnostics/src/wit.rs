use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use wit_parser::{Resolve, WorldId, WorldItem};

use crate::report::{DiagnosticFinding, DiagnosticPhase};

pub fn check_wit(root: &Path, declared_toys: &BTreeSet<String>) -> Result<Vec<DiagnosticFinding>> {
    let wit_root = root.join("wit");
    let mut findings = Vec::new();

    if !wit_root.exists() {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Wit,
            "PTN-WIT-001",
            Some(wit_root),
            "push-pure child package is missing a WIT world",
            Some("add wit/world.wit so the component imports and exports are explicit".to_string()),
        ));
        return Ok(findings);
    }

    let mut resolve = Resolve::default();
    let package_id = match resolve.push_dir(&wit_root) {
        Ok((package_id, _source_map)) => package_id,
        Err(error) => {
            findings.push(DiagnosticFinding::error(
                DiagnosticPhase::Wit,
                "PTN-WIT-002",
                Some(wit_root),
                "WIT world does not resolve",
                Some(format!(
                    "fix package, interface, use, import, and export names until standard WIT tooling can resolve the world: {error:#}"
                )),
            ));
            return Ok(findings);
        }
    };

    let package = &resolve.packages[package_id];
    if package.worlds.is_empty() {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Wit,
            "PTN-WIT-002",
            Some(wit_root),
            "WIT package contains no world",
            Some("add a world that declares child imports and exports".to_string()),
        ));
        return Ok(findings);
    }

    for (_name, world_id) in &package.worlds {
        check_world(&resolve, *world_id, &wit_root, declared_toys, &mut findings)?;
    }

    Ok(findings)
}

fn check_world(
    resolve: &Resolve,
    world_id: WorldId,
    wit_root: &Path,
    declared_toys: &BTreeSet<String>,
    findings: &mut Vec<DiagnosticFinding>,
) -> Result<()> {
    let world = &resolve.worlds[world_id];
    let location = wit_location(wit_root);

    if world.exports.is_empty() {
        findings.push(DiagnosticFinding::error(
            DiagnosticPhase::Wit,
            "PTN-WIT-003",
            Some(location.clone()),
            "WIT world has no exports",
            Some(
                "export the child business contract that Mother or another component will call"
                    .to_string(),
            ),
        ));
    }

    for (key, item) in &world.imports {
        let import_name = resolve.name_world_key(key);

        if contains_backend_specific_name(&import_name) {
            findings.push(DiagnosticFinding::warning(
                DiagnosticPhase::Wit,
                "PTN-WIT-004",
                Some(location.clone()),
                "WIT world contains orchestration-backend-specific names",
                Some(
                    "keep child contracts backend-neutral; place Mother/Rivet/queue-specific adaptation outside child business WIT"
                        .to_string(),
                ),
            ));
        }

        let Some(toy_name) = toy_for_import(resolve, item, &import_name)? else {
            findings.push(DiagnosticFinding::warning(
                DiagnosticPhase::Wit,
                "PTN-WIT-006",
                Some(location.clone()),
                format!(
                    "WIT imports `{import_name}`, which is not known to the toy authority registry"
                ),
                Some(
                    "use approved WASI or Patina toy interfaces, or document the community/local toy lane explicitly"
                        .to_string(),
                ),
            ));
            continue;
        };

        if !declared_toys.contains(&toy_name) {
            findings.push(DiagnosticFinding::error(
                DiagnosticPhase::Wit,
                "PTN-WIT-005",
                Some(location.clone()),
                format!(
                    "WIT imports `{import_name}`, but child.toml does not request the `{toy_name}` toy"
                ),
                Some(format!(
                    "add `{toy_name}` to [needs].toys so host authority requests match component imports"
                )),
            ));
        }
    }

    Ok(())
}

fn wit_location(wit_root: &Path) -> PathBuf {
    let world = wit_root.join("world.wit");
    if world.exists() {
        world
    } else {
        wit_root.to_path_buf()
    }
}

fn toy_for_import(
    resolve: &Resolve,
    item: &WorldItem,
    import_name: &str,
) -> Result<Option<String>> {
    let canonical = match item {
        WorldItem::Interface { id, .. } => resolve
            .id_of(*id)
            .unwrap_or_else(|| import_name.to_string()),
        WorldItem::Function(_) | WorldItem::Type(_) => import_name.to_string(),
    };

    Ok(toy_name_from_interface_id(&canonical))
}

fn toy_name_from_interface_id(interface_id: &str) -> Option<String> {
    let without_version = interface_id.split('@').next().unwrap_or(interface_id);
    let (namespace, rest) = without_version.split_once(':')?;
    let (package, interface) = rest.split_once('/')?;

    match (namespace, package, interface) {
        ("wasi", "logging", "logging") => Some("logging".to_string()),
        ("wasi", "keyvalue", _) => Some("keyvalue".to_string()),
        ("wasi", "filesystem", _) => Some("filesystem".to_string()),
        ("wasi", "http", _) => Some("http".to_string()),
        ("wasi", "sql", _) => Some("sql".to_string()),
        ("wasi", "messaging", _) => Some("messaging".to_string()),
        ("patina", "measure", "measure") => Some("measure".to_string()),
        ("patina", "config", "config") => Some("config".to_string()),
        ("patina", "keyvalue", _) => Some("keyvalue".to_string()),
        ("patina", "logging", "logging") => Some("logging".to_string()),
        ("patina", "connect", "connect") => Some("connect".to_string()),
        ("patina", "events-stream", "events-stream") => Some("events".to_string()),
        ("patina", "git", "git") => Some("git".to_string()),
        ("patina", "peer", "peer") => Some("peer".to_string()),
        ("patina", "task", "task") => Some("task".to_string()),
        ("patina", "sql", _) => Some("sql".to_string()),
        _ => None,
    }
}

fn contains_backend_specific_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("rivet") || lower.contains("workflow") || lower.contains("mother:")
}

#[cfg(test)]
mod tests {
    use super::toy_name_from_interface_id;

    #[test]
    fn maps_known_toy_imports() {
        assert_eq!(
            toy_name_from_interface_id("wasi:logging/logging@0.1.0").as_deref(),
            Some("logging")
        );
        assert_eq!(
            toy_name_from_interface_id("patina:measure/measure@0.1.0").as_deref(),
            Some("measure")
        );
        assert_eq!(toy_name_from_interface_id("patina:records/transform"), None);
    }
}
