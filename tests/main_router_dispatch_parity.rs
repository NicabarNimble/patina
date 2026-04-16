use std::path::Path;
use std::process::{Command, Output};

fn run_patina(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_patina"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("failed to run patina {:?}: {}", args, e))
}

fn assert_success(args: &[&str], output: &Output) {
    assert!(
        output.status.success(),
        "command failed: patina {:?}\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn router_parity_mother_family_status_executes() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let args = ["mother", "status"];
    let output = run_patina(&args, repo_root);
    assert_success(&args, &output);
}

#[test]
fn router_parity_spec_family_list_executes_json() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let args = ["spec", "list", "--json"];
    let output = run_patina(&args, repo_root);
    assert_success(&args, &output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim_start().starts_with('['),
        "expected json array output, got: {}",
        stdout
    );
}

#[test]
fn router_parity_scrape_family_context_executes() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let args = ["context"];
    let output = run_patina(&args, repo_root);
    assert_success(&args, &output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Project") || stdout.contains("Context") || !stdout.trim().is_empty(),
        "expected non-empty context output, got: {}",
        stdout
    );
}

#[test]
fn router_parity_measure_family_json_executes() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let args = ["measure", "--json"];
    let output = run_patina(&args, repo_root);
    assert_success(&args, &output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim_start().starts_with('{'),
        "expected json object output, got: {}",
        stdout
    );
}

#[test]
fn router_parity_project_family_init_executes_and_creates_scaffold() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let project_name = "router-parity-demo";
    let args = ["init", project_name, "--local", "--no-commit"];
    let output = run_patina(&args, temp.path());
    assert_success(&args, &output);

    let project_dir = temp.path().join(project_name);
    assert!(project_dir.exists(), "expected project dir to exist");
    assert!(
        project_dir.join("layer/core").exists(),
        "expected initialized layer/core to exist"
    );
}
