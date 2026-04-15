pub(crate) use std::collections::HashMap;
pub(crate) use std::path::{Path, PathBuf};

pub(crate) use sha2::Digest;
pub(crate) use tempfile::TempDir;
pub(crate) use wac_graph::{types::Package, CompositionGraph, EncodeOptions};
pub(crate) use wasmtime::component::{Component, Linker, Resource, ResourceTable};
pub(crate) use wasmtime::Store;

pub(crate) mod parity_source_bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/pando-parity/wit",
        world: "source-runner",
    });
}

pub(crate) mod parity_extract_bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/pando-parity/wit",
        world: "extract-runner",
    });
}

pub(crate) mod parity_schema_bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/pando-parity/wit",
        world: "schema-runner",
    });
}

pub(crate) mod parity_dedup_bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/pando-parity/wit",
        world: "dedup-runner",
    });
}

pub(crate) mod parity_write_bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/pando-parity/wit",
        world: "write-runner",
    });
}

pub(crate) mod parity_catalog_bindings {
    wasmtime::component::bindgen!({
        path: "tests/fixtures/pando-parity/wit",
        world: "catalog-runner",
    });
}

pub(crate) mod fsm_child_bindings {
    wasmtime::component::bindgen!({
        path: "children/file-system-monitor/wit",
        world: "file-system-monitor",
    });
}

pub(crate) mod extractor_child_bindings {
    wasmtime::component::bindgen!({
        path: "children/content-extractor/wit",
        world: "content-extractor",
    });
}

pub(crate) mod schema_child_bindings {
    wasmtime::component::bindgen!({
        path: "children/schema-enforcer/wit",
        world: "schema-enforcer",
    });
}

pub(crate) mod dedup_child_bindings {
    wasmtime::component::bindgen!({
        path: "children/dedup-filter/wit",
        world: "dedup-filter",
    });
}

pub(crate) mod write_child_bindings {
    wasmtime::component::bindgen!({
        path: "children/parquet-writer/wit",
        world: "parquet-writer",
    });
}

pub(crate) mod catalog_child_bindings {
    wasmtime::component::bindgen!({
        path: "children/lakehouse-catalog/wit",
        world: "lakehouse-catalog",
    });
}

#[derive(Clone)]
pub(crate) struct FixtureRecord {
    pub record_id: &'static str,
    pub source_path: &'static str,
    pub source_hash: &'static str,
    pub content: &'static str,
    pub content_hash: &'static str,
}

pub(crate) fn to_schema_parity_record(
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

pub(crate) fn to_dedup_parity_record(
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

pub(crate) fn to_schema_push_record(
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

pub(crate) fn to_dedup_push_record(
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

pub(crate) fn to_extract_parity_file(
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

pub(crate) fn to_extract_push_file(
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

pub(crate) fn to_write_parity_record(
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

pub(crate) fn to_write_push_record(
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

pub(crate) fn to_catalog_parity_file(
    file_path: &str,
    record_count: u64,
) -> parity_catalog_bindings::patina::records::types::FileWritten {
    parity_catalog_bindings::patina::records::types::FileWritten {
        file_path: file_path.to_string(),
        record_count,
        written_at: "2026-04-09T00:00:00Z".to_string(),
    }
}

pub(crate) fn to_catalog_push_file(
    file_path: &str,
    record_count: u64,
) -> catalog_child_bindings::patina::records::types::FileWritten {
    catalog_child_bindings::patina::records::types::FileWritten {
        file_path: file_path.to_string(),
        record_count,
        written_at: "2026-04-09T00:00:00Z".to_string(),
    }
}

pub(crate) fn discover_by_path_parity(
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

pub(crate) fn discover_by_path_push(
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

pub(crate) fn extract_by_path_parity(
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

pub(crate) fn extract_by_path_push(
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

pub(crate) fn guest_to_host_output_path(guest_file_path: &str, output_root: &Path) -> PathBuf {
    if let Some(relative) = guest_file_path.strip_prefix("/output/") {
        output_root.join(relative)
    } else {
        PathBuf::from(guest_file_path)
    }
}

pub(crate) fn parquet_row_count(path: &Path) -> i64 {
    let conn = duckdb::Connection::open_in_memory().expect("open duckdb");
    let escaped = path.to_string_lossy().replace('\'', "''");
    conn.query_row(
        &format!("SELECT COUNT(*) FROM read_parquet('{}')", escaped),
        [],
        |row| row.get(0),
    )
    .expect("query parquet row count")
}

pub(crate) fn child_wasm_path(crate_stem: &str) -> Option<PathBuf> {
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

pub(crate) fn adapter_wasm_path(crate_stem: &str) -> Option<PathBuf> {
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

pub(crate) fn typed_interface_candidates(interface_name: &str) -> Vec<String> {
    if interface_name.contains('@') {
        vec![interface_name.to_string()]
    } else {
        vec![
            interface_name.to_string(),
            format!("{}@0.1.0", interface_name),
        ]
    }
}

pub(crate) fn compose_component(
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

pub(crate) struct SchemaHostState {
    pub wasi: wasmtime_wasi::WasiCtx,
    pub table: ResourceTable,
    pub upstream_records: Vec<parity_schema_bindings::patina::records::types::RecordEnvelope>,
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

pub(crate) struct BucketHandle {
    pub identifier: String,
}

pub(crate) struct DedupHostState {
    pub wasi: wasmtime_wasi::WasiCtx,
    pub table: ResourceTable,
    pub upstream_records: Vec<parity_dedup_bindings::patina::records::types::RecordEnvelope>,
    pub kv: HashMap<String, Vec<u8>>,
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

pub(crate) struct PushSchemaHost {
    pub wasi: wasmtime_wasi::WasiCtx,
    pub table: ResourceTable,
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

pub(crate) struct PushDedupHost {
    pub wasi: wasmtime_wasi::WasiCtx,
    pub table: ResourceTable,
    pub kv: HashMap<String, Vec<u8>>,
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

pub(crate) struct SourceHostState {
    pub wasi: wasmtime_wasi::WasiCtx,
    pub table: ResourceTable,
    pub config: HashMap<String, String>,
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

pub(crate) struct PushFsmHost {
    pub wasi: wasmtime_wasi::WasiCtx,
    pub table: ResourceTable,
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

pub(crate) struct ExtractHostState {
    pub wasi: wasmtime_wasi::WasiCtx,
    pub table: ResourceTable,
    pub upstream_files: Vec<parity_extract_bindings::patina::records::types::FileFound>,
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

pub(crate) struct PushExtractHost {
    pub wasi: wasmtime_wasi::WasiCtx,
    pub table: ResourceTable,
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

pub(crate) struct WriteHostState {
    pub wasi: wasmtime_wasi::WasiCtx,
    pub table: ResourceTable,
    pub upstream: parity_write_bindings::patina::records::types::TransformResult,
    pub kv: HashMap<String, Vec<u8>>,
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

pub(crate) struct PushWriteHost {
    pub wasi: wasmtime_wasi::WasiCtx,
    pub table: ResourceTable,
    pub kv: HashMap<String, Vec<u8>>,
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

pub(crate) struct CatalogHostState {
    pub wasi: wasmtime_wasi::WasiCtx,
    pub table: ResourceTable,
    pub upstream_files: Vec<parity_catalog_bindings::patina::records::types::FileWritten>,
    pub kv: HashMap<String, Vec<u8>>,
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

pub(crate) struct PushCatalogHost {
    pub wasi: wasmtime_wasi::WasiCtx,
    pub table: ResourceTable,
    pub kv: HashMap<String, Vec<u8>>,
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
