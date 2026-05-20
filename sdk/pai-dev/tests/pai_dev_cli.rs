use std::path::PathBuf;
use std::process::Command;

fn pai_dev() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pai-dev"))
}

fn diagnostics_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../patina-child-diagnostics/tests/fixtures")
        .join(name)
}

#[test]
fn help_is_sdk_developer_focused() {
    let output = Command::new(pai_dev())
        .arg("--help")
        .output()
        .expect("run pai-dev --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Patina SDK developer entry point"));
    assert!(stdout.contains("child"));
    assert!(stdout.contains("children"));
    assert!(!stdout.contains("mother"));
}

#[test]
fn local_dev_check_passes_for_valid_child() {
    let output = Command::new(pai_dev())
        .args(["child", "check", "local-dev"])
        .arg(diagnostics_fixture("valid-local-dev"))
        .output()
        .expect("run pai-dev local-dev");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ok: child diagnostics passed"));
}

#[test]
fn local_dev_check_fails_closed_for_invalid_child() {
    let output = Command::new(pai_dev())
        .args(["child", "check", "local-dev"])
        .arg(diagnostics_fixture("legacy-manifest"))
        .output()
        .expect("run pai-dev local-dev on invalid child");

    assert!(!output.status.success(), "expected non-zero exit status");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PTN-MANIFEST-005"), "stdout:\n{stdout}");
    assert!(stdout.contains("PTN-MANIFEST-006"), "stdout:\n{stdout}");
}

#[test]
fn local_dev_check_prints_warnings_without_failing() {
    let output = Command::new(pai_dev())
        .args(["child", "check", "local-dev"])
        .arg(diagnostics_fixture("broad-filesystem-scope"))
        .output()
        .expect("run pai-dev local-dev on warning fixture");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PTN-MANIFEST-009"), "stdout:\n{stdout}");
    assert!(stdout.contains("/project"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("ok: child diagnostics passed with warnings"),
        "stdout:\n{stdout}"
    );
}

#[test]
fn children_dev_check_uses_repo_config() {
    let output = Command::new(pai_dev())
        .args(["children", "check", "local-dev"])
        .arg(diagnostics_fixture("multi-child-dev-config"))
        .output()
        .expect("run pai-dev children local-dev");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ok: children-dev diagnostics passed"));
}
