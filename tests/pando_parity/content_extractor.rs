use super::common::*;

#[test]
fn content_extractor_push_equals_composed_with_mocked_upstream() {
    let Some(ce_wasm) = child_wasm_path("content_extractor") else {
        return;
    };
    let Some(ce_adapter_wasm) = adapter_wasm_path("ce_pando") else {
        return;
    };

    let input = TempDir::new().expect("temp input");
    std::fs::write(input.path().join("alpha.txt"), "Hello Alpha").expect("write alpha");
    std::fs::write(input.path().join("beta.md"), "Hello Beta").expect("write beta");

    let parity_files = [
        to_extract_parity_file("/input/alpha.txt", &input.path().join("alpha.txt")),
        to_extract_parity_file("/input/beta.md", &input.path().join("beta.md")),
    ];
    let push_files = [
        to_extract_push_file("/input/alpha.txt", &input.path().join("alpha.txt")),
        to_extract_push_file("/input/beta.md", &input.path().join("beta.md")),
    ];

    let mut push_config = wasmtime::Config::new();
    push_config.wasm_component_model(true);
    let push_engine = wasmtime::Engine::new(&push_config).expect("push engine");
    let push_component = Component::from_file(&push_engine, &ce_wasm).expect("extract child");
    let mut push_linker = Linker::new(&push_engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut push_linker).expect("wasi linker");
    extractor_child_bindings::ContentExtractor::add_to_linker::<
        PushExtractHost,
        wasmtime::component::HasSelf<PushExtractHost>,
    >(&mut push_linker, |state| state)
    .expect("extract linker");
    let mut push_wasi = wasmtime_wasi::WasiCtxBuilder::new();
    push_wasi
        .preopened_dir(
            input.path(),
            "/input",
            wasmtime_wasi::DirPerms::READ,
            wasmtime_wasi::FilePerms::READ,
        )
        .expect("preopen input");
    let mut push_store = Store::new(
        &push_engine,
        PushExtractHost {
            wasi: push_wasi.build(),
            table: ResourceTable::new(),
        },
    );
    let push_instance = extractor_child_bindings::ContentExtractor::instantiate(
        &mut push_store,
        &push_component,
        &push_linker,
    )
    .expect("extract instantiate");
    let push_result = push_instance
        .patina_records_extract()
        .call_extract(&mut push_store, &push_files)
        .expect("push call")
        .expect("push result");

    let composed = compose_component(
        &[
            ("ce", "patina:test:ce", ce_wasm.as_path()),
            (
                "ce-pando",
                "patina:test:ce-pando",
                ce_adapter_wasm.as_path(),
            ),
        ],
        &[("ce", "ce-pando", "patina:records/extract")],
        ("ce-pando", "patina:pando/extract"),
    )
    .expect("compose extract");

    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = wasmtime::Engine::new(&config).expect("engine");
    let component = Component::new(&engine, composed).expect("component");
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("wasi linker");
    parity_extract_bindings::ExtractRunner::add_to_linker::<
        ExtractHostState,
        wasmtime::component::HasSelf<ExtractHostState>,
    >(&mut linker, |state| state)
    .expect("extract-runner linker");
    let mut wasi = wasmtime_wasi::WasiCtxBuilder::new();
    wasi.preopened_dir(
        input.path(),
        "/input",
        wasmtime_wasi::DirPerms::READ,
        wasmtime_wasi::FilePerms::READ,
    )
    .expect("preopen input");
    let mut store = Store::new(
        &engine,
        ExtractHostState {
            wasi: wasi.build(),
            table: ResourceTable::new(),
            upstream_files: parity_files.to_vec(),
        },
    );
    let instance =
        parity_extract_bindings::ExtractRunner::instantiate(&mut store, &component, &linker)
            .expect("extract-runner instantiate");
    let composed_result = instance
        .patina_pando_extract()
        .call_run(&mut store)
        .expect("composed call")
        .expect("composed result");

    assert_eq!(push_result.len(), composed_result.len());
    assert_eq!(
        extract_by_path_push(&push_result),
        extract_by_path_parity(&composed_result)
    );
}
