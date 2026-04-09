use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sha2::Digest;
use tempfile::TempDir;
use wac_graph::{types::Package, CompositionGraph, EncodeOptions};
use wasmtime::component::{Component, Linker, Resource, ResourceTable};
use wasmtime::Store;

mod parity_source_bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/pando-parity/wit",
        world: "source-runner",
    });
}

mod parity_extract_bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/pando-parity/wit",
        world: "extract-runner",
    });
}

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

mod parity_write_bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/pando-parity/wit",
        world: "write-runner",
    });
}

mod parity_catalog_bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/pando-parity/wit",
        world: "catalog-runner",
    });
}

mod fsm_child_bindings {
    wasmtime::component::bindgen!({
        path: "children/file-system-monitor/wit",
        world: "file-system-monitor",
    });
}

mod extractor_child_bindings {
    wasmtime::component::bindgen!({
        path: "children/content-extractor/wit",
        world: "content-extractor",
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

mod write_child_bindings {
    wasmtime::component::bindgen!({
        path: "children/record-writer/wit",
        world: "record-writer",
    });
}

mod catalog_child_bindings {
    wasmtime::component::bindgen!({
        path: "children/lakehouse-catalog/wit",
        world: "lakehouse-catalog",
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

fn to_extract_parity_file(
    source_path: &str,
    host_path: &Path,
) -> parity_extract_bindings::patina::records::types::FileFound {
    let bytes = std::fs::read(host_path).expect("read fixture file");
    let source_hash = format!("{:x}", sha2::Sha256::digest(&bytes));
    parity_extract_bindings::patina::records::types::FileFound {
        source_path: source_path.to_string(),
        source_hash,
        source_size_bytes: bytes.len() as u64,
        discovered_at: "2026-04-09T00:00:00Z".to_string(),
    }
}

fn to_extract_push_file(
    source_path: &str,
    host_path: &Path,
) -> extractor_child_bindings::patina::records::types::FileFound {
    let bytes = std::fs::read(host_path).expect("read fixture file");
    let source_hash = format!("{:x}", sha2::Sha256::digest(&bytes));
    extractor_child_bindings::patina::records::types::FileFound {
        source_path: source_path.to_string(),
        source_hash,
        source_size_bytes: bytes.len() as u64,
        discovered_at: "2026-04-09T00:00:00Z".to_string(),
    }
}

fn to_write_parity_record(
    f: &FixtureRecord,
) -> parity_write_bindings::patina::records::types::RecordEnvelope {
    parity_write_bindings::patina::records::types::RecordEnvelope {
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

fn to_write_push_record(
    f: &FixtureRecord,
) -> write_child_bindings::patina::records::types::RecordEnvelope {
    write_child_bindings::patina::records::types::RecordEnvelope {
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

fn to_catalog_parity_file(
    file_path: &str,
    record_count: u64,
) -> parity_catalog_bindings::patina::records::types::FileWritten {
    parity_catalog_bindings::patina::records::types::FileWritten {
        file_path: file_path.to_string(),
        record_count,
        written_at: "2026-04-09T00:00:00Z".to_string(),
    }
}

fn to_catalog_push_file(
    file_path: &str,
    record_count: u64,
) -> catalog_child_bindings::patina::records::types::FileWritten {
    catalog_child_bindings::patina::records::types::FileWritten {
        file_path: file_path.to_string(),
        record_count,
        written_at: "2026-04-09T00:00:00Z".to_string(),
    }
}

fn discover_by_path_parity(
    source: &[parity_source_bindings::patina::records::types::FileFound],
) -> HashMap<String, (String, u64)> {
    source
        .iter()
        .map(|file| {
            (
                file.source_path.clone(),
                (file.source_hash.clone(), file.source_size_bytes),
            )
        })
        .collect()
}

fn discover_by_path_push(
    source: &[fsm_child_bindings::patina::records::types::FileFound],
) -> HashMap<String, (String, u64)> {
    source
        .iter()
        .map(|file| {
            (
                file.source_path.clone(),
                (file.source_hash.clone(), file.source_size_bytes),
            )
        })
        .collect()
}

fn extract_by_path_parity(
    source: &[parity_extract_bindings::patina::records::types::RecordEnvelope],
) -> HashMap<String, (String, String, String, u64)> {
    source
        .iter()
        .map(|record| {
            (
                record.source_path.clone(),
                (
                    record.source_hash.clone(),
                    record.content.clone(),
                    record.content_hash.clone(),
                    record.source_size_bytes,
                ),
            )
        })
        .collect()
}

fn extract_by_path_push(
    source: &[extractor_child_bindings::patina::records::types::RecordEnvelope],
) -> HashMap<String, (String, String, String, u64)> {
    source
        .iter()
        .map(|record| {
            (
                record.source_path.clone(),
                (
                    record.source_hash.clone(),
                    record.content.clone(),
                    record.content_hash.clone(),
                    record.source_size_bytes,
                ),
            )
        })
        .collect()
}

fn guest_to_host_output_path(guest_file_path: &str, output_root: &Path) -> PathBuf {
    if let Some(relative) = guest_file_path.strip_prefix("/output/") {
        output_root.join(relative)
    } else {
        PathBuf::from(guest_file_path)
    }
}

fn parquet_row_count(path: &Path) -> i64 {
    let conn = duckdb::Connection::open_in_memory().expect("open duckdb");
    let escaped = path.to_string_lossy().replace('\'', "''");
    conn.query_row(
        &format!("SELECT COUNT(*) FROM read_parquet('{}')", escaped),
        [],
        |row| row.get(0),
    )
    .expect("query parquet row count")
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
    fn get(
        &mut self,
        bucket: Resource<dedup_child_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
    ) -> Result<Option<Vec<u8>>, dedup_child_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            dedup_child_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(self
            .kv
            .get(&format!("{}:{}", handle.identifier, key))
            .cloned())
    }

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

struct SourceHostState {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
    config: HashMap<String, String>,
}

impl wasmtime_wasi::WasiView for SourceHostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl parity_source_bindings::patina::config::config::Host for SourceHostState {
    fn get(&mut self, key: String) -> Result<String, String> {
        self.config
            .get(&key)
            .cloned()
            .ok_or_else(|| format!("missing config key '{}'", key))
    }
}

impl parity_source_bindings::wasi::logging::logging::Host for SourceHostState {
    fn log(
        &mut self,
        _level: parity_source_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl parity_source_bindings::patina::measure::measure::Host for SourceHostState {
    fn emit(
        &mut self,
        _metric: parity_source_bindings::patina::measure::measure::Metric,
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

impl parity_source_bindings::patina::records::types::Host for SourceHostState {}

struct PushFsmHost {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
}

impl wasmtime_wasi::WasiView for PushFsmHost {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl fsm_child_bindings::wasi::logging::logging::Host for PushFsmHost {
    fn log(
        &mut self,
        _level: fsm_child_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl fsm_child_bindings::patina::measure::measure::Host for PushFsmHost {
    fn emit(
        &mut self,
        _metric: fsm_child_bindings::patina::measure::measure::Metric,
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

impl fsm_child_bindings::patina::records::types::Host for PushFsmHost {}

struct ExtractHostState {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
    upstream_files: Vec<parity_extract_bindings::patina::records::types::FileFound>,
}

impl wasmtime_wasi::WasiView for ExtractHostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl parity_extract_bindings::patina::pando::source::Host for ExtractHostState {
    fn run(
        &mut self,
    ) -> Result<Vec<parity_extract_bindings::patina::records::types::FileFound>, String> {
        Ok(self.upstream_files.clone())
    }
}

impl parity_extract_bindings::wasi::logging::logging::Host for ExtractHostState {
    fn log(
        &mut self,
        _level: parity_extract_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl parity_extract_bindings::patina::records::types::Host for ExtractHostState {}

struct PushExtractHost {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
}

impl wasmtime_wasi::WasiView for PushExtractHost {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl extractor_child_bindings::wasi::logging::logging::Host for PushExtractHost {
    fn log(
        &mut self,
        _level: extractor_child_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl extractor_child_bindings::patina::records::types::Host for PushExtractHost {}

struct WriteHostState {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
    upstream: parity_write_bindings::patina::records::types::TransformResult,
    kv: HashMap<String, Vec<u8>>,
}

impl wasmtime_wasi::WasiView for WriteHostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl parity_write_bindings::patina::pando::transform::Host for WriteHostState {
    fn run(
        &mut self,
    ) -> Result<parity_write_bindings::patina::records::types::TransformResult, String> {
        Ok(self.upstream.clone())
    }
}

impl parity_write_bindings::wasi::logging::logging::Host for WriteHostState {
    fn log(
        &mut self,
        _level: parity_write_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl parity_write_bindings::patina::measure::measure::Host for WriteHostState {
    fn emit(
        &mut self,
        _metric: parity_write_bindings::patina::measure::measure::Metric,
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

impl parity_write_bindings::wasi::keyvalue::store::Host for WriteHostState {
    fn open(
        &mut self,
        identifier: String,
    ) -> Result<
        Resource<parity_write_bindings::wasi::keyvalue::store::Bucket>,
        parity_write_bindings::wasi::keyvalue::store::Error,
    > {
        let rep = self.table.push(BucketHandle { identifier }).map_err(|e| {
            parity_write_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(Resource::new_own(rep.rep()))
    }
}

impl parity_write_bindings::wasi::keyvalue::store::HostBucket for WriteHostState {
    fn set(
        &mut self,
        bucket: Resource<parity_write_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
        value: Vec<u8>,
    ) -> Result<(), parity_write_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            parity_write_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        self.kv
            .insert(format!("{}:{}", handle.identifier, key), value);
        Ok(())
    }

    fn exists(
        &mut self,
        bucket: Resource<parity_write_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
    ) -> Result<bool, parity_write_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            parity_write_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(self
            .kv
            .contains_key(&format!("{}:{}", handle.identifier, key)))
    }

    fn drop(
        &mut self,
        bucket: Resource<parity_write_bindings::wasi::keyvalue::store::Bucket>,
    ) -> wasmtime::Result<()> {
        let owned = Resource::<BucketHandle>::new_own(bucket.rep());
        Ok(self.table.delete(owned).map(|_| ())?)
    }
}

impl parity_write_bindings::patina::records::types::Host for WriteHostState {}

struct PushWriteHost {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
    kv: HashMap<String, Vec<u8>>,
}

impl wasmtime_wasi::WasiView for PushWriteHost {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl write_child_bindings::wasi::logging::logging::Host for PushWriteHost {
    fn log(
        &mut self,
        _level: write_child_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl write_child_bindings::patina::measure::measure::Host for PushWriteHost {
    fn emit(
        &mut self,
        _metric: write_child_bindings::patina::measure::measure::Metric,
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

impl write_child_bindings::wasi::keyvalue::store::Host for PushWriteHost {
    fn open(
        &mut self,
        identifier: String,
    ) -> Result<
        Resource<write_child_bindings::wasi::keyvalue::store::Bucket>,
        write_child_bindings::wasi::keyvalue::store::Error,
    > {
        let rep = self.table.push(BucketHandle { identifier }).map_err(|e| {
            write_child_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(Resource::new_own(rep.rep()))
    }
}

impl write_child_bindings::wasi::keyvalue::store::HostBucket for PushWriteHost {
    fn get(
        &mut self,
        bucket: Resource<write_child_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
    ) -> Result<Option<Vec<u8>>, write_child_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            write_child_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(self
            .kv
            .get(&format!("{}:{}", handle.identifier, key))
            .cloned())
    }

    fn set(
        &mut self,
        bucket: Resource<write_child_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
        value: Vec<u8>,
    ) -> Result<(), write_child_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            write_child_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        self.kv
            .insert(format!("{}:{}", handle.identifier, key), value);
        Ok(())
    }

    fn exists(
        &mut self,
        bucket: Resource<write_child_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
    ) -> Result<bool, write_child_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            write_child_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(self
            .kv
            .contains_key(&format!("{}:{}", handle.identifier, key)))
    }

    fn drop(
        &mut self,
        bucket: Resource<write_child_bindings::wasi::keyvalue::store::Bucket>,
    ) -> wasmtime::Result<()> {
        let owned = Resource::<BucketHandle>::new_own(bucket.rep());
        Ok(self.table.delete(owned).map(|_| ())?)
    }
}

impl write_child_bindings::patina::records::types::Host for PushWriteHost {}

struct CatalogHostState {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
    upstream_files: Vec<parity_catalog_bindings::patina::records::types::FileWritten>,
    kv: HashMap<String, Vec<u8>>,
}

impl wasmtime_wasi::WasiView for CatalogHostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl parity_catalog_bindings::patina::pando::write::Host for CatalogHostState {
    fn run(
        &mut self,
    ) -> Result<Vec<parity_catalog_bindings::patina::records::types::FileWritten>, String> {
        Ok(self.upstream_files.clone())
    }
}

impl parity_catalog_bindings::wasi::logging::logging::Host for CatalogHostState {
    fn log(
        &mut self,
        _level: parity_catalog_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl parity_catalog_bindings::wasi::keyvalue::store::Host for CatalogHostState {
    fn open(
        &mut self,
        identifier: String,
    ) -> Result<
        Resource<parity_catalog_bindings::wasi::keyvalue::store::Bucket>,
        parity_catalog_bindings::wasi::keyvalue::store::Error,
    > {
        let rep = self.table.push(BucketHandle { identifier }).map_err(|e| {
            parity_catalog_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(Resource::new_own(rep.rep()))
    }
}

impl parity_catalog_bindings::wasi::keyvalue::store::HostBucket for CatalogHostState {
    fn set(
        &mut self,
        bucket: Resource<parity_catalog_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
        value: Vec<u8>,
    ) -> Result<(), parity_catalog_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            parity_catalog_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        self.kv
            .insert(format!("{}:{}", handle.identifier, key), value);
        Ok(())
    }

    fn exists(
        &mut self,
        bucket: Resource<parity_catalog_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
    ) -> Result<bool, parity_catalog_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            parity_catalog_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(self
            .kv
            .contains_key(&format!("{}:{}", handle.identifier, key)))
    }

    fn drop(
        &mut self,
        bucket: Resource<parity_catalog_bindings::wasi::keyvalue::store::Bucket>,
    ) -> wasmtime::Result<()> {
        let owned = Resource::<BucketHandle>::new_own(bucket.rep());
        Ok(self.table.delete(owned).map(|_| ())?)
    }
}

impl parity_catalog_bindings::patina::records::types::Host for CatalogHostState {}

struct PushCatalogHost {
    wasi: wasmtime_wasi::WasiCtx,
    table: ResourceTable,
    kv: HashMap<String, Vec<u8>>,
}

impl wasmtime_wasi::WasiView for PushCatalogHost {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl catalog_child_bindings::wasi::logging::logging::Host for PushCatalogHost {
    fn log(
        &mut self,
        _level: catalog_child_bindings::wasi::logging::logging::Level,
        _context: String,
        _message: String,
    ) {
    }
}

impl catalog_child_bindings::wasi::keyvalue::store::Host for PushCatalogHost {
    fn open(
        &mut self,
        identifier: String,
    ) -> Result<
        Resource<catalog_child_bindings::wasi::keyvalue::store::Bucket>,
        catalog_child_bindings::wasi::keyvalue::store::Error,
    > {
        let rep = self.table.push(BucketHandle { identifier }).map_err(|e| {
            catalog_child_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(Resource::new_own(rep.rep()))
    }
}

impl catalog_child_bindings::wasi::keyvalue::store::HostBucket for PushCatalogHost {
    fn get(
        &mut self,
        bucket: Resource<catalog_child_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
    ) -> Result<Option<Vec<u8>>, catalog_child_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            catalog_child_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(self
            .kv
            .get(&format!("{}:{}", handle.identifier, key))
            .cloned())
    }

    fn set(
        &mut self,
        bucket: Resource<catalog_child_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
        value: Vec<u8>,
    ) -> Result<(), catalog_child_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            catalog_child_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        self.kv
            .insert(format!("{}:{}", handle.identifier, key), value);
        Ok(())
    }

    fn exists(
        &mut self,
        bucket: Resource<catalog_child_bindings::wasi::keyvalue::store::Bucket>,
        key: String,
    ) -> Result<bool, catalog_child_bindings::wasi::keyvalue::store::Error> {
        let handle_ref = Resource::<BucketHandle>::new_borrow(bucket.rep());
        let handle = self.table.get(&handle_ref).map_err(|e| {
            catalog_child_bindings::wasi::keyvalue::store::Error::Other(e.to_string())
        })?;
        Ok(self
            .kv
            .contains_key(&format!("{}:{}", handle.identifier, key)))
    }

    fn drop(
        &mut self,
        bucket: Resource<catalog_child_bindings::wasi::keyvalue::store::Bucket>,
    ) -> wasmtime::Result<()> {
        let owned = Resource::<BucketHandle>::new_own(bucket.rep());
        Ok(self.table.delete(owned).map(|_| ())?)
    }
}

impl catalog_child_bindings::patina::records::types::Host for PushCatalogHost {}

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

#[test]
fn record_writer_push_equals_composed_with_mocked_upstream() {
    let Some(rw_wasm) = child_wasm_path("record_writer") else {
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
    write_child_bindings::RecordWriter::add_to_linker::<
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
    let push_instance = write_child_bindings::RecordWriter::instantiate(
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
