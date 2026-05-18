use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use patina_child_diagnostics::{
    check_children_dev_release_candidates, check_package, check_release_candidate,
    check_release_candidate_with_tag, CheckOptions, DiagnosticStage,
};
use sha2::{Digest, Sha256};
use wasm_encoder::{
    Component, ComponentExportKind, ComponentExportSection, ComponentImportSection,
    ComponentTypeRef,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn temp_repo(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "patina-child-diagnostics-release-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create temp repo");
    root
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

fn write_release_bundle(release_dir: &Path, wasm_name: &str, component_path: &Path) {
    fs::create_dir_all(release_dir).expect("create release dir");
    let wasm_asset = release_dir.join(wasm_name);
    fs::copy(component_path, &wasm_asset).expect("copy wasm asset");

    let manifest_asset = release_dir.join("child.toml");
    fs::copy(
        fixture("valid-local-dev").join("child.toml"),
        &manifest_asset,
    )
    .expect("copy manifest asset");

    let manifest_hash = sha256_file(&manifest_asset);
    fs::write(
        release_dir.join("child.toml.sha256"),
        format!("{manifest_hash}  child.toml\n"),
    )
    .expect("write child.toml.sha256");

    let wasm_hash = sha256_file(&wasm_asset);
    fs::write(
        release_dir.join("checksums.txt"),
        format!("{wasm_hash}  {wasm_name}\nSHA256 (child.toml) = {manifest_hash}\n"),
    )
    .expect("write checksums.txt");
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).expect("read asset");
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn matching_component(path: &Path) {
    write_component(path, &["wasi:logging/logging@0.1.0"], &["run"]);
}

#[test]
fn release_candidate_requires_release_bundle_path() {
    let repo = temp_repo("missing-release-path");
    let component_path = repo.join("valid-local-dev.wasm");
    matching_component(&component_path);

    let report = check_package(
        fixture("valid-local-dev"),
        CheckOptions {
            stage: DiagnosticStage::ReleaseCandidate,
            component_path: Some(component_path),
            release_path: None,
            release_tag: None,
        },
    );

    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == "PTN-RELEASE-001"),
        "{}",
        report.render_text()
    );
}

#[test]
fn release_candidate_accepts_bundle_with_manifest_hash_and_checksums() {
    let repo = temp_repo("valid-release");
    let component_path = repo.join(".patina/dev/components/valid-local-dev.wasm");
    let release_dir = repo.join(".patina/dev/releases/valid-local-dev");
    matching_component(&component_path);
    write_release_bundle(&release_dir, "valid-local-dev.wasm", &component_path);

    let report = check_release_candidate(fixture("valid-local-dev"), &component_path, &release_dir);
    assert!(report.is_ok(), "{}", report.render_text());
}

#[test]
fn release_candidate_reports_missing_manifest_and_checksums() {
    let repo = temp_repo("missing-assets");
    let component_path = repo.join("valid-local-dev.wasm");
    let release_dir = repo.join("release");
    matching_component(&component_path);
    fs::create_dir_all(&release_dir).expect("create release dir");
    fs::copy(&component_path, release_dir.join("valid-local-dev.wasm")).expect("copy wasm");

    let report = check_release_candidate(fixture("valid-local-dev"), &component_path, &release_dir);
    let codes = report
        .findings
        .iter()
        .map(|finding| finding.code.as_str())
        .collect::<BTreeSet<_>>();

    assert!(
        codes.contains("PTN-RELEASE-003"),
        "{}",
        report.render_text()
    );
    assert!(
        codes.contains("PTN-RELEASE-005"),
        "{}",
        report.render_text()
    );
}

#[test]
fn release_candidate_reports_checksum_coverage_and_hash_mismatch() {
    let repo = temp_repo("bad-checksums");
    let component_path = repo.join("valid-local-dev.wasm");
    let release_dir = repo.join("release");
    matching_component(&component_path);
    write_release_bundle(&release_dir, "valid-local-dev.wasm", &component_path);
    fs::write(
        release_dir.join("checksums.txt"),
        "0000000000000000000000000000000000000000000000000000000000000000  valid-local-dev.wasm\n",
    )
    .expect("write bad checksums");

    let report = check_release_candidate(fixture("valid-local-dev"), &component_path, &release_dir);
    let codes = report
        .findings
        .iter()
        .map(|finding| finding.code.as_str())
        .collect::<BTreeSet<_>>();

    assert!(
        codes.contains("PTN-RELEASE-006"),
        "{}",
        report.render_text()
    );
    assert!(
        codes.contains("PTN-RELEASE-007"),
        "{}",
        report.render_text()
    );
}

#[test]
fn release_candidate_reports_tag_mismatch() {
    let repo = temp_repo("tag-mismatch");
    let component_path = repo.join("valid-local-dev.wasm");
    let release_dir = repo.join("release");
    matching_component(&component_path);
    write_release_bundle(&release_dir, "valid-local-dev.wasm", &component_path);

    let report = check_release_candidate_with_tag(
        fixture("valid-local-dev"),
        &component_path,
        &release_dir,
        "other-child-v9.9.9",
    );

    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == "PTN-RELEASE-008"),
        "{}",
        report.render_text()
    );
}

#[test]
fn children_dev_release_candidate_uses_configured_release_bundle() {
    let repo = temp_repo("children-dev-release");
    let child_root = repo.join("children/valid-local-dev");
    fs::create_dir_all(child_root.parent().expect("child parent")).expect("create child parent");
    copy_dir(&fixture("valid-local-dev"), &child_root);

    let component_path = repo.join(".patina/dev/components/valid-local-dev.wasm");
    let release_dir = repo.join(".patina/dev/releases/valid-local-dev");
    matching_component(&component_path);
    write_release_bundle(&release_dir, "valid-local-dev.wasm", &component_path);

    fs::create_dir_all(repo.join(".patina")).expect("create .patina");
    fs::write(
        repo.join(".patina/children-dev.toml"),
        r#"[children.valid-local-dev]
root = "children/valid-local-dev"
component = ".patina/dev/components/valid-local-dev.wasm"
release = ".patina/dev/releases/valid-local-dev"
tag = "valid-local-dev-v0.1.0"
"#,
    )
    .expect("write children-dev config");

    let report = check_children_dev_release_candidates(&repo);
    assert!(report.is_ok(), "{}", report.render_text());
    assert_eq!(report.children.len(), 1);
    assert_eq!(
        report.children[0].release.as_deref(),
        Some(release_dir.as_path())
    );
    assert_eq!(
        report.children[0].tag.as_deref(),
        Some("valid-local-dev-v0.1.0")
    );
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create destination dir");
    for entry in fs::read_dir(from).expect("read source dir") {
        let entry = entry.expect("source entry");
        let source = entry.path();
        let dest = to.join(entry.file_name());
        if source.is_dir() {
            copy_dir(&source, &dest);
        } else {
            fs::copy(&source, &dest).expect("copy fixture file");
        }
    }
}
