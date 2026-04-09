use std::collections::HashMap;
use std::path::{Path, PathBuf};

use wac_graph::{types::Package, CompositionGraph, EncodeOptions};
use wasmtime::component::{Component, Linker, Resource, ResourceTable};
use wasmtime::Store;

mod parity_schema_bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/pando-parity/wit",
        world: "schema-runner",
    });
}

mod parity_dedup_bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/pando-parity/wit",
        world: "dedup-runner",
    });
}

mod schema_child_bindings {
    wasmtime::component::bindgen!({
        path: "children/schema-enforcer/wit",
        world: "schema-enforcer",
    });
}

mod dedup_child_bindings {
    wasmtime::component::bindgen!({
        path: "children/dedup-filter/wit",
        world: "dedup-filter",
    });
}

#[derive(Clone)]
struct FixtureRecord {
    record_id: &'static str,
    source_path: &'static str,
    source_hash: &'static str,
    content: &'static str,
    content_hash: &'static str,
}

fn to_schema_parity_record(
    f: &FixtureRecord,
) -> parity_schema_bindings::patina::records::types::RecordEnvelope {
    parity_schema_bindings::patina::records::types::RecordEnvelope {
        record_id: f.record_id.to_string(),
        source_path: f.source_path.to_string(),
        source_hash: f.source_hash.to_string(),
        source_modified_at: "2026-04-09T00:00:00Z".to_string(),
        source_size_bytes: f.content.len() as u64,
        content: f.content.to_string(),
        content_hash: f.content_hash.to_string(),
        content_type: "text/plain".to_string(),
        encoding: "utf-8".to_string(),
        line_count: 1,
        ingested_at: "2026-04-09T00:00:00Z".to_string(),
        batch_id: "batch-1".to_string(),
        schema_version: 1,
    }
}

fn to_dedup_parity_record(
    f: &FixtureRecord,
) -> parity_dedup_bindings::patina::records::types::RecordEnvelope {
    parity_dedup_bindings::patina::records::types::RecordEnvelope {
        record_id: f.record_id.to_string(),
        source_path: f.source_path.to_string(),
        source_hash: f.source_hash.to_string(),
        source_modified_at: "2026-04-09T00:00:00Z".to_string(),
        source_size_bytes: f.content.len() as u64,
        content: f.content.to_string(),
        content_hash: f.content_hash.to_string(),
        content_type: "text/plain".to_string(),
        encoding: "utf-8".to_string(),
        line_count: 1,
        ingested_at: "2026-04-09T00:00:00Z".to_string(),
        batch_id: "batch-1".to_string(),
        schema_version: 1,
    }
}

fn to_schema_push_record(
    f: &FixtureRecord,
) -> schema_child_bindings::patina::records::types::RecordEnvelope {
    schema_child_bindings::patina::records::types::RecordEnvelope {
        record_id: f.record_id.to_string(),
        source_path: f.source_path.to_string(),
        source_hash: f.source_hash.to_string(),
        source_modified_at: "2026-04-09T00:00:00Z".to_string(),
        source_size_bytes: f.content.len() as u64,
        content: f.content.to_string(),
        content_hash: f.content_hash.to_string(),
        content_type: "text/plain".to_string(),
        encoding: "utf-8".to_string(),
        line_count: 1,
        ingested_at: "2026-04-09T00:00:00Z".to_string(),
        batch_id: "batch-1".to_string(),
        schema_version: 1,
    }
}

fn to_dedup_push_record(
    f: &FixtureRecord,
) -> dedup_child_bindings::patina::records::types::RecordEnvelope {
    dedup_child_bindings::patina::records::types::RecordEnvelope {
        record_id: f.record_id.to_string(),
        source_path: f.source_path.to_string(),
        source_hash: f.source_hash.to_string(),
        source_modified_at: "2026-04-09T00:00:00Z".to_string(),
        source_size_bytes: f.content.len() as u64,
        content: f.content.to_string(),
        content_hash: f.content_hash.to_string(),
        content_type: "text/plain".to_string(),
        encoding: "utf-8".to_string(),
        line_count: 1,
        ingested_at: "2026-04-09T00:00:00Z".to_string(),
        batch_id: "batch-1".to_string(),
        schema_version: 1,
    }
}

fn child_wasm_path(crate_stem: &str) -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        format!("target/wasm32-wasip2/debug/patina_ai_child_{crate_stem}.wasm"),
        format!("target/wasm32-wasip2/release/patina_ai_child_{crate_stem}.wasm"),
    ] {
        let path = root.join(rel);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn adapter_wasm_path(crate_stem: &str) -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        format!("target/wasm32-wasip2/debug/patina_ai_adapter_{crate_stem}.wasm"),
        format!("target/wasm32-wasip2/release/patina_ai_adapter_{crate_stem}.wasm"),
    ] {
        let path = root.join(rel);
        if path.exists() {
            return Some(path);
        }
    }
    None
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

fn compose_component(
    packages: &[(&str, &str, &Path)],
    wiring: &[(&str, &str, &str)],
    entry: (&str, &str),
) -> anyhow::Result<Vec<u8>> {
    let mut graph = CompositionGraph::new();
    let mut instance_ids = HashMap::new();

    for (instance_name, package_name, wasm_path) in packages {
        let package = Package::from_file(package_name, None, wasm_path, graph.types_mut())?;
        let package_id = graph.register_package(package)?;
        let instance_id = graph.instantiate(package_id);
        instance_ids.insert((*instance_name).to_string(), instance_id);
    }

    for (from, to, toy) in wiring {
        let from_id = *instance_ids
            .get(*from)
            .ok_or_else(|| anyhow::anyhow!("missing from instance {}", from))?;
        let to_id = *instance_ids
            .get(*to)
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
        .get(entry.0)
        .ok_or_else(|| anyhow::anyhow!("missing entry instance {}", entry.0))?;
    let mut exported = false;
    for candidate in typed_interface_candidates(entry.1) {
        let Ok(export_id) = graph.alias_instance_export(entry_id, &candidate) else {
            continue;
        };
        if graph.export(export_id, &candidate).is_ok() {
            exported = true;
            break;
        }
    }
    if !exported {
        anyhow::bail!("failed to export entry interface {}", entry.1);
    }

    Ok(graph.encode(EncodeOptions::default())?)
}

struct SchemaHostState {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
    upstream_records: Vec<parity_schema_bindings::patina::records::types::RecordEnvelope>,
}

impl wasmtime_wasi::WasiView for SchemaHostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl parity_schema_bindings::patina::pando::extract::Host for SchemaHostState {
    fn run(
        &mut self,
    ) -> Result<Vec<parity_schema_bindings::patina::records::types::RecordEnvelope>, String> {
        Ok(self.upstream_records.clone())
    }
}

impl parity_schema_bindings::wasi::logging::logging::Host for SchemaHostState {
    fn log(
        &mut self,
        _level: parity_schema_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl parity_schema_bindings::patina::measure::measure::Host for SchemaHostState {
    fn emit(
        &mut self,
        _metric: parity_schema_bindings::patina::measure::measure::Metric,
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

impl parity_schema_bindings::patina::records::types::Host for SchemaHostState {}

struct BucketHandle {
    identifier: String,
}

struct DedupHostState {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
    upstream_records: Vec<parity_dedup_bindings::patina::records::types::RecordEnvelope>,
    kv: HashMap<String, Vec<u8>>,
}

impl wasmtime_wasi::WasiView for DedupHostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl parity_dedup_bindings::patina::pando::extract::Host for DedupHostState {
    fn run(
        &mut self,
    ) -> Result<Vec<parity_dedup_bindings::patina::records::types::RecordEnvelope>, String> {
        Ok(self.upstream_records.clone())
    }
}

impl parity_dedup_bindings::wasi::logging::logging::Host for DedupHostState {
    fn log(
        &mut self,
        _level: parity_dedup_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl parity_dedup_bindings::patina::measure::measure::Host for DedupHostState {
    fn emit(
        &mut self,
        _metric: parity_dedup_bindings::patina::measure::measure::Metric,
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

impl parity_dedup_bindings::wasi::keyvalue::store::Host for DedupHostState {
    fn open(
        &mut self,
        identifier: String,
    ) -> Result<
        Resource<parity_dedup_bindings::wasi::keyvalue::store::Bucket>,
        parity_dedup_bindings::wasi::keyvalue::store::Error,
    > {
        let rep = self.table.push(BucketHandle { identifier }).map_err(|e| {
            parity_dedup_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(Resource::new_own(rep.rep()))
    }
}

impl parity_dedup_bindings::wasi::keyvalue::store::HostBucket for DedupHostState {
    fn set(
        &mut self,
        bucket: Resource<parity_dedup_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
        value: Vec<u8>,
    ) -> Result<(), parity_dedup_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            parity_dedup_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        self.kv
            .insert(format!("{}:{}", handle.identifier, key), value);
        Ok(())
    }

    fn exists(
        &mut self,
        bucket: Resource<parity_dedup_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
    ) -> Result<bool, parity_dedup_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            parity_dedup_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(self
            .kv
            .contains_key(&format!("{}:{}", handle.identifier, key)))
    }

    fn drop(
        &mut self,
        bucket: Resource<parity_dedup_bindings::wasi::keyvalue::store::Bucket>,
    ) -> wasmtime::Result<()> {
        let owned = Resource::<BucketHandle>::new_own(bucket.rep());
        Ok(self.table.delete(owned).map(|_| ())?)
    }
}

impl parity_dedup_bindings::patina::records::types::Host for DedupHostState {}

struct PushSchemaHost {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
}

impl wasmtime_wasi::WasiView for PushSchemaHost {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl schema_child_bindings::wasi::logging::logging::Host for PushSchemaHost {
    fn log(
        &mut self,
        _level: schema_child_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl schema_child_bindings::patina::measure::measure::Host for PushSchemaHost {
    fn emit(
        &mut self,
        _metric: schema_child_bindings::patina::measure::measure::Metric,
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

impl schema_child_bindings::patina::records::types::Host for PushSchemaHost {}

struct PushDedupHost {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
    kv: HashMap<String, Vec<u8>>,
}

impl wasmtime_wasi::WasiView for PushDedupHost {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl dedup_child_bindings::wasi::logging::logging::Host for PushDedupHost {
    fn log(
        &mut self,
        _level: dedup_child_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl dedup_child_bindings::patina::measure::measure::Host for PushDedupHost {
    fn emit(
        &mut self,
        _metric: dedup_child_bindings::patina::measure::measure::Metric,
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

impl dedup_child_bindings::wasi::keyvalue::store::Host for PushDedupHost {
    fn open(
        &mut self,
        identifier: String,
    ) -> Result<
        Resource<dedup_child_bindings::wasi::keyvalue::store::Bucket>,
        dedup_child_bindings::wasi::keyvalue::store::Error,
    > {
        let rep = self.table.push(BucketHandle { identifier }).map_err(|e| {
            dedup_child_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(Resource::new_own(rep.rep()))
    }
}

impl dedup_child_bindings::wasi::keyvalue::store::HostBucket for PushDedupHost {
    fn set(
        &mut self,
        bucket: Resource<dedup_child_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
        value: Vec<u8>,
    ) -> Result<(), dedup_child_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            dedup_child_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        self.kv
            .insert(format!("{}:{}", handle.identifier, key), value);
        Ok(())
    }

    fn exists(
        &mut self,
        bucket: Resource<dedup_child_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
    ) -> Result<bool, dedup_child_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            dedup_child_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(self
            .kv
            .contains_key(&format!("{}:{}", handle.identifier, key)))
    }

    fn drop(
        &mut self,
        bucket: Resource<dedup_child_bindings::wasi::keyvalue::store::Bucket>,
    ) -> wasmtime::Result<()> {
        let owned = Resource::<BucketHandle>::new_own(bucket.rep());
        Ok(self.table.delete(owned).map(|_| ())?)
    }
}

impl dedup_child_bindings::patina::records::types::Host for PushDedupHost {}

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

#[test]
fn dedup_filter_push_equals_composed_with_mocked_upstream() {
    let Some(schema_wasm) = child_wasm_path("schema_enforcer") else {
        return;
    };
    let Some(dedup_wasm) = child_wasm_path("dedup_filter") else {
        return;
    };
    let Some(se_adapter_wasm) = adapter_wasm_path("se_pando") else {
        return;
    };
    let Some(df_adapter_wasm) = adapter_wasm_path("df_pando") else {
        return;
    };

    let fixtures = [
        FixtureRecord {
            record_id: "r1",
            source_path: "alpha.txt",
            source_hash: "hash-a",
            content: "Hello Alpha",
            content_hash: "same-content",
        },
        FixtureRecord {
            record_id: "r2",
            source_path: "beta.txt",
            source_hash: "hash-b",
            content: "Hello Beta",
            content_hash: "same-content",
        },
    ];

    let push_input = fixtures
        .iter()
        .map(to_dedup_push_record)
        .collect::<Vec<_>>();

    let mut push_config = wasmtime::Config::new();
    push_config.wasm_component_model(true);
    let push_engine = wasmtime::Engine::new(&push_config).expect("push engine");
    let push_component = Component::from_file(&push_engine, &dedup_wasm).expect("dedup child");
    let mut push_linker = Linker::new(&push_engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut push_linker).expect("wasi linker");
    dedup_child_bindings::DedupFilter::add_to_linker::<
        PushDedupHost,
        wasmtime::component::HasSelf<PushDedupHost>,
    >(&mut push_linker, |state| state)
    .expect("dedup linker");
    let mut push_store = Store::new(
        &push_engine,
        PushDedupHost {
            wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
            table: ResourceTable::new(),
            kv: HashMap::new(),
        },
    );
    let push_instance = dedup_child_bindings::DedupFilter::instantiate(
        &mut push_store,
        &push_component,
        &push_linker,
    )
    .expect("dedup instantiate");
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
            ("df", "patina:test:df", dedup_wasm.as_path()),
            (
                "dedup-transform",
                "patina:test:df-pando",
                df_adapter_wasm.as_path(),
            ),
        ],
        &[
            ("se", "schema-transform", "patina:records/transform"),
            (
                "schema-transform",
                "dedup-transform",
                "patina:pando/transform",
            ),
            ("df", "dedup-transform", "patina:records/transform"),
        ],
        ("dedup-transform", "patina:pando/transform"),
    )
    .expect("compose dedup");

    let upstream = fixtures
        .iter()
        .map(to_dedup_parity_record)
        .collect::<Vec<_>>();
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = wasmtime::Engine::new(&config).expect("engine");
    let component = Component::new(&engine, composed).expect("component");
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("wasi linker");
    parity_dedup_bindings::DedupRunner::add_to_linker::<
        DedupHostState,
        wasmtime::component::HasSelf<DedupHostState>,
    >(&mut linker, |state| state)
    .expect("dedup-runner linker");
    let mut store = Store::new(
        &engine,
        DedupHostState {
            wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
            table: ResourceTable::new(),
            upstream_records: upstream,
            kv: HashMap::new(),
        },
    );
    let instance = parity_dedup_bindings::DedupRunner::instantiate(&mut store, &component, &linker)
        .expect("dedup-runner instantiate");
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
    assert_eq!(push_result.rejected[0].envelope.source_path, "beta.txt");
    assert_eq!(composed_result.rejected[0].envelope.source_path, "beta.txt");
}
