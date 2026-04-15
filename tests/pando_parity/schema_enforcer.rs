use super::common::*;

#[test]
fn schema_enforcer_push_equals_composed_with_mocked_upstream() {
    let Some(schema_wasm) = child_wasm_path("schema_enforcer") else {
        return;
    };
    let Some(se_adapter_wasm) = adapter_wasm_path("se_pando") else {
        return;
    };

    let fixtures = [
        FixtureRecord {
            record_id: "r1",
            source_path: "alpha.txt",
            source_hash: "hash-a",
            content: "Hello Alpha",
            content_hash: "content-a",
        },
        FixtureRecord {
            record_id: "",
            source_path: "broken.txt",
            source_hash: "hash-b",
            content: "Bad",
            content_hash: "content-b",
        },
    ];

    let push_input = fixtures
        .iter()
        .map(to_schema_push_record)
        .collect::<Vec<_>>();

    let mut push_config = wasmtime::Config::new();
    push_config.wasm_component_model(true);
    let push_engine = wasmtime::Engine::new(&push_config).expect("push engine");
    let push_component = Component::from_file(&push_engine, &schema_wasm).expect("schema child");
    let mut push_linker = Linker::new(&push_engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut push_linker).expect("wasi linker");
    schema_child_bindings::SchemaEnforcer::add_to_linker::<
        PushSchemaHost,
        wasmtime::component::HasSelf<PushSchemaHost>,
    >(&mut push_linker, |state| state)
    .expect("schema linker");
    let mut push_store = Store::new(
        &push_engine,
        PushSchemaHost {
            wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
            table: ResourceTable::new(),
        },
    );
    let push_instance = schema_child_bindings::SchemaEnforcer::instantiate(
        &mut push_store,
        &push_component,
        &push_linker,
    )
    .expect("schema instantiate");
    let push_result = push_instance
        .patina_records_transform()
        .call_transform(&mut push_store, &push_input)
        .expect("push call")
        .expect("push result");

    let composed = compose_component(
        &[
            ("se", "patina:test:se", schema_wasm.as_path()),
            (
                "schema-transform",
                "patina:test:se-pando",
                se_adapter_wasm.as_path(),
            ),
        ],
        &[("se", "schema-transform", "patina:records/transform")],
        ("schema-transform", "patina:pando/transform"),
    )
    .expect("compose schema");

    let upstream = fixtures
        .iter()
        .map(to_schema_parity_record)
        .collect::<Vec<_>>();
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = wasmtime::Engine::new(&config).expect("engine");
    let component = Component::new(&engine, composed).expect("component");
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("wasi linker");
    parity_schema_bindings::SchemaRunner::add_to_linker::<
        SchemaHostState,
        wasmtime::component::HasSelf<SchemaHostState>,
    >(&mut linker, |state| state)
    .expect("schema-runner linker");
    let mut store = Store::new(
        &engine,
        SchemaHostState {
            wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
            table: ResourceTable::new(),
            upstream_records: upstream,
        },
    );
    let instance =
        parity_schema_bindings::SchemaRunner::instantiate(&mut store, &component, &linker)
            .expect("schema-runner instantiate");
    let composed_result = instance
        .patina_pando_transform()
        .call_run(&mut store)
        .expect("composed call")
        .expect("composed result");

    assert_eq!(push_result.accepted.len(), composed_result.accepted.len());
    assert_eq!(push_result.rejected.len(), composed_result.rejected.len());
    assert_eq!(
        push_result.rejected[0].reason,
        composed_result.rejected[0].reason
    );
}
