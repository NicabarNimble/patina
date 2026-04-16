use patina::interface;
use std::fs;

#[test]
fn codex_fixture_proves_registry_generality_without_enum_support() {
    let _guard = patina::test_support::env_test_mutex()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let temp = tempfile::TempDir::new().unwrap();
    let patina_home = temp.path().join("patina-home");
    let interfaces_dir = patina_home.join("interfaces");
    fs::create_dir_all(&interfaces_dir).unwrap();
    fs::write(
        interfaces_dir.join("registry.toml"),
        include_str!("fixtures/interface-registry/codex-only.toml"),
    )
    .unwrap();

    let codex_config_dir = temp.path().join(".config/codex");
    fs::create_dir_all(&codex_config_dir).unwrap();
    fs::write(
        codex_config_dir.join("settings.json"),
        r#"{"mcpServers":{"patina":{}}}"#,
    )
    .unwrap();

    let old_home = std::env::var_os("HOME");
    let old_patina_home = std::env::var_os("PATINA_HOME");
    unsafe {
        std::env::set_var("HOME", temp.path());
        std::env::set_var("PATINA_HOME", &patina_home);
    }

    let result = std::panic::catch_unwind(|| {
        let supported = interface::supported_ai_interfaces();
        assert_eq!(supported, vec!["codex"]);

        let iface = interface::interface("codex").unwrap();
        assert_eq!(iface.name(), "codex");
        assert_eq!(iface.display_name(), "Codex Fixture");

        interface::launch::set_default_interface("codex").unwrap();
        assert_eq!(
            interface::launch::default_interface_name().unwrap(),
            "codex"
        );

        assert!(interface::launch::interface_mcp_available("codex").unwrap());
    });

    match old_home {
        Some(value) => unsafe {
            std::env::set_var("HOME", value);
        },
        None => unsafe {
            std::env::remove_var("HOME");
        },
    }
    match old_patina_home {
        Some(value) => unsafe {
            std::env::set_var("PATINA_HOME", value);
        },
        None => unsafe {
            std::env::remove_var("PATINA_HOME");
        },
    }

    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}
