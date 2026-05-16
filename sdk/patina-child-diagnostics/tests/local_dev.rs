use std::collections::BTreeSet;
use std::path::PathBuf;

use patina_child_diagnostics::{check_local_dev, DiagnosticPhase, DiagnosticSeverity};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn valid_local_dev_child_has_no_findings() {
    let report = check_local_dev(fixture("valid-local-dev"));
    assert!(report.findings.is_empty(), "{}", report.render_text());
}

#[test]
fn local_dev_does_not_require_built_component_or_release_assets() {
    let root = fixture("valid-local-dev");
    assert!(!root.join("target/child.wasm").exists());
    assert!(!root.join("checksums.txt").exists());

    let report = check_local_dev(root);
    assert!(report.is_ok(), "{}", report.render_text());
    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.phase != DiagnosticPhase::Component
                && finding.phase != DiagnosticPhase::Release),
        "{}",
        report.render_text()
    );
}

#[test]
fn manifest_diagnostics_reject_legacy_and_missing_needs() {
    let report = check_local_dev(fixture("legacy-manifest"));
    let codes = report
        .findings
        .iter()
        .map(|finding| finding.code.as_str())
        .collect::<BTreeSet<_>>();

    assert!(
        codes.contains("PTN-MANIFEST-005"),
        "{}",
        report.render_text()
    );
    assert!(
        codes.contains("PTN-MANIFEST-006"),
        "{}",
        report.render_text()
    );
    assert!(
        codes.contains("PTN-MANIFEST-007"),
        "{}",
        report.render_text()
    );
}

#[test]
fn wit_toy_imports_must_match_needs_toys() {
    let report = check_local_dev(fixture("missing-logging-need"));
    assert!(report.has_errors(), "expected errors");

    let toy_mismatch = report.findings.iter().find(|finding| {
        finding.code == "PTN-WIT-005" && finding.severity == DiagnosticSeverity::Error
    });

    assert!(toy_mismatch.is_some(), "{}", report.render_text());
}

#[test]
fn unresolved_wit_is_reported() {
    let report = check_local_dev(fixture("unresolved-wit"));
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == "PTN-WIT-002"),
        "{}",
        report.render_text()
    );
}
