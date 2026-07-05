use std::path::PathBuf;
use std::process::Command;

fn patina_bin() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_patina") {
        return PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_patina-ai") {
        return PathBuf::from(path);
    }

    // Fallback for environments that don't expose CARGO_BIN_EXE_*
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join(if cfg!(windows) {
            "patina.exe"
        } else {
            "patina"
        })
}

fn has_wasm32_wasip2_target() -> bool {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| {
            o.status.success()
                && String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .any(|line| line.trim() == "wasm32-wasip2")
        })
        .unwrap_or(false)
}

#[test]
fn child_init_defaults_to_mct_lane() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let out = Command::new(patina_bin())
        .current_dir(tmp.path())
        .args(["child", "init", "typed-default"])
        .output()
        .expect("run patina child init");

    assert!(
        out.status.success(),
        "child init failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let project = tmp.path().join("typed-default");
    assert!(project.join("Cargo.toml").exists());
    assert!(project.join("child.toml").exists());
    assert!(project.join("src/lib.rs").exists());
    assert!(project.join("README.md").exists());
    assert!(project.join("checks/diagnostics/Cargo.toml").exists());
    assert!(project
        .join("checks/diagnostics/tests/diagnostics.rs")
        .exists());
    assert!(project.join(".patina/children-dev.toml").exists());
    assert!(project.join("wit/world.wit").exists());
    assert!(project.join("wit/deps/logging.wit").exists());
    assert!(project.join("wit/deps/patina-measure.wit").exists());
    assert!(project.join("wit/deps/patina-git.wit").exists());
    assert!(!project.join("wit/deps/patina-record.wit").exists());

    let lib = std::fs::read_to_string(project.join("src/lib.rs")).expect("read lib.rs");
    assert!(lib.contains("wit_bindgen::generate!"));
    assert!(lib.contains("export!(TypedDefault);"));
    assert!(lib.contains("exports::patina::mct::child::Guest"));
    assert!(!lib.contains("register_child!"));

    let manifest = std::fs::read_to_string(project.join("child.toml")).expect("read child.toml");
    assert!(manifest.contains("kind = \"child\""));
    assert!(manifest.contains("role = \"app\""));
    assert!(manifest.contains("[child.artifact]"));
    assert!(manifest.contains("wasm = \"artifacts/typed-default.wasm\""));
    assert!(manifest.contains("mode = \"wit-only\""));
    assert!(manifest.contains("patina:mct/child@0.1.0.echo"));
    assert!(manifest.contains("[needs]"));
    assert!(manifest.contains("toys = [\"logging\", \"measure\", \"git\"]"));

    let diagnostics =
        std::fs::read_to_string(project.join("checks/diagnostics/tests/diagnostics.rs"))
            .expect("read diagnostics test");
    assert!(diagnostics.contains("patina_child_diagnostics::check_local_dev"));

    let children_dev = std::fs::read_to_string(project.join(".patina/children-dev.toml"))
        .expect("read children-dev config");
    assert!(children_dev.contains("[children.typed-default]"));
    assert!(children_dev.contains(".patina/dev/components/typed-default.wasm"));
}

#[test]
fn child_init_integrated_template_uses_preserved_records_lane() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let out = Command::new(patina_bin())
        .current_dir(tmp.path())
        .args([
            "child",
            "init",
            "integrated-child",
            "--template",
            "integrated",
        ])
        .output()
        .expect("run patina child init --template integrated");

    assert!(
        out.status.success(),
        "integrated child init failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let project = tmp.path().join("integrated-child");
    assert!(project.join("wit/deps/patina-record.wit").exists());
    assert!(!project.join("wit/deps/patina-git.wit").exists());

    let lib = std::fs::read_to_string(project.join("src/lib.rs")).expect("read lib.rs");
    assert!(lib.contains("exports::patina::records::transform::Guest"));
    assert!(lib.contains("export!(IntegratedChild);"));

    let world = std::fs::read_to_string(project.join("wit/world.wit")).expect("read world.wit");
    assert!(world.contains("export patina:records/transform@0.1.0"));
}

#[test]
fn child_init_legacy_flag_uses_legacy_lane() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let out = Command::new(patina_bin())
        .current_dir(tmp.path())
        .args(["child", "init", "legacy-child", "--legacy"])
        .output()
        .expect("run patina child init --legacy");

    assert!(
        out.status.success(),
        "legacy child init failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let project = tmp.path().join("legacy-child");
    assert!(project.join("Cargo.toml").exists());
    assert!(project.join("child.toml").exists());
    assert!(project.join("src/lib.rs").exists());
    assert!(!project.join("wit/world.wit").exists());

    let lib = std::fs::read_to_string(project.join("src/lib.rs")).expect("read lib.rs");
    assert!(lib.contains("register_child!"));
}

#[test]
fn mct_scaffold_contract_and_integrated_template_stay_separate() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let embedded_cargo =
        std::fs::read_to_string(repo.join("resources/templates/child/child/Cargo.toml.tmpl"))
            .expect("read embedded cargo template");
    let embedded_manifest =
        std::fs::read_to_string(repo.join("resources/templates/child/child/child.toml.tmpl"))
            .expect("read embedded manifest template");
    let embedded_lib =
        std::fs::read_to_string(repo.join("resources/templates/child/child/lib.rs.tmpl"))
            .expect("read embedded lib template");
    let embedded_world =
        std::fs::read_to_string(repo.join("resources/templates/child/child/world.wit.tmpl"))
            .expect("read embedded world template");
    let embedded_diagnostics_cargo = std::fs::read_to_string(
        repo.join("resources/templates/child/child/diagnostics-Cargo.toml.tmpl"),
    )
    .expect("read embedded diagnostics cargo template");
    let embedded_diagnostics_test = std::fs::read_to_string(
        repo.join("resources/templates/child/child/diagnostics-test.rs.tmpl"),
    )
    .expect("read embedded diagnostics test template");
    let embedded_children_dev = std::fs::read_to_string(
        repo.join("resources/templates/child/child/children-dev.toml.tmpl"),
    )
    .expect("read embedded children-dev template");

    let integrated_lib = std::fs::read_to_string(
        repo.join("resources/templates/child/integrated-child/lib.rs.tmpl"),
    )
    .expect("read integrated lib template");
    let integrated_world = std::fs::read_to_string(
        repo.join("resources/templates/child/integrated-child/world.wit.tmpl"),
    )
    .expect("read integrated world template");
    let sdk_template_lib = std::fs::read_to_string(repo.join("sdk/template/src/lib.rs"))
        .expect("read sdk template lib");
    let sdk_template_world = std::fs::read_to_string(repo.join("sdk/template/wit/world.wit"))
        .expect("read sdk template world");

    for marker in [
        "wit-bindgen",
        "[package.metadata.component.target]",
        "world = \"mct-child\"",
    ] {
        assert!(
            embedded_cargo.contains(marker),
            "MCT cargo template missing marker: {marker}"
        );
    }
    assert!(
        embedded_cargo.contains("patina-sdk = \"__SDK_VERSION__\""),
        "MCT cargo template should use scaffold-substituted published SDK version"
    );
    assert!(
        embedded_manifest.contains("[child.artifact]")
            && embedded_manifest.contains("wasm = \"artifacts/__NAME__.wasm\"")
            && embedded_manifest.contains("mode = \"wit-only\"")
            && embedded_manifest.contains("patina:mct/child@0.1.0.echo"),
        "MCT manifest template should declare package-relative artifact and WIT ingress"
    );
    assert!(
        embedded_manifest.contains("toys = [\"logging\", \"measure\", \"git\"]"),
        "MCT manifest template should request the hosted MCT toy surface"
    );

    for marker in [
        "wit_bindgen::generate!",
        "exports::patina::mct::child::Guest",
        "export!(",
    ] {
        assert!(
            embedded_lib.contains(marker),
            "MCT lib template missing marker: {marker}"
        );
    }

    for marker in [
        "export child;",
        "import wasi:logging/logging@0.1.0",
        "import patina:measure/measure@0.1.0",
        "import patina:git/git@0.1.0",
    ] {
        assert!(
            embedded_world.contains(marker),
            "MCT world template missing marker: {marker}"
        );
    }

    for marker in ["check_local_dev", "check_children_dev_config"] {
        assert!(
            embedded_diagnostics_test.contains(marker),
            "embedded diagnostics test missing marker: {marker}"
        );
    }
    assert!(
        embedded_diagnostics_cargo.contains("git = \"https://github.com/NicabarNimble/patina\""),
        "embedded diagnostics cargo should use external-safe git dependency"
    );
    for marker in [".patina/dev/components", ".patina/dev/releases"] {
        assert!(
            embedded_children_dev.contains(marker),
            "embedded children-dev missing marker: {marker}"
        );
    }

    assert!(
        !embedded_lib.contains("register_child!"),
        "MCT template drift: legacy register_child! appeared"
    );
    assert!(
        !embedded_lib.contains("exports::patina::records::transform::Guest"),
        "MCT template drift: integrated records export appeared"
    );
    assert!(
        !embedded_cargo.contains("patina-sdk-legacy"),
        "MCT template drift: legacy SDK appeared"
    );

    assert!(
        integrated_lib.contains("exports::patina::records::transform::Guest"),
        "integrated template should preserve records export"
    );
    assert!(
        integrated_world.contains("export patina:records/transform@0.1.0"),
        "integrated template should preserve records world"
    );
    assert!(
        sdk_template_lib.contains("exports::patina::records::transform::Guest"),
        "sdk/template should remain the integrated-Mother template"
    );
    assert!(
        sdk_template_world.contains("export patina:records/transform@0.1.0"),
        "sdk/template world should remain integrated-Mother"
    );
}

#[test]
fn typed_scaffold_builds_for_wasm32_wasip2_when_target_installed() {
    if !has_wasm32_wasip2_target() {
        eprintln!("skipping child-init wasm build check: wasm32-wasip2 target not installed");
        return;
    }

    let tmp = tempfile::tempdir().expect("tempdir");

    let init = Command::new(patina_bin())
        .current_dir(tmp.path())
        .args(["child", "init", "typed-build-check"])
        .output()
        .expect("run child init");

    assert!(
        init.status.success(),
        "child init failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );

    let project = tmp.path().join("typed-build-check");
    let cargo_toml_path = project.join("Cargo.toml");
    let original = std::fs::read_to_string(&cargo_toml_path).expect("read generated Cargo.toml");
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sdk_path = repo.join("sdk/patina-sdk");
    let patched = original.replace(
        "patina-sdk = \"0.22.0\"",
        &format!("patina-sdk = {{ path = \"{}\" }}", sdk_path.display()),
    );
    std::fs::write(&cargo_toml_path, patched).expect("patch sdk path in generated Cargo.toml");

    let diagnostics_cargo_toml_path = project.join("checks/diagnostics/Cargo.toml");
    let diagnostics_original = std::fs::read_to_string(&diagnostics_cargo_toml_path)
        .expect("read generated diagnostics Cargo.toml");
    let diagnostics_path = repo.join("sdk/patina-child-diagnostics");
    let diagnostics_patched = diagnostics_original.replace(
        "patina-child-diagnostics = { git = \"https://github.com/NicabarNimble/patina\", package = \"patina-child-diagnostics\" }",
        &format!(
            "patina-child-diagnostics = {{ path = \"{}\" }}",
            diagnostics_path.display()
        ),
    );
    std::fs::write(&diagnostics_cargo_toml_path, diagnostics_patched)
        .expect("patch diagnostics sdk path in generated Cargo.toml");

    let check = Command::new("cargo")
        .current_dir(&project)
        .args(["check", "--target", "wasm32-wasip2", "--quiet"])
        .output()
        .expect("run cargo check for typed scaffold");

    assert!(
        check.status.success(),
        "typed scaffold check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );

    let diagnostics = Command::new("cargo")
        .current_dir(&project)
        .args([
            "test",
            "--manifest-path",
            "checks/diagnostics/Cargo.toml",
            "--quiet",
        ])
        .output()
        .expect("run diagnostics test for typed scaffold");

    assert!(
        diagnostics.status.success(),
        "typed scaffold diagnostics test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&diagnostics.stdout),
        String::from_utf8_lossy(&diagnostics.stderr)
    );
}
