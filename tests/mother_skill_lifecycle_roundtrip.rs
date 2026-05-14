use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn patina_bin() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_patina") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join(if cfg!(windows) {
            "patina.exe"
        } else {
            "patina"
        })
}

fn run_json(patina_home: &Path, cwd: Option<&Path>, args: &[&str]) -> Value {
    let mut command = Command::new(patina_bin());
    command.env("PATINA_HOME", patina_home).args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output().expect("run patina command");
    assert!(
        output.status.success(),
        "command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("json stdout")
}

fn run_expect_failure(patina_home: &Path, cwd: &Path, args: &[&str]) -> String {
    let output = Command::new(patina_bin())
        .env("PATINA_HOME", patina_home)
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("run patina command");
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn create_sandbox(patina_home: &Path, scenario: &str) -> Value {
    run_json(
        patina_home,
        None,
        &[
            "mother",
            "skills",
            "sandbox",
            "create",
            "--scenario",
            scenario,
            "--default-interface",
            "gemini",
            "--json",
        ],
    )
}

#[test]
fn skill_lifecycle_install_sync_uninstall_roundtrip() {
    let temp = tempfile::tempdir().unwrap();
    let patina_home = temp.path().join("patina-home");
    let sandbox = create_sandbox(&patina_home, "project-empty");
    let project = PathBuf::from(sandbox["project_root"].as_str().unwrap());

    let install = run_json(
        &patina_home,
        Some(&project),
        &["mother", "skills", "install", "fixture-skill-app", "--json"],
    );
    assert_eq!(install["schema"], "patina.mother.skills.install-plan.v1");
    assert_eq!(install["applied"], true);
    assert_eq!(install["actions"].as_array().unwrap().len(), 2);

    let status = run_json(
        &patina_home,
        Some(&project),
        &["mother", "skills", "status", "--json"],
    );
    for tuple in status["tuples"].as_array().unwrap() {
        assert_eq!(tuple["state"], "installed");
        assert_eq!(tuple["managed"], true);
    }

    let manifest =
        project.join(".patina/local/mother/skills/gemini/project/fixture-skill-app.json");
    assert!(manifest.exists(), "projection manifest should exist");

    let sync = run_json(
        &patina_home,
        Some(&project),
        &["mother", "skills", "sync", "--json"],
    );
    assert_eq!(sync["applied"], true);
    assert!(sync["actions"].as_array().unwrap().is_empty());

    let uninstall = run_json(
        &patina_home,
        Some(&project),
        &[
            "mother",
            "skills",
            "uninstall",
            "fixture-skill-app",
            "--json",
        ],
    );
    assert_eq!(
        uninstall["schema"],
        "patina.mother.skills.uninstall-plan.v1"
    );
    assert_eq!(uninstall["applied"], true);
    assert_eq!(uninstall["actions"].as_array().unwrap().len(), 2);
    assert!(!manifest.exists(), "projection manifest should be removed");

    let final_status = run_json(
        &patina_home,
        Some(&project),
        &["mother", "skills", "status", "--json"],
    );
    for tuple in final_status["tuples"].as_array().unwrap() {
        assert_eq!(tuple["state"], "absent");
    }

    let events = project.join(".patina/local/data/events.db");
    assert!(events.exists(), "projection audit events db should exist");
}

#[test]
fn skill_lifecycle_force_overwrites_conflicted_projection() {
    let temp = tempfile::tempdir().unwrap();
    let patina_home = temp.path().join("patina-home");
    let sandbox = create_sandbox(&patina_home, "project-empty");
    let project = PathBuf::from(sandbox["project_root"].as_str().unwrap());

    run_json(
        &patina_home,
        Some(&project),
        &["mother", "skills", "install", "fixture-skill-app", "--json"],
    );

    let hello = project.join(".gemini/skills/fixture-skill-app/hello/SKILL.md");
    std::fs::write(&hello, "unmanaged user change\n").unwrap();

    let error = run_expect_failure(
        &patina_home,
        &project,
        &["mother", "skills", "sync", "fixture-skill-app", "--json"],
    );
    assert!(error.contains("--force") || error.contains("force-required"));

    let forced = run_json(
        &patina_home,
        Some(&project),
        &[
            "mother",
            "skills",
            "sync",
            "fixture-skill-app",
            "--force",
            "--json",
        ],
    );
    assert_eq!(forced["applied"], true);
    assert!(forced["actions"].as_array().unwrap()[0]["requires_force"]
        .as_bool()
        .unwrap());
    let restored = std::fs::read_to_string(&hello).unwrap();
    assert!(!restored.contains("unmanaged user change"));
}

#[test]
fn skill_lifecycle_unknown_global_scope_reports_unsupported() {
    let temp = tempfile::tempdir().unwrap();
    let patina_home = temp.path().join("patina-home");
    let sandbox = create_sandbox(&patina_home, "project-empty");
    let project = PathBuf::from(sandbox["project_root"].as_str().unwrap());

    let status = run_json(
        &patina_home,
        Some(&project),
        &[
            "--interface",
            "unknown",
            "mother",
            "skills",
            "status",
            "--global",
            "--json",
        ],
    );
    for tuple in status["tuples"].as_array().unwrap() {
        assert_eq!(tuple["state"], "unsupported");
    }
}
