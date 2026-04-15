use super::common::*;

#[test]
fn file_system_monitor_push_equals_composed_with_config_injection() {
    let Some(fsm_wasm) = child_wasm_path("file_system_monitor") else {
        return;
    };
    let Some(fsm_adapter_wasm) = adapter_wasm_path("fsm_pando") else {
        return;
    };

    let input = TempDir::new().expect("temp input");
    std::fs::write(input.path().join("alpha.txt"), "Hello Alpha").expect("write alpha");
    std::fs::write(input.path().join("beta.md"), "Hello Beta").expect("write beta");
    std::fs::write(input.path().join("skip.bin"), [0_u8, 1, 2]).expect("write bin");
    std::fs::write(input.path().join(".hidden.txt"), "hidden").expect("write hidden");

    let mut push_config = wasmtime::Config::new();
    push_config.wasm_component_model(true);
    let push_engine = wasmtime::Engine::new(&push_config).expect("push engine");
    let push_component = Component::from_file(&push_engine, &fsm_wasm).expect("fsm child");
    let mut push_linker = Linker::new(&push_engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut push_linker).expect("wasi linker");
    fsm_child_bindings::FileSystemMonitor::add_to_linker::<
        PushFsmHost,
        wasmtime::component::HasSelf<PushFsmHost>,
    >(&mut push_linker, |state| state)
    .expect("fsm linker");
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
        PushFsmHost {
            wasi: push_wasi.build(),
            table: ResourceTable::new(),
        },
    );
    let push_instance = fsm_child_bindings::FileSystemMonitor::instantiate(
        &mut push_store,
        &push_component,
        &push_linker,
    )
    .expect("fsm instantiate");
    let push_result = push_instance
        .patina_records_source()
        .call_scan(&mut push_store, "/input")
        .expect("push call")
        .expect("push result");

    let composed = compose_component(
        &[
            ("fsm", "patina:test:fsm", fsm_wasm.as_path()),
            (
                "fsm-pando",
                "patina:test:fsm-pando",
                fsm_adapter_wasm.as_path(),
            ),
        ],
        &[("fsm", "fsm-pando", "patina:records/source")],
        ("fsm-pando", "patina:pando/source"),
    )
    .expect("compose source");

    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = wasmtime::Engine::new(&config).expect("engine");
    let component = Component::new(&engine, composed).expect("component");
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("wasi linker");
    parity_source_bindings::SourceRunner::add_to_linker::<
        SourceHostState,
        wasmtime::component::HasSelf<SourceHostState>,
    >(&mut linker, |state| state)
    .expect("source-runner linker");
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
        SourceHostState {
            wasi: wasi.build(),
            table: ResourceTable::new(),
            config: HashMap::from([("folder_path".to_string(), "/input".to_string())]),
        },
    );
    let instance =
        parity_source_bindings::SourceRunner::instantiate(&mut store, &component, &linker)
            .expect("source-runner instantiate");
    let composed_result = instance
        .patina_pando_source()
        .call_run(&mut store)
        .expect("composed call")
        .expect("composed result");

    assert_eq!(push_result.len(), 2);
    assert_eq!(composed_result.len(), 2);
    assert_eq!(
        discover_by_path_push(&push_result),
        discover_by_path_parity(&composed_result)
    );
}
