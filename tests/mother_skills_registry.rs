use patina::interface::{ensure_bundle_projection, ProjectionMode};
use patina::mother::skills;
use patina::project::{self, ProjectConfig};
use std::fs;

fn with_temp_env<T>(f: impl FnOnce(&tempfile::TempDir) -> T) -> T {
    let _guard = patina::test_support::env_test_mutex()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let temp = tempfile::TempDir::new().unwrap();
    let patina_home = temp.path().join("patina-home");
    fs::create_dir_all(&patina_home).unwrap();

    let old_home = std::env::var_os("HOME");
    let old_patina_home = std::env::var_os("PATINA_HOME");
    let old_cwd = std::env::current_dir().ok();
    unsafe {
        std::env::set_var("HOME", temp.path());
        std::env::set_var("PATINA_HOME", &patina_home);
    }
    std::env::set_current_dir(temp.path()).unwrap();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&temp)));

    match old_home {
        Some(value) => unsafe { std::env::set_var("HOME", value) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    match old_patina_home {
        Some(value) => unsafe { std::env::set_var("PATINA_HOME", value) },
        None => unsafe { std::env::remove_var("PATINA_HOME") },
    }
    if let Some(path) = old_cwd {
        let _ = std::env::set_current_dir(path);
    }

    match result {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[test]
fn mother_owns_known_skills_per_interface() {
    let required = [
        "session-new",
        "session-update",
        "session-note",
        "session-end",
        "spec",
        "epistemic-beliefs",
    ];

    for interface in ["claude", "gemini", "opencode", "pi"] {
        let known = skills::known_skills(interface);
        for skill in required {
            assert!(
                known.contains(&skill),
                "interface '{}' missing '{}' from mother-known skills",
                interface,
                skill
            );
        }
    }
}

#[test]
fn projection_errors_when_declared_skill_not_in_mother() {
    with_temp_env(|temp| {
        let registry_path = temp.path().join("patina-home/interfaces/registry.toml");
        fs::create_dir_all(registry_path.parent().unwrap()).unwrap();
        fs::write(
            &registry_path,
            r#"[[interface]]
name = "broken"
display_name = "Broken"
classification = "hitl"
command = "true"
detect_commands = ["true"]
tmux_policy = "auto"
version = "fixture"
managed_paths = [
  { relative_path = "AGENTS.md", kind = "file" },
  { relative_path = ".broken", kind = "directory" },
]
skills = { include = ["nonexistent-skill"] }
"#,
        )
        .unwrap();

        let mut config = ProjectConfig::with_name("patina");
        config.interfaces.allowed = vec!["broken".to_string()];
        config.interfaces.default = "broken".to_string();
        project::save(temp.path(), &config).unwrap();

        let result =
            ensure_bundle_projection("broken", temp.path(), ProjectionMode::RefreshManaged);
        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("Mother skill 'nonexistent-skill' is not available"));
        assert!(message.contains("interface 'broken'"));
    });
}
