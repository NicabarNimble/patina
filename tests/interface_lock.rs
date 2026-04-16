use std::path::PathBuf;

fn child_wasm_path(crate_stem: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/wasm32-wasip2/debug")
        .join(format!("patina_ai_child_{crate_stem}.wasm"))
}

fn import_export_names(path: &std::path::Path) -> (Vec<String>, Vec<String>) {
    assert!(
        path.exists(),
        "missing compiled child component at {} (run: cargo build -p <child-crate> --target wasm32-wasip2)",
        path.display()
    );

    let engine = wasmtime::Engine::new(wasmtime::Config::new().wasm_component_model(true))
        .expect("construct wasmtime engine");
    let component = wasmtime::component::Component::from_file(&engine, path)
        .unwrap_or_else(|e| panic!("loading component {}: {e}", path.display()));

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
fn typed_child_interface_contracts_remain_locked() {
    let (schema_imports, schema_exports) = import_export_names(&child_wasm_path("schema_enforcer"));
    assert!(
        schema_exports
            .iter()
            .any(|name| name == "patina:records/transform@0.1.0"),
        "schema-enforcer missing transform export: {schema_exports:?}"
    );
    assert!(
        !schema_imports
            .iter()
            .any(|name| name == "patina:records/transform@0.1.0"),
        "schema-enforcer unexpectedly imports transform: {schema_imports:?}"
    );

    let (dedup_imports, dedup_exports) = import_export_names(&child_wasm_path("dedup_filter"));
    assert!(
        !dedup_imports
            .iter()
            .any(|name| name == "patina:records/transform@0.1.0"),
        "dedup-filter unexpectedly imports transform: {dedup_imports:?}"
    );
    assert!(
        dedup_exports
            .iter()
            .any(|name| name == "patina:records/transform@0.1.0"),
        "dedup-filter missing transform export seam: {dedup_exports:?}"
    );

    let (_, source_exports) = import_export_names(&child_wasm_path("file_system_monitor"));
    assert!(
        source_exports
            .iter()
            .any(|name| name == "patina:records/source@0.1.0"),
        "file-system-monitor missing source export: {source_exports:?}"
    );

    let (_, extract_exports) = import_export_names(&child_wasm_path("content_extractor"));
    assert!(
        extract_exports
            .iter()
            .any(|name| name == "patina:records/extract@0.1.0"),
        "content-extractor missing extract export: {extract_exports:?}"
    );

    let (write_imports, write_exports) = import_export_names(&child_wasm_path("parquet_writer"));
    assert!(
        !write_imports
            .iter()
            .any(|name| name == "patina:records/transform@0.1.0"),
        "parquet-writer unexpectedly imports transform: {write_imports:?}"
    );
    assert!(
        write_exports
            .iter()
            .any(|name| name == "patina:records/write@0.1.0"),
        "parquet-writer missing write export: {write_exports:?}"
    );

    let (_, catalog_exports) = import_export_names(&child_wasm_path("lakehouse_catalog"));
    assert!(
        catalog_exports
            .iter()
            .any(|name| name == "patina:records/catalog@0.1.0"),
        "lakehouse-catalog missing catalog export: {catalog_exports:?}"
    );
}
