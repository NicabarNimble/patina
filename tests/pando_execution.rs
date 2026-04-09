use std::collections::HashMap;
use std::path::{Path, PathBuf};

use wac_graph::{types::Package, CompositionGraph, EncodeOptions};
use wasmtime::component::{Component, Linker, Resource, ResourceTable};
use wasmtime::Store;

mod catalog_bindings {
    wasmtime::component::bindgen!({
        path: "wit/pando",
        world: "catalog-runner",
    });
}

#[derive(Clone)]
struct BucketHandle {
    identifier: String,
}

struct HostState {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
    config: HashMap<String, String>,
    kv: HashMap<String, Vec<u8>>,
}

impl wasmtime_wasi::WasiView for HostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl catalog_bindings::wasi::logging::logging::Host for HostState {
    fn log(
        &mut self,
        _level: catalog_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl catalog_bindings::patina::measure::measure::Host for HostState {
    fn emit(
        &mut self,
        _metric: catalog_bindings::patina::measure::measure::Metric,
    ) -> Result<(), String> {
        Ok(())
    }

    fn gauge(&mut self, _name: String, _value: f64) -> Result<(), String> {
        Ok(())
    }

    fn counter(&mut self, _name: String, _delta: f64) -> Result<(), String> {
        Ok(())
    }
}

impl catalog_bindings::patina::config::config::Host for HostState {
    fn get(&mut self, key: String) -> Result<String, String> {
        self.config
            .get(&key)
            .cloned()
            .ok_or_else(|| format!("missing config key '{}'", key))
    }
}

impl catalog_bindings::wasi::keyvalue::store::Host for HostState {
    fn open(
        &mut self,
        identifier: String,
    ) -> Result<
        Resource<catalog_bindings::wasi::keyvalue::store::Bucket>,
        catalog_bindings::wasi::keyvalue::store::Error,
    > {
        let rep = self
            .table
            .push(BucketHandle { identifier })
            .map_err(|e| catalog_bindings::wasi::keyvalue::store::Error::Other(e.to_string()))?;
        Ok(Resource::new_own(rep.rep()))
    }
}

impl catalog_bindings::wasi::keyvalue::store::HostBucket for HostState {
    fn get(
        &mut self,
        bucket: Resource<catalog_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
    ) -> Result<Option<Vec<u8>>, catalog_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self
            .table
            .get(&handle_ref)
            .map_err(|e| catalog_bindings::wasi::keyvalue::store::Error::Other(e.to_string()))?;
        Ok(self
            .kv
            .get(&format!("{}:{}", handle.identifier, key))
            .cloned())
    }

    fn set(
        &mut self,
        bucket: Resource<catalog_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
        value: Vec<u8>,
    ) -> Result<(), catalog_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self
            .table
            .get(&handle_ref)
            .map_err(|e| catalog_bindings::wasi::keyvalue::store::Error::Other(e.to_string()))?;
        self.kv
            .insert(format!("{}:{}", handle.identifier, key), value);
        Ok(())
    }

    fn exists(
        &mut self,
        bucket: Resource<catalog_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
    ) -> Result<bool, catalog_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self
            .table
            .get(&handle_ref)
            .map_err(|e| catalog_bindings::wasi::keyvalue::store::Error::Other(e.to_string()))?;
        Ok(self
            .kv
            .contains_key(&format!("{}:{}", handle.identifier, key)))
    }

    fn drop(
        &mut self,
        bucket: Resource<catalog_bindings::wasi::keyvalue::store::Bucket>,
    ) -> wasmtime::Result<()> {
        let owned = Resource::<BucketHandle>::new_own(bucket.rep());
        Ok(self.table.delete(owned).map(|_| ())?)
    }
}

impl catalog_bindings::patina::records::types::Host for HostState {}

fn with_temp_patina_home<T>(f: impl FnOnce(&Path) -> T) -> T {
    let _guard = patina::test_support::env_test_mutex()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let temp = tempfile::TempDir::new().expect("temp dir");
    let home = temp.path().join("patina-home");
    std::fs::create_dir_all(&home).expect("create home");
    let old_home = std::env::var_os("PATINA_HOME");
    unsafe {
        std::env::set_var("PATINA_HOME", &home);
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&home)));
    match old_home {
        Some(value) => unsafe {
            std::env::set_var("PATINA_HOME", value);
        },
        None => unsafe {
            std::env::remove_var("PATINA_HOME");
        },
    }
    match result {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

fn component_wasm_path(name: &str) -> Option<PathBuf> {
    let stem = name.replace('-', "_");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        root.join(format!(
            "target/wasm32-wasip2/debug/patina_ai_child_{stem}.wasm"
        )),
        root.join(format!(
            "target/wasm32-wasip2/release/patina_ai_child_{stem}.wasm"
        )),
        root.join(format!(
            "target/wasm32-wasip2/debug/patina_ai_adapter_{stem}.wasm"
        )),
        root.join(format!(
            "target/wasm32-wasip2/release/patina_ai_adapter_{stem}.wasm"
        )),
    ];
    candidates.into_iter().find(|path| path.exists())
}

fn typed_interface_candidates(interface_name: &str) -> Vec<String> {
    if interface_name.contains('@') {
        vec![interface_name.to_string()]
    } else {
        vec![
            interface_name.to_string(),
            format!("{}@0.1.0", interface_name),
        ]
    }
}

fn compose_folder_text_to_parquet() -> anyhow::Result<Vec<u8>> {
    let children = [
        ("fsm", "file-system-monitor"),
        ("fsm-pando", "fsm-pando"),
        ("ce", "content-extractor"),
        ("ce-pando", "ce-pando"),
        ("se", "schema-enforcer"),
        ("schema-transform", "se-pando"),
        ("df", "dedup-filter"),
        ("dedup-transform", "df-pando"),
        ("rw", "record-writer"),
        ("rw-pando", "rw-pando"),
        ("lc", "lakehouse-catalog"),
        ("lc-pando", "lc-pando"),
    ];
    let wiring = [
        ("fsm", "fsm-pando", "patina:records/source"),
        ("fsm-pando", "ce-pando", "patina:pando/source"),
        ("ce", "ce-pando", "patina:records/extract"),
        ("ce-pando", "schema-transform", "patina:pando/extract"),
        ("se", "schema-transform", "patina:records/transform"),
        (
            "schema-transform",
            "dedup-transform",
            "patina:pando/transform",
        ),
        ("df", "dedup-transform", "patina:records/transform"),
        ("dedup-transform", "rw-pando", "patina:pando/transform"),
        ("rw", "rw-pando", "patina:records/write"),
        ("rw-pando", "lc-pando", "patina:pando/write"),
        ("lc", "lc-pando", "patina:records/catalog"),
    ];

    let mut graph = CompositionGraph::new();
    let mut instance_ids = HashMap::new();
    for (instance, name) in children {
        let wasm_path = component_wasm_path(name)
            .ok_or_else(|| anyhow::anyhow!("missing component wasm for {}", name))?;
        let package = Package::from_file(
            &format!("patina:test:{}", instance),
            None,
            &wasm_path,
            graph.types_mut(),
        )?;
        let package_id = graph.register_package(package)?;
        let instance_id = graph.instantiate(package_id);
        instance_ids.insert(instance.to_string(), instance_id);
    }

    for (from, to, toy) in wiring {
        let from_id = *instance_ids
            .get(from)
            .ok_or_else(|| anyhow::anyhow!("missing from instance {}", from))?;
        let to_id = *instance_ids
            .get(to)
            .ok_or_else(|| anyhow::anyhow!("missing to instance {}", to))?;
        let mut wired = false;
        for candidate in typed_interface_candidates(toy) {
            let Ok(export_id) = graph.alias_instance_export(from_id, &candidate) else {
                continue;
            };
            if graph
                .set_instantiation_argument(to_id, &candidate, export_id)
                .is_ok()
            {
                wired = true;
                break;
            }
        }
        if !wired {
            anyhow::bail!("failed to wire {from}->{to} on {toy}");
        }
    }

    let entry_id = *instance_ids
        .get("lc-pando")
        .ok_or_else(|| anyhow::anyhow!("missing entry instance"))?;
    let mut exported = false;
    for candidate in typed_interface_candidates("patina:pando/catalog") {
        let Ok(export_id) = graph.alias_instance_export(entry_id, &candidate) else {
            continue;
        };
        if graph.export(export_id, &candidate).is_ok() {
            exported = true;
            break;
        }
    }
    if !exported {
        anyhow::bail!("failed to export catalog entry");
    }

    Ok(graph.encode(EncodeOptions::default())?)
}

#[test]
fn folder_text_to_parquet_catalog_run_end_to_end() {
    with_temp_patina_home(|home| {
        let input_dir = home.join("input");
        std::fs::create_dir_all(&input_dir).expect("create input dir");
        std::fs::write(input_dir.join("alpha.txt"), "Hello Alpha").expect("write alpha");
        std::fs::write(input_dir.join("beta.txt"), "Hello Beta").expect("write beta");
        std::fs::write(input_dir.join("gamma.txt"), "Hello Gamma").expect("write gamma");

        let output_dir = home
            .join("pandos")
            .join("folder-text-to-parquet")
            .join("output");
        std::fs::create_dir_all(&output_dir).expect("create output dir");

        let Ok(composed_bytes) = compose_folder_text_to_parquet() else {
            return;
        };

        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config).expect("engine");
        let component = Component::new(&engine, &composed_bytes).expect("component");
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("wasi linker");
        catalog_bindings::CatalogRunner::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(&mut linker, |state| state)
        .expect("catalog linker");

        let mut wasi = wasmtime_wasi::WasiCtxBuilder::new();
        wasi.preopened_dir(
            &input_dir,
            "/input",
            wasmtime_wasi::DirPerms::READ,
            wasmtime_wasi::FilePerms::READ,
        )
        .expect("preopen input");
        wasi.preopened_dir(
            &output_dir,
            "/output",
            wasmtime_wasi::DirPerms::READ | wasmtime_wasi::DirPerms::MUTATE,
            wasmtime_wasi::FilePerms::READ | wasmtime_wasi::FilePerms::WRITE,
        )
        .expect("preopen output");

        let mut store = Store::new(
            &engine,
            HostState {
                wasi: wasi.build(),
                table: ResourceTable::new(),
                config: HashMap::from([("folder_path".to_string(), "/input".to_string())]),
                kv: HashMap::new(),
            },
        );

        let instance =
            catalog_bindings::CatalogRunner::instantiate(&mut store, &component, &linker)
                .expect("catalog instantiate");
        let catalog_entries = instance
            .patina_pando_catalog()
            .call_run(&mut store)
            .expect("catalog call")
            .expect("catalog result");

        assert_eq!(catalog_entries.len(), 1);
        assert_eq!(catalog_entries[0].record_count, 3);
        let written_path =
            if let Some(relative) = catalog_entries[0].file_path.strip_prefix("/output/") {
                output_dir.join(relative)
            } else {
                PathBuf::from(&catalog_entries[0].file_path)
            };
        assert!(written_path.exists(), "missing parquet output");
        assert!(written_path.starts_with(&output_dir));
        assert!(!catalog_entries[0].file_path.starts_with("/tmp/patina/"));
        assert_eq!(catalog_entries[0].record_count, 3);

        if let Ok(bin) = std::env::var("CARGO_BIN_EXE_patina") {
            let output = std::process::Command::new(bin)
                .arg("spec")
                .arg("list")
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .output()
                .expect("run patina spec list");
            assert!(
                output.status.success(),
                "spec list failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    });
}
