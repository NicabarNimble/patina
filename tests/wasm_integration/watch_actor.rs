use super::common::*;

#[test]
fn folder_watch_actor_typed_call_contracts_end_to_end() {
    let Some(wasm_path) = folder_watch_actor_component_path() else {
        return;
    };

    with_temp_patina_home(|_| {
        let engine = ChildEngine::new().unwrap();
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        let component = engine.load_component(&wasm_bytes).unwrap();
        let manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("children/folder-watch-actor/child.toml");
        let manifest = ChildManifest::from_path(&manifest_path).unwrap();

        let child = engine
            .instantiate_child(&component, &manifest, None)
            .expect("folder-watch-actor should instantiate");

        let status_response = child
            .call(&ChildCallRequest {
                operation_id: "patina:watch/control.status".to_string(),
                args: serde_json::json!([]),
                correlation: None,
            })
            .expect("typed status call should succeed");
        assert!(
            status_response.payload.get("ticks").is_some(),
            "status payload should expose watcher-stats fields: {}",
            status_response.payload
        );

        let configure_response = child
            .call(&ChildCallRequest {
                operation_id: "patina:watch/control.configure".to_string(),
                args: serde_json::json!([
                    {
                        "watch-path": "/tmp",
                        "stream-name": "watch.folder",
                        "recursive": true,
                        "include-hidden": false,
                        "emit-existing-on-start": false,
                        "extensions": ["txt"]
                    },
                    true
                ]),
                correlation: None,
            })
            .expect("typed configure call should succeed");
        assert_eq!(
            configure_response
                .payload
                .get("ok")
                .and_then(|v| v.get("watch-path"))
                .and_then(|v| v.as_str()),
            Some("/tmp")
        );

        let scan_response = child
            .call(&ChildCallRequest {
                operation_id: "patina:watch/control.scan-now".to_string(),
                args: serde_json::json!([]),
                correlation: None,
            })
            .expect("typed scan-now call should succeed");
        assert!(
            scan_response
                .payload
                .get("ok")
                .and_then(|v| v.get("files-seen"))
                .is_some(),
            "scan-now payload should expose scan-outcome result: {}",
            scan_response.payload
        );

        let reset_response = child
            .call(&ChildCallRequest {
                operation_id: "patina:watch/control.reset".to_string(),
                args: serde_json::json!([]),
                correlation: None,
            })
            .expect("typed reset call should succeed");
        assert_eq!(
            reset_response.payload.get("ok"),
            Some(&serde_json::Value::Null)
        );

        let handle_response = child.handle(&ChildRequest {
            action: "status".to_string(),
            payload: serde_json::json!({}),
        });
        assert!(
            handle_response.is_err(),
            "handle business ingress should be denied"
        );
    });
}
