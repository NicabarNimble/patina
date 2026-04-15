use super::common::*;

#[test]
fn pipeline_echo_name() {
    let (engine, component) = match load_echo_pipeline_component() {
        Some(ec) => ec,
        None => {
            panic!(
                "test fixture missing: tests/fixtures/echo_pipeline.wasm\n\
                 Build: cd tests/echo-pipeline && cargo build --release --target wasm32-wasip2\n\
                 Copy: cp tests/echo-pipeline/target/wasm32-wasip2/release/echo_pipeline.wasm tests/fixtures/"
            );
        }
    };

    let name = engine.get_name(&component).expect("get_name failed");
    assert_eq!(name, "echo-pipeline");
}

#[test]
fn pipeline_echo_handle_roundtrip() {
    let (engine, component) = match load_echo_pipeline_component() {
        Some(ec) => ec,
        None => return,
    };

    let manifest = echo_pipeline_manifest();
    let request = r#"{"op":"echo","version":"1","payload":{"key":"value","count":42}}"#;
    let response = engine
        .handle(&component, &manifest.name, request)
        .expect("handle failed");

    // Echo returns payload unchanged
    let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(parsed.get("key").and_then(|v| v.as_str()), Some("value"));
    assert_eq!(parsed.get("count").and_then(|v| v.as_i64()), Some(42));
}

#[test]
fn pipeline_echo_unknown_op_error() {
    let (engine, component) = match load_echo_pipeline_component() {
        Some(ec) => ec,
        None => return,
    };

    let manifest = echo_pipeline_manifest();
    let request = r#"{"op":"frobnicate","version":"1","payload":{}}"#;
    let result = engine.handle(&component, &manifest.name, request);

    assert!(
        result.is_err(),
        "unknown op should return error, got: {:?}",
        result
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("unknown op"),
        "error should mention 'unknown op', got: {}",
        err
    );
}

#[test]
fn pipeline_echo_version_mismatch_error() {
    let (engine, component) = match load_echo_pipeline_component() {
        Some(ec) => ec,
        None => return,
    };

    let manifest = echo_pipeline_manifest();
    let request = r#"{"op":"echo","version":"999","payload":{}}"#;
    let result = engine.handle(&component, &manifest.name, request);

    assert!(
        result.is_err(),
        "version mismatch should return error, got: {:?}",
        result
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("version"),
        "error should mention 'version', got: {}",
        err
    );
}
