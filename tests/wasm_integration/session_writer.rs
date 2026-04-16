use super::common::*;

#[test]
fn session_writer_component_instantiates_in_knowledge_child_engine() {
    let Some(wasm_path) = session_writer_component_path() else {
        return;
    };

    let engine = ChildEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("children/session-writer/child.toml");
    let manifest = ChildManifest::from_path(&manifest_path).unwrap();

    let result = engine.instantiate_child(&component, &manifest, None);
    assert!(
        result.is_ok(),
        "session-writer should instantiate in knowledge-child engine"
    );
}
