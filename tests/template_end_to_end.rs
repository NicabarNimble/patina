use std::path::PathBuf;
use std::process::Command;

fn has_command(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[test]
fn cargo_generate_template_builds_for_wasm() {
    if !has_command("cargo", &["generate", "--version"]) {
        eprintln!("skipping template e2e: cargo-generate not installed");
        return;
    }

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let template_dir = repo_root.join("children/template");
    let sandbox = tempfile::tempdir().expect("create tempdir");

    let generate = Command::new("cargo")
        .current_dir(sandbox.path())
        .args([
            "generate",
            "--path",
            template_dir.to_str().expect("template path utf-8"),
            "--name",
            "generated-child",
            "--define",
            "child_name=generated-child",
            "--define",
            "package_name=patina-ai-child-generated-child",
            "--define",
            "description=Generated child for template e2e",
            "--silent",
        ])
        .output()
        .expect("run cargo generate");

    assert!(
        generate.status.success(),
        "cargo generate failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&generate.stdout),
        String::from_utf8_lossy(&generate.stderr)
    );

    let generated = sandbox.path().join("generated-child");
    assert!(generated.join("Cargo.toml").exists());
    assert!(generated.join("child.toml").exists());
    assert!(generated.join("src/lib.rs").exists());

    let cargo_toml = generated.join("Cargo.toml");
    let original = std::fs::read_to_string(&cargo_toml).expect("read generated Cargo.toml");
    let sdk_path = repo_root.join("sdk/patina-sdk");
    let patched = original.replace("../../sdk/patina-sdk", &sdk_path.to_string_lossy());
    std::fs::write(&cargo_toml, patched).expect("patch sdk path in generated Cargo.toml");

    let check = Command::new("cargo")
        .current_dir(&generated)
        .args(["check", "--target", "wasm32-wasip2", "--quiet"])
        .output()
        .expect("run cargo check for generated child");

    assert!(
        check.status.success(),
        "generated child check failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
