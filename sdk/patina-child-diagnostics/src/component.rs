use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use wasmparser::{Encoding, Parser, Payload};

use crate::report::{DiagnosticFinding, DiagnosticPhase};
use crate::wit::{toy_name_from_interface_id, WitInfo};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ComponentInfo {
    pub imports: BTreeSet<String>,
    pub exports: BTreeSet<String>,
    pub toy_imports: BTreeSet<String>,
}

pub(crate) fn check_component(
    package_root: &Path,
    component_path: Option<&Path>,
    wit: Option<&WitInfo>,
    declared_toys: &BTreeSet<String>,
) -> Result<Vec<DiagnosticFinding>> {
    let mut findings = Vec::new();
    let Some(component_path) = component_path else {
        findings.push(missing_component_finding(package_root.to_path_buf()));
        return Ok(findings);
    };

    if !component_path.exists() {
        findings.push(missing_component_finding(component_path.to_path_buf()));
        return Ok(findings);
    }

    let bytes = match std::fs::read(component_path)
        .with_context(|| format!("reading component artifact {}", component_path.display()))
    {
        Ok(bytes) => bytes,
        Err(error) => {
            findings.push(DiagnosticFinding::error(
                DiagnosticPhase::Component,
                "PTN-COMPONENT-003",
                Some(component_path.to_path_buf()),
                "WASM component could not be inspected for imports and exports",
                Some(format!("ensure the artifact is readable and valid for component-model introspection tooling: {error:#}")),
            ));
            return Ok(findings);
        }
    };

    let component = match inspect_component(&bytes) {
        Ok(component) => component,
        Err(ComponentInspectError::CoreModule) => {
            findings.push(DiagnosticFinding::error(
                DiagnosticPhase::Component,
                "PTN-COMPONENT-002",
                Some(component_path.to_path_buf()),
                "WASM artifact is a core module, not a component",
                Some(
                    "produce a WebAssembly component for push-pure child packages, such as a wasm32-wasip2/component-model artifact"
                        .to_string(),
                ),
            ));
            return Ok(findings);
        }
        Err(ComponentInspectError::Parse(error)) => {
            findings.push(DiagnosticFinding::error(
                DiagnosticPhase::Component,
                "PTN-COMPONENT-002",
                Some(component_path.to_path_buf()),
                "WASM artifact does not load as a component",
                Some(format!(
                    "produce a valid WebAssembly component, not an arbitrary file or malformed wasm: {error}"
                )),
            ));
            return Ok(findings);
        }
    };

    if let Some(wit) = wit {
        if component.imports != wit.imports {
            findings.push(DiagnosticFinding::error(
                DiagnosticPhase::Component,
                "PTN-COMPONENT-005",
                Some(component_path.to_path_buf()),
                "built component imports differ from the declared WIT imports",
                Some(format_set_delta(
                    "compare component imports to wit/world.wit and rebuild from the same WIT dependency set",
                    &wit.imports,
                    &component.imports,
                )),
            ));
        }

        if component.exports != wit.exports {
            findings.push(DiagnosticFinding::error(
                DiagnosticPhase::Component,
                "PTN-COMPONENT-006",
                Some(component_path.to_path_buf()),
                "built component exports differ from the declared WIT exports",
                Some(format_set_delta(
                    "compare component exports to wit/world.wit and rebuild from the intended business contract",
                    &wit.exports,
                    &component.exports,
                )),
            ));
        }
    }

    for toy_name in &component.toy_imports {
        if !declared_toys.contains(toy_name) {
            findings.push(DiagnosticFinding::error(
                DiagnosticPhase::Component,
                "PTN-COMPONENT-007",
                Some(component_path.to_path_buf()),
                format!(
                    "built component imports the `{toy_name}` toy, but child.toml does not request it"
                ),
                Some(format!(
                    "add `{toy_name}` to [needs].toys so host authority requests match actual component imports"
                )),
            ));
        }
    }

    Ok(findings)
}

fn inspect_component(bytes: &[u8]) -> std::result::Result<ComponentInfo, ComponentInspectError> {
    let mut encoding = None;
    let mut info = ComponentInfo::default();

    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|error| ComponentInspectError::Parse(error.to_string()))? {
            Payload::Version {
                encoding: found, ..
            } => match found {
                Encoding::Component => encoding = Some(Encoding::Component),
                Encoding::Module => return Err(ComponentInspectError::CoreModule),
            },
            Payload::ComponentImportSection(reader) => {
                for import in reader {
                    let import =
                        import.map_err(|error| ComponentInspectError::Parse(error.to_string()))?;
                    let import_name = import.name.0.to_string();
                    if let Some(toy_name) = toy_name_from_interface_id(&import_name) {
                        info.toy_imports.insert(toy_name);
                    }
                    info.imports.insert(import_name);
                }
            }
            Payload::ComponentExportSection(reader) => {
                for export in reader {
                    let export =
                        export.map_err(|error| ComponentInspectError::Parse(error.to_string()))?;
                    info.exports.insert(export.name.0.to_string());
                }
            }
            _ => {}
        }
    }

    if encoding == Some(Encoding::Component) {
        Ok(info)
    } else {
        Err(ComponentInspectError::Parse(
            "missing WebAssembly component header".to_string(),
        ))
    }
}

fn missing_component_finding(location: PathBuf) -> DiagnosticFinding {
    DiagnosticFinding::error(
        DiagnosticPhase::Component,
        "PTN-COMPONENT-001",
        Some(location),
        "component-stage diagnostics require a built WASM component artifact",
        Some(
            "build or copy the component to the explicit path, preferably .patina/dev/components/<child>.wasm"
                .to_string(),
        ),
    )
}

fn format_set_delta(
    message: &str,
    expected: &BTreeSet<String>,
    actual: &BTreeSet<String>,
) -> String {
    let missing = expected
        .difference(actual)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let extra = actual
        .difference(expected)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");

    let mut help = message.to_string();
    if !missing.is_empty() {
        help.push_str(&format!("; missing from component: {missing}"));
    }
    if !extra.is_empty() {
        help.push_str(&format!("; extra in component: {extra}"));
    }
    help
}

#[derive(Debug)]
enum ComponentInspectError {
    CoreModule,
    Parse(String),
}

#[cfg(test)]
mod tests {
    use super::inspect_component;

    #[test]
    fn header_only_component_is_inspectable() {
        let component = [0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00];
        let info = inspect_component(&component).expect("component header parses");
        assert!(info.imports.is_empty());
        assert!(info.exports.is_empty());
    }
}
