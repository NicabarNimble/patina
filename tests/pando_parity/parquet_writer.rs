use super::common::*;

#[test]
fn parquet_writer_push_equals_composed_with_mocked_upstream() {
    let Some(rw_wasm) = child_wasm_path("parquet_writer") else {
        return;
    };
    let Some(rw_adapter_wasm) = adapter_wasm_path("rw_pando") else {
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
            record_id: "r2",
            source_path: "beta.txt",
            source_hash: "hash-b",
            content: "Hello Beta",
            content_hash: "content-b",
        },
    ];

    let push_input = fixtures
        .iter()
        .map(to_write_push_record)
        .collect::<Vec<_>>();
    let upstream_records = fixtures
        .iter()
        .map(to_write_parity_record)
        .collect::<Vec<_>>();

    let push_output = TempDir::new().expect("push output");
    let composed_output = TempDir::new().expect("composed output");

    let mut push_config = wasmtime::Config::new();
    push_config.wasm_component_model(true);
    let push_engine = wasmtime::Engine::new(&push_config).expect("push engine");
    let push_component = Component::from_file(&push_engine, &rw_wasm).expect("write child");
    let mut push_linker = Linker::new(&push_engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut push_linker).expect("wasi linker");
    write_child_bindings::ParquetWriter::add_to_linker::<
        PushWriteHost,
        wasmtime::component::HasSelf<PushWriteHost>,
    >(&mut push_linker, |state| state)
    .expect("write linker");
    let mut push_wasi = wasmtime_wasi::WasiCtxBuilder::new();
    push_wasi
        .preopened_dir(
            push_output.path(),
            "/output",
            wasmtime_wasi::DirPerms::READ | wasmtime_wasi::DirPerms::MUTATE,
            wasmtime_wasi::FilePerms::READ | wasmtime_wasi::FilePerms::WRITE,
        )
        .expect("preopen output");
    let mut push_store = Store::new(
        &push_engine,
        PushWriteHost {
            wasi: push_wasi.build(),
            table: ResourceTable::new(),
            kv: HashMap::new(),
        },
    );
    let push_instance = write_child_bindings::ParquetWriter::instantiate(
        &mut push_store,
        &push_component,
        &push_linker,
    )
    .expect("write instantiate");
    let push_result = push_instance
        .patina_records_write()
        .call_write(&mut push_store, &push_input)
        .expect("push call")
        .expect("push result");

    let composed = compose_component(
        &[
            ("rw", "patina:test:rw", rw_wasm.as_path()),
            (
                "rw-pando",
                "patina:test:rw-pando",
                rw_adapter_wasm.as_path(),
            ),
        ],
        &[("rw", "rw-pando", "patina:records/write")],
        ("rw-pando", "patina:pando/write"),
    )
    .expect("compose write");

    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = wasmtime::Engine::new(&config).expect("engine");
    let component = Component::new(&engine, composed).expect("component");
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("wasi linker");
    parity_write_bindings::WriteRunner::add_to_linker::<
        WriteHostState,
        wasmtime::component::HasSelf<WriteHostState>,
    >(&mut linker, |state| state)
    .expect("write-runner linker");
    let mut wasi = wasmtime_wasi::WasiCtxBuilder::new();
    wasi.preopened_dir(
        composed_output.path(),
        "/output",
        wasmtime_wasi::DirPerms::READ | wasmtime_wasi::DirPerms::MUTATE,
        wasmtime_wasi::FilePerms::READ | wasmtime_wasi::FilePerms::WRITE,
    )
    .expect("preopen output");
    let mut store = Store::new(
        &engine,
        WriteHostState {
            wasi: wasi.build(),
            table: ResourceTable::new(),
            upstream: parity_write_bindings::patina::records::types::TransformResult {
                accepted: upstream_records,
                rejected: Vec::new(),
            },
            kv: HashMap::new(),
        },
    );
    let instance = parity_write_bindings::WriteRunner::instantiate(&mut store, &component, &linker)
        .expect("write-runner instantiate");
    let composed_result = instance
        .patina_pando_write()
        .call_run(&mut store)
        .expect("composed call")
        .expect("composed result");

    assert_eq!(push_result.len(), 1);
    assert_eq!(composed_result.len(), 1);
    assert_eq!(push_result[0].record_count, fixtures.len() as u64);
    assert_eq!(composed_result[0].record_count, fixtures.len() as u64);

    let push_parquet = guest_to_host_output_path(&push_result[0].file_path, push_output.path());
    let composed_parquet =
        guest_to_host_output_path(&composed_result[0].file_path, composed_output.path());
    assert!(push_parquet.exists(), "push parquet file missing");
    assert!(composed_parquet.exists(), "composed parquet file missing");
    assert_eq!(
        parquet_row_count(&push_parquet),
        parquet_row_count(&composed_parquet)
    );
}
