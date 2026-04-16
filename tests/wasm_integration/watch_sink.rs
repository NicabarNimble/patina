use super::common::*;

#[test]
fn watch_null_sink_emit_contract_end_to_end() {
    let Some(wasm_path) = watch_null_sink_component_path() else {
        return;
    };

    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = wasmtime::Engine::new(&config).expect("engine");
    let component = wasmtime::component::Component::from_file(&engine, &wasm_path)
        .expect("load watch-null-sink component");

    #[derive(Default)]
    struct HostState {
        wasi: wasmtime_wasi::WasiCtx,
        table: wasmtime::component::ResourceTable,
        counters: Vec<(String, f64)>,
    }

    impl wasmtime_wasi::WasiView for HostState {
        fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
            wasmtime_wasi::WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.table,
            }
        }
    }

    impl watch_null_sink_bindings::wasi::logging::logging::Host for HostState {
        fn log(
            &mut self,
            _level: watch_null_sink_bindings::wasi::logging::logging::Level,
            _context: String,
            _message: String,
        ) {
        }
    }

    impl watch_null_sink_bindings::patina::measure::measure::Host for HostState {
        fn emit(
            &mut self,
            _metric: watch_null_sink_bindings::patina::measure::measure::Metric,
        ) -> Result<(), String> {
            Ok(())
        }

        fn gauge(&mut self, _name: String, _value: f64) -> Result<(), String> {
            Ok(())
        }

        fn counter(&mut self, name: String, delta: f64) -> Result<(), String> {
            self.counters.push((name, delta));
            Ok(())
        }
    }

    impl watch_null_sink_bindings::patina::watch::types::Host for HostState {}

    let mut linker = wasmtime::component::Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("add wasi linker bindings");
    watch_null_sink_bindings::WatchNullSink::add_to_linker::<
        HostState,
        wasmtime::component::HasSelf<HostState>,
    >(&mut linker, |state| state)
    .expect("add watch-null-sink linker bindings");

    let mut store = wasmtime::Store::new(
        &engine,
        HostState {
            wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
            table: wasmtime::component::ResourceTable::new(),
            counters: Vec::new(),
        },
    );

    let instance =
        watch_null_sink_bindings::WatchNullSink::instantiate(&mut store, &component, &linker)
            .expect("instantiate watch-null-sink");

    let call_result = instance
        .patina_watch_events()
        .call_emit(
            &mut store,
            &watch_null_sink_bindings::patina::watch::types::FileChange {
                watcher: "folder-watch-actor".to_string(),
                stream_name: "watch.folder".to_string(),
                change_kind: watch_null_sink_bindings::patina::watch::types::ChangeKind::Created,
                absolute_path: "/tmp/doc.txt".to_string(),
                relative_path: "doc.txt".to_string(),
                size_bytes: Some(11),
                modified_unix_ms: Some(1234),
                sha256: Some("deadbeef".to_string()),
                detected_at: "2026-04-14T00:00:00Z".to_string(),
            },
        )
        .expect("watch-null-sink emit should not trap");

    assert!(call_result.is_ok(), "watch-null-sink emit should succeed");
    assert!(
        store.data().counters.iter().any(|(name, delta)| {
            name == "watch_events_dropped" && (*delta - 1.0).abs() < f64::EPSILON
        }),
        "watch-null-sink should increment watch_events_dropped counter"
    );
}
