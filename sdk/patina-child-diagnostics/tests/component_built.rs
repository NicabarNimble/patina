use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use patina_child_diagnostics::{
    check_children_dev_components, check_component_built, check_package, CheckOptions,
    DiagnosticPhase, DiagnosticSeverity, DiagnosticStage,
};
use wasm_encoder::{
    Component, ComponentExportKind, ComponentExportSection, ComponentImportSection,
    ComponentTypeRef, Module,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn temp_repo(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "patina-child-diagnostics-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create temp repo");
    root
}

fn write_valid_child(root: &Path, name: &str) {
    fs::create_dir_all(root.join("wit/deps/logging")).expect("create wit dirs");
    fs::write(
        root.join("child.toml"),
        format!(
            r#"[child]
name = "{name}"
version = "0.1.0"
kind = "actor"

[needs]
toys = ["logging"]
"#
        ),
    )
    .expect("write child.toml");
    fs::write(
        root.join("wit/world.wit"),
        format!(
            r#"package patina:{name}@0.1.0;

world {name} {{
    import wasi:logging/logging@0.1.0;
    export run: func();
}}
"#
        ),
    )
    .expect("write world.wit");
    fs::write(
        root.join("wit/deps/logging/logging.wit"),
        r#"package wasi:logging@0.1.0;

interface logging {
    log: func(message: string);
}
"#,
    )
    .expect("write logging dep");
}

fn write_component(path: &Path, imports: &[&str], exports: &[&str]) {
    let mut component = Component::new();

    if !imports.is_empty() {
        let mut import_section = ComponentImportSection::new();
        for import in imports {
            import_section.import(import, ComponentTypeRef::Func(0));
        }
        component.section(&import_section);
    }

    if !exports.is_empty() {
        let mut export_section = ComponentExportSection::new();
        for export in exports {
            export_section.export(export, ComponentExportKind::Func, 0, None);
        }
        component.section(&export_section);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create component parent");
    }
    fs::write(path, component.finish()).expect("write component");
}

#[test]
fn component_built_requires_explicit_component_path() {
    let report = check_package(
        fixture("valid-local-dev"),
        CheckOptions {
            stage: DiagnosticStage::ComponentBuilt,
            component_path: None,
        },
    );

    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == "PTN-COMPONENT-001"),
        "{}",
        report.render_text()
    );
}

#[test]
fn component_built_rejects_core_wasm_module() {
    let repo = temp_repo("core-module");
    let component_path = repo.join("core.wasm");
    fs::write(&component_path, Module::new().finish()).expect("write core wasm module");

    let report = check_component_built(fixture("valid-local-dev"), &component_path);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == "PTN-COMPONENT-002"),
        "{}",
        report.render_text()
    );
}

#[test]
fn component_built_accepts_matching_component_contract() {
    let repo = temp_repo("matching-component");
    let component_path = repo.join(".patina/dev/components/valid-local-dev.wasm");
    write_component(&component_path, &["wasi:logging/logging@0.1.0"], &["run"]);

    let report = check_component_built(fixture("valid-local-dev"), &component_path);
    assert!(report.is_ok(), "{}", report.render_text());
}

#[test]
fn component_built_reports_import_export_drift() {
    let repo = temp_repo("drift-component");
    let component_path = repo.join(".patina/dev/components/valid-local-dev.wasm");
    write_component(&component_path, &[], &["other-run"]);

    let report = check_component_built(fixture("valid-local-dev"), &component_path);
    let codes = report
        .findings
        .iter()
        .map(|finding| finding.code.as_str())
        .collect::<BTreeSet<_>>();

    assert!(
        codes.contains("PTN-COMPONENT-005"),
        "{}",
        report.render_text()
    );
    assert!(
        codes.contains("PTN-COMPONENT-006"),
        "{}",
        report.render_text()
    );
}

#[test]
fn component_built_reports_toy_imports_missing_from_manifest() {
    let repo = temp_repo("missing-toy-component");
    let component_path = repo.join(".patina/dev/components/missing-logging-need.wasm");
    write_component(&component_path, &["wasi:logging/logging@0.1.0"], &["run"]);

    let report = check_component_built(fixture("missing-logging-need"), &component_path);
    let component_toy_mismatch = report.findings.iter().find(|finding| {
        finding.phase == DiagnosticPhase::Component
            && finding.severity == DiagnosticSeverity::Error
            && finding.code == "PTN-COMPONENT-007"
    });

    assert!(component_toy_mismatch.is_some(), "{}", report.render_text());
}

#[test]
fn children_dev_component_stage_uses_configured_component_path() {
    let repo = temp_repo("children-dev-component");
    let child_root = repo.join("children/configured-child");
    let component_path = repo.join(".patina/dev/components/configured-child.wasm");

    write_valid_child(&child_root, "configured-child");
    fs::create_dir_all(repo.join(".patina")).expect("create .patina");
    fs::write(
        repo.join(".patina/children-dev.toml"),
        r#"[children.configured-child]
root = "children/configured-child"
component = ".patina/dev/components/configured-child.wasm"
"#,
    )
    .expect("write children-dev config");
    write_component(&component_path, &["wasi:logging/logging@0.1.0"], &["run"]);

    let report = check_children_dev_components(&repo);
    assert!(report.is_ok(), "{}", report.render_text());
    assert_eq!(report.children.len(), 1);
    assert_eq!(
        report.children[0].component.as_deref(),
        Some(component_path.as_path())
    );
}
