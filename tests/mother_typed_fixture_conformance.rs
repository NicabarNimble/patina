use std::path::{Path, PathBuf};

fn fixture_path(dir: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(dir)
        .join(name)
}

fn import_export_names(path: &Path) -> (Vec<String>, Vec<String>) {
    assert!(path.exists(), "missing fixture {}", path.display());

    let engine = wasmtime::Engine::new(wasmtime::Config::new().wasm_component_model(true))
        .expect("construct wasmtime engine");
    let component = wasmtime::component::Component::from_file(&engine, path)
        .unwrap_or_else(|e| panic!("loading fixture {}: {e}", path.display()));

    let imports = component
        .component_type()
        .imports(&engine)
        .map(|(name, _)| name.to_string())
        .collect::<Vec<_>>();
    let exports = component
        .component_type()
        .exports(&engine)
        .map(|(name, _)| name.to_string())
        .collect::<Vec<_>>();
    (imports, exports)
}

#[test]
fn mother_typed_fixtures_match_expected_interfaces() {
    let (schema_imports, schema_exports) =
        import_export_names(&fixture_path("mother-typed", "schema-enforcer-child.wasm"));
    assert!(
        schema_exports
            .iter()
            .any(|name| name == "patina:records/transform@0.1.0"),
        "schema fixture missing transform export: {schema_exports:?}"
    );
    assert!(
        !schema_imports
            .iter()
            .any(|name| name == "patina:records/transform@0.1.0"),
        "schema fixture unexpectedly imports transform: {schema_imports:?}"
    );

    let (adapter_imports, adapter_exports) =
        import_export_names(&fixture_path("mother-typed", "se-pando-adapter.wasm"));
    assert!(
        adapter_imports
            .iter()
            .any(|name| name == "patina:records/transform@0.1.0"),
        "adapter fixture missing transform import: {adapter_imports:?}"
    );
    assert!(
        adapter_exports
            .iter()
            .any(|name| name == "patina:pando/transform@0.1.0"),
        "adapter fixture missing pando export: {adapter_exports:?}"
    );
}

#[test]
fn mother_service_fixture_exports_handle_contract() {
    let (imports, exports) = import_export_names(&fixture_path(
        "mother-service",
        "belief-verifier-child.wasm",
    ));

    for expected in ["handle", "drain", "tick", "health", "on-load"] {
        assert!(
            exports.iter().any(|name| name == expected),
            "service fixture missing export '{expected}': {exports:?}"
        );
    }

    assert!(
        imports.iter().any(|name| name == "patina:task/task@0.1.0"),
        "service fixture missing expected task import: {imports:?}"
    );
}
