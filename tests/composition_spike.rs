use std::path::PathBuf;

use wac_graph::{types::Package, CompositionGraph, EncodeOptions};

fn child_wasm_path(crate_stem: &str) -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root
        .join("target/wasm32-wasip2/debug")
        .join(format!("patina_ai_child_{crate_stem}.wasm"));
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

#[test]
fn compose_schema_and_dedup_with_wac_graph() {
    let Some(schema_wasm) = child_wasm_path("schema_enforcer") else {
        return;
    };
    let Some(dedup_wasm) = child_wasm_path("dedup_filter") else {
        return;
    };

    let mut graph = CompositionGraph::new();

    let schema_pkg = Package::from_file(
        "patina:schema-enforcer",
        None,
        &schema_wasm,
        graph.types_mut(),
    )
    .expect("load schema-enforcer package");
    let schema_pkg_id = graph
        .register_package(schema_pkg)
        .expect("register schema-enforcer package");

    let dedup_pkg = Package::from_file("patina:dedup-filter", None, &dedup_wasm, graph.types_mut())
        .expect("load dedup-filter package");
    let dedup_pkg_id = graph
        .register_package(dedup_pkg)
        .expect("register dedup-filter package");

    let schema_inst = graph.instantiate(schema_pkg_id);
    let dedup_inst = graph.instantiate(dedup_pkg_id);

    let dedup_imports = graph
        .get_instantiation_arguments(dedup_inst)
        .map(|(name, _)| name.to_string())
        .collect::<Vec<_>>();
    assert!(
        !dedup_imports
            .iter()
            .any(|name| name == "patina:records/transform@0.1.0"),
        "dedup-filter unexpectedly imports transform in push-pure model"
    );

    let _schema_transform = graph
        .alias_instance_export(schema_inst, "patina:records/transform@0.1.0")
        .expect("alias schema transform export");

    let dedup_transform = graph
        .alias_instance_export(dedup_inst, "patina:records/transform@0.1.0")
        .expect("alias dedup transform export");
    graph
        .export(dedup_transform, "patina:records/transform@0.1.0")
        .expect("export dedup transform from composition");

    let composed_bytes = graph
        .encode(EncodeOptions::default())
        .expect("encode composed component");

    let engine = wasmtime::Engine::new(wasmtime::Config::new().wasm_component_model(true))
        .expect("construct wasmtime engine");
    let component = wasmtime::component::Component::new(&engine, &composed_bytes)
        .expect("load composed component in wasmtime");

    let export_names = component
        .component_type()
        .exports(&engine)
        .map(|(name, _)| name.to_string())
        .collect::<Vec<_>>();
    assert!(
        export_names
            .iter()
            .any(|name| name == "patina:records/transform@0.1.0"),
        "missing expected transform export: {export_names:?}"
    );
}
