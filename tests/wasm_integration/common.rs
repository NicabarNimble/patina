//! WASM integration tests — separated from unit tests for CI lane targeting.
//!
//! These tests load wasmtime-backed WASM components and are slower than unit
//! tests. Tier 2 pre-push runs `--lib` only; CI can target these independently
//! via `--test wasm_integration`.

pub(crate) use mother_crate::registry::ChildRegistry;
pub(crate) use patina::child::testing::{
    events_subscribe, ChildEngine, ChildIngressMode, ChildKind, ChildManifest, ChildProvides,
    FilesystemAccessMode, FilesystemPreopen, PipelineEngine,
};
pub(crate) use patina::mother::{Child, ChildHealth, ChildRequest, GrantedToys};

// =====================================================================
// Helpers
// =====================================================================

pub(crate) fn with_temp_patina_home<T>(f: impl FnOnce(&std::path::Path) -> T) -> T {
    let _guard = patina::test_support::env_test_mutex()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let temp = tempfile::TempDir::new().unwrap();
    let home = temp.path().join("patina-home");
    std::fs::create_dir_all(&home).unwrap();
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

pub(crate) fn session_writer_component_path() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "target/wasm32-wasip1/debug/patina_ai_child_session_writer.wasm",
        "target/wasm32-wasip1/release/patina_ai_child_session_writer.wasm",
    ] {
        let path = root.join(rel);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

pub(crate) fn folder_text_to_parquet_component_path() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "target/wasm32-wasip2/debug/patina_ai_child_folder_text_to_parquet.wasm",
        "target/wasm32-wasip2/release/patina_ai_child_folder_text_to_parquet.wasm",
        "target/wasm32-wasip1/debug/patina_ai_child_folder_text_to_parquet.wasm",
        "target/wasm32-wasip1/release/patina_ai_child_folder_text_to_parquet.wasm",
    ] {
        let path = root.join(rel);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

pub(crate) fn file_system_monitor_component_path() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "target/wasm32-wasip2/debug/patina_ai_child_file_system_monitor.wasm",
        "target/wasm32-wasip2/release/patina_ai_child_file_system_monitor.wasm",
        "target/wasm32-wasip1/debug/patina_ai_child_file_system_monitor.wasm",
        "target/wasm32-wasip1/release/patina_ai_child_file_system_monitor.wasm",
    ] {
        let path = root.join(rel);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

pub(crate) fn content_extractor_component_path() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "target/wasm32-wasip2/debug/patina_ai_child_content_extractor.wasm",
        "target/wasm32-wasip2/release/patina_ai_child_content_extractor.wasm",
        "target/wasm32-wasip1/debug/patina_ai_child_content_extractor.wasm",
        "target/wasm32-wasip1/release/patina_ai_child_content_extractor.wasm",
    ] {
        let path = root.join(rel);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

pub(crate) fn schema_enforcer_component_path() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "target/wasm32-wasip2/debug/patina_ai_child_schema_enforcer.wasm",
        "target/wasm32-wasip2/release/patina_ai_child_schema_enforcer.wasm",
        "target/wasm32-wasip1/debug/patina_ai_child_schema_enforcer.wasm",
        "target/wasm32-wasip1/release/patina_ai_child_schema_enforcer.wasm",
    ] {
        let path = root.join(rel);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

pub(crate) fn dedup_filter_component_path() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "target/wasm32-wasip2/debug/patina_ai_child_dedup_filter.wasm",
        "target/wasm32-wasip2/release/patina_ai_child_dedup_filter.wasm",
        "target/wasm32-wasip1/debug/patina_ai_child_dedup_filter.wasm",
        "target/wasm32-wasip1/release/patina_ai_child_dedup_filter.wasm",
    ] {
        let path = root.join(rel);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

pub(crate) fn parquet_writer_component_path() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "target/wasm32-wasip2/debug/patina_ai_child_parquet_writer.wasm",
        "target/wasm32-wasip2/release/patina_ai_child_parquet_writer.wasm",
        "target/wasm32-wasip1/debug/patina_ai_child_parquet_writer.wasm",
        "target/wasm32-wasip1/release/patina_ai_child_parquet_writer.wasm",
    ] {
        let path = root.join(rel);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

pub(crate) fn lakehouse_catalog_component_path() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "target/wasm32-wasip2/debug/patina_ai_child_lakehouse_catalog.wasm",
        "target/wasm32-wasip2/release/patina_ai_child_lakehouse_catalog.wasm",
        "target/wasm32-wasip1/debug/patina_ai_child_lakehouse_catalog.wasm",
        "target/wasm32-wasip1/release/patina_ai_child_lakehouse_catalog.wasm",
    ] {
        let path = root.join(rel);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Helper: load repos.wasm fixture and instantiate child.
pub(crate) fn load_repos_child() -> Option<Box<dyn Child>> {
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/patina_plugin_repos.wasm");
    if !wasm_path.exists() {
        return None;
    }

    let engine = ChildEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let manifest = ChildManifest {
        name: "patina-repos".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: ChildKind::Child,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec!["git".into(), "patina".into()],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: Some("repos".into()),
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
        declared_metrics: std::collections::HashMap::new(),
        filesystem_preopens: vec![],
        state_enabled: false,
        checkpoint_streams: vec![],
        lake_names: vec![],
        ingress_sources: std::collections::HashMap::new(),
        subscribed_streams: vec![],
        task_intent_names: vec![],
        task_intents: vec![],
        graph_read: false,
        graph_write_actions: vec![],
        belief_read: false,
        belief_write_actions: vec![],
        toys: GrantedToys::default(),
        inside_accepts: vec![],
        ingress_mode: ChildIngressMode::Handle,
        contract_default_operation: None,
        contract_allow_operations: vec![],
    };

    engine.instantiate_child(&component, &manifest, None).ok()
}

pub(crate) fn load_echo_pipeline_component(
) -> Option<(PipelineEngine, wasmtime::component::Component)> {
    let wasm_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/echo_pipeline.wasm");
    if !wasm_path.exists() {
        return None;
    }
    let engine = PipelineEngine::new().expect("PipelineEngine::new() failed");
    let wasm_bytes = std::fs::read(&wasm_path).expect("failed to read echo-pipeline wasm");
    let component = engine
        .load_component(&wasm_bytes)
        .expect("load_component failed");
    Some((engine, component))
}

pub(crate) fn echo_pipeline_manifest() -> ChildManifest {
    ChildManifest {
        name: "echo-pipeline".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: ChildKind::Pipeline,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            pipeline_ops: vec!["echo".into()],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
        declared_metrics: std::collections::HashMap::new(),
        filesystem_preopens: vec![],
        state_enabled: false,
        checkpoint_streams: vec![],
        lake_names: vec![],
        ingress_sources: std::collections::HashMap::new(),
        subscribed_streams: vec![],
        task_intent_names: vec![],
        task_intents: vec![],
        graph_read: false,
        graph_write_actions: vec![],
        belief_read: false,
        belief_write_actions: vec![],
        toys: GrantedToys::default(),
        inside_accepts: vec![],
        ingress_mode: ChildIngressMode::Handle,
        contract_default_operation: None,
        contract_allow_operations: vec![],
    }
}

/// Load the panic-pipeline WASM fixture.
pub(crate) fn load_panic_pipeline_component(
) -> Option<(PipelineEngine, wasmtime::component::Component)> {
    let wasm_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/panic_pipeline.wasm");
    if !wasm_path.exists() {
        return None;
    }
    let engine = PipelineEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    Some((engine, component))
}

// =====================================================================
// WASM integration — session-writer and canon children
// =====================================================================
