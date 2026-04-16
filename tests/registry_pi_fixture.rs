use patina::interface;
use std::fs;

#[test]
fn registry_fixture_can_define_pi_without_static_catalog_edits() {
    let _guard = patina::test_support::env_test_mutex()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    let temp = tempfile::TempDir::new().unwrap();
    let patina_home = temp.path().join("patina-home");
    let interfaces_dir = patina_home.join("interfaces");
    fs::create_dir_all(&interfaces_dir).unwrap();
    fs::write(
        interfaces_dir.join("registry.toml"),
        include_str!("fixtures/interface-registry/pi-only.toml"),
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
        assert_eq!(supported, vec!["pi"]);

        let bundle = interface::interface_bundle("pi").unwrap();
        assert_eq!(bundle.display_name, "PI Fixture");
        assert_eq!(bundle.version, "fixture");

        let launch_info = interface::launch::get("pi").unwrap();
        assert_eq!(launch_info.name, "pi");
        assert_eq!(launch_info.display, "PI Fixture");
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
