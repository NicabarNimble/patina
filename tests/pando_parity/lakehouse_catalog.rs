use super::common::*;

#[test]
fn lakehouse_catalog_push_equals_composed_with_mocked_upstream() {
    let Some(lc_wasm) = child_wasm_path("lakehouse_catalog") else {
        return;
    };
    let Some(lc_adapter_wasm) = adapter_wasm_path("lc_pando") else {
        return;
    };

    let push_input = vec![to_catalog_push_file("/output/records.parquet", 3)];
    let upstream = vec![to_catalog_parity_file("/output/records.parquet", 3)];

    let mut push_config = wasmtime::Config::new();
    push_config.wasm_component_model(true);
    let push_engine = wasmtime::Engine::new(&push_config).expect("push engine");
    let push_component = Component::from_file(&push_engine, &lc_wasm).expect("catalog child");
    let mut push_linker = Linker::new(&push_engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut push_linker).expect("wasi linker");
    catalog_child_bindings::LakehouseCatalog::add_to_linker::<
        PushCatalogHost,
        wasmtime::component::HasSelf<PushCatalogHost>,
    >(&mut push_linker, |state| state)
    .expect("catalog linker");
    let mut push_store = Store::new(
        &push_engine,
        PushCatalogHost {
            wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
            table: ResourceTable::new(),
            kv: HashMap::new(),
        },
    );
    let push_instance = catalog_child_bindings::LakehouseCatalog::instantiate(
        &mut push_store,
        &push_component,
        &push_linker,
    )
    .expect("catalog instantiate");
    let push_result = push_instance
        .patina_records_catalog()
        .call_register(&mut push_store, &push_input)
        .expect("push call")
        .expect("push result");

    let composed = compose_component(
        &[
            ("lc", "patina:test:lc", lc_wasm.as_path()),
            (
                "lc-pando",
                "patina:test:lc-pando",
                lc_adapter_wasm.as_path(),
            ),
        ],
        &[("lc", "lc-pando", "patina:records/catalog")],
        ("lc-pando", "patina:pando/catalog"),
    )
    .expect("compose catalog");

    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = wasmtime::Engine::new(&config).expect("engine");
    let component = Component::new(&engine, composed).expect("component");
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("wasi linker");
    parity_catalog_bindings::CatalogRunner::add_to_linker::<
        CatalogHostState,
        wasmtime::component::HasSelf<CatalogHostState>,
    >(&mut linker, |state| state)
    .expect("catalog-runner linker");
    let mut store = Store::new(
        &engine,
        CatalogHostState {
            wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
            table: ResourceTable::new(),
            upstream_files: upstream,
            kv: HashMap::new(),
        },
    );
    let instance =
        parity_catalog_bindings::CatalogRunner::instantiate(&mut store, &component, &linker)
            .expect("catalog-runner instantiate");
    let composed_result = instance
        .patina_pando_catalog()
        .call_run(&mut store)
        .expect("composed call")
        .expect("composed result");

    assert_eq!(push_result.len(), 1);
    assert_eq!(composed_result.len(), 1);
    assert_eq!(push_result[0].file_path, composed_result[0].file_path);
    assert_eq!(push_result[0].record_count, composed_result[0].record_count);
    assert_eq!(
        push_result[0].schema_version,
        composed_result[0].schema_version
    );
}
