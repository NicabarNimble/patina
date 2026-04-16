use super::common::*;

#[test]
fn folder_text_to_parquet_scan_contract_end_to_end() {
    let Some(wasm_path) = folder_text_to_parquet_component_path() else {
        return;
    };

    with_temp_patina_home(|_| {
        let engine = ChildEngine::new().unwrap();
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        let component = engine.load_component(&wasm_bytes).unwrap();
        let manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("children/folder-text-to-parquet/child.toml");
        let manifest = ChildManifest::from_path(&manifest_path).unwrap();
        let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/folder-text-to-parquet");
        let output_dir = tempfile::TempDir::new().unwrap();
        let host_parquet_path = output_dir.path().join("scan-batch.parquet");
        let child = engine
            .instantiate_child_with_preopens(
                &component,
                &manifest,
                None,
                &[
                    FilesystemPreopen {
                        host_path: fixture_dir.clone(),
                        guest_path: "/input".to_string(),
                        mode: FilesystemAccessMode::ReadOnly,
                    },
                    FilesystemPreopen {
                        host_path: output_dir.path().to_path_buf(),
                        guest_path: "/output".to_string(),
                        mode: FilesystemAccessMode::ReadWrite,
                    },
                ],
            )
            .unwrap();

        let response = child
            .handle(&ChildRequest {
                action: "scan".into(),
                payload: serde_json::json!({
                    "folder_path": "/input",
                    "output_path": "/output/scan-batch.parquet",
                }),
            })
            .unwrap();

        let payload = response.payload;
        assert_eq!(
            payload.get("status").and_then(|v| v.as_str()),
            Some("ok"),
            "scan should complete successfully"
        );
        assert_eq!(
            payload
                .get("processed_records")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            4,
            "expected four discovered files (.txt/.md except empty/hidden/wrong ext)"
        );
        assert_eq!(
            payload.get("parquet_path").and_then(|v| v.as_str()),
            Some("/output/scan-batch.parquet"),
            "scan should report guest parquet output path"
        );
        assert!(
            host_parquet_path.exists(),
            "expected parquet file at {}",
            host_parquet_path.display()
        );

        let records = payload
            .get("records")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        assert_eq!(records.len(), 4, "scan result should return four records");

        let mut by_name = std::collections::HashMap::new();
        for record in &records {
            let source_path = record
                .get("source_path")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let name = std::path::Path::new(source_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            by_name.insert(name, record.clone());
        }

        for required in [
            "hello.txt",
            "duplicate-of-hello.txt",
            "notes.md",
            "readme.txt",
        ] {
            assert!(
                by_name.contains_key(required),
                "missing expected record for {}",
                required
            );
        }

        let fixture_files = [
            ("hello.txt", "text/plain", "utf-8", 3_u64, 1_u32),
            (
                "duplicate-of-hello.txt",
                "text/plain",
                "utf-8",
                3_u64,
                1_u32,
            ),
            ("notes.md", "text/markdown", "utf-8", 10_u64, 1_u32),
            ("readme.txt", "text/plain", "utf-8", 22_u64, 1_u32),
        ];

        for (name, content_type, encoding, line_count, schema_version) in fixture_files {
            let file_path = fixture_dir.join(name);
            let bytes = std::fs::read(&file_path).unwrap();
            let content = String::from_utf8(bytes.clone()).unwrap();
            let source_hash = {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                format!("{:x}", hasher.finalize())
            };
            let content_hash = {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(content.as_bytes());
                format!("{:x}", hasher.finalize())
            };

            let record = by_name.get(name).unwrap();
            assert_eq!(
                record.get("source_hash").and_then(|v| v.as_str()),
                Some(source_hash.as_str()),
                "source_hash should match for {}",
                name
            );
            assert_eq!(
                record.get("content_hash").and_then(|v| v.as_str()),
                Some(content_hash.as_str()),
                "content_hash should match for {}",
                name
            );
            assert_eq!(
                record.get("line_count").and_then(|v| v.as_u64()),
                Some(line_count),
                "line_count should match for {}",
                name
            );
            assert_eq!(
                record.get("source_size_bytes").and_then(|v| v.as_u64()),
                Some(bytes.len() as u64),
                "source_size_bytes should match for {}",
                name
            );
            assert_eq!(
                record.get("content_type").and_then(|v| v.as_str()),
                Some(content_type),
                "content_type should match for {}",
                name
            );
            assert_eq!(
                record.get("encoding").and_then(|v| v.as_str()),
                Some(encoding),
                "encoding should match for {}",
                name
            );
            assert_eq!(
                record.get("schema_version").and_then(|v| v.as_u64()),
                Some(schema_version as u64),
                "schema_version should match for {}",
                name
            );
            assert_eq!(
                record.get("content").and_then(|v| v.as_str()),
                Some(content.as_str()),
                "content should match for {}",
                name
            );

            let source_modified_at = record
                .get("source_modified_at")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            assert!(
                chrono::DateTime::parse_from_rfc3339(source_modified_at).is_ok(),
                "source_modified_at should be RFC3339 for {}",
                name
            );

            let ingested_at = record
                .get("ingested_at")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            assert!(
                chrono::DateTime::parse_from_rfc3339(ingested_at).is_ok(),
                "ingested_at should be RFC3339 for {}",
                name
            );

            let record_id = record
                .get("record_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            assert!(
                uuid::Uuid::parse_str(record_id).is_ok(),
                "record_id should be UUID for {}",
                name
            );

            let batch_id = record
                .get("batch_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            assert!(
                !batch_id.is_empty(),
                "batch_id should be non-empty for {}",
                name
            );
        }

        let state_keys = payload
            .get("state_keys")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect::<Vec<_>>();
        assert_eq!(
            state_keys.len(),
            3,
            "duplicate file should share source_hash key and produce three unique record:* keys"
        );
        for key in &state_keys {
            assert!(
                key.starts_with("record:"),
                "state keys must use record:{{source_hash}} format"
            );
        }

        let events = events_subscribe("file.found", None, 64).unwrap();
        assert!(
            events.len() >= 4,
            "expected at least four file.found events for discovered files"
        );
        let mut event_names = std::collections::HashSet::new();
        for event in &events {
            let payload: serde_json::Value = serde_json::from_str(&event.payload).unwrap();
            let source_path = payload
                .get("source_path")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let name = std::path::Path::new(source_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if matches!(
                name,
                "hello.txt" | "duplicate-of-hello.txt" | "notes.md" | "readme.txt"
            ) {
                event_names.insert(name.to_string());
            }
        }
        assert_eq!(
            event_names.len(),
            4,
            "expected events for each discovered fixture file"
        );

        let written_events = events_subscribe("file.written", None, 64).unwrap();
        assert!(
            !written_events.is_empty(),
            "expected at least one file.written event"
        );
        let last_written = written_events.last().unwrap();
        let written_payload: serde_json::Value =
            serde_json::from_str(&last_written.payload).unwrap();
        assert_eq!(
            written_payload.get("file_path").and_then(|v| v.as_str()),
            Some("/output/scan-batch.parquet")
        );
        assert_eq!(
            written_payload.get("record_count").and_then(|v| v.as_u64()),
            Some(4)
        );

        let db = duckdb::Connection::open_in_memory()
            .expect("open in-memory duckdb for parquet verification");
        let row_count: u64 = db
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM read_parquet('{}')",
                    host_parquet_path.to_string_lossy()
                ),
                [],
                |row| row.get(0),
            )
            .expect("query parquet row count");
        assert_eq!(
            row_count, 4,
            "parquet row count should match discovered files"
        );

        let conn = patina::eventlog::open_events_db().unwrap();
        let mut stmt = conn
            .prepare("SELECT data FROM eventlog WHERE event_type = 'measure.metric' ORDER BY seq")
            .unwrap();
        let metrics = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(
            !metrics.is_empty(),
            "expected measure.metric events to be emitted"
        );

        let mut names = std::collections::HashSet::new();
        for metric_json in metrics {
            let value: serde_json::Value = serde_json::from_str(&metric_json).unwrap();
            if value.get("source").and_then(|v| v.as_str()) != Some("folder-text-to-parquet") {
                continue;
            }
            if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
                names.insert(name.to_string());
            }
        }

        for required in ["files_discovered", "records_written", "write_latency_ms"] {
            assert!(
                names.contains(required),
                "missing expected metric {}",
                required
            );
        }

        let skipped_paths = payload
            .get("skipped_files")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut skipped_names = std::collections::HashSet::new();
        for skipped in skipped_paths {
            let path = skipped
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if let Some(name) = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
            {
                skipped_names.insert(name.to_string());
            }
        }
        for expected in ["empty.txt", ".hidden", "image.png"] {
            assert!(
                skipped_names.contains(expected),
                "expected skipped file {}",
                expected
            );
        }
    });
}

#[test]
fn folder_text_to_parquet_first_split_composes_via_events() {
    let Some(monitor_wasm_path) = file_system_monitor_component_path() else {
        return;
    };
    let Some(processor_wasm_path) = folder_text_to_parquet_component_path() else {
        return;
    };

    with_temp_patina_home(|_| {
        let engine = ChildEngine::new().unwrap();

        let monitor_wasm_bytes = std::fs::read(&monitor_wasm_path).unwrap();
        let monitor_component = engine.load_component(&monitor_wasm_bytes).unwrap();
        let monitor_manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("children/file-system-monitor/child.toml");
        let monitor_manifest = ChildManifest::from_path(&monitor_manifest_path).unwrap();

        let processor_wasm_bytes = std::fs::read(&processor_wasm_path).unwrap();
        let processor_component = engine.load_component(&processor_wasm_bytes).unwrap();
        let processor_manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("children/folder-text-to-parquet/child.toml");
        let processor_manifest = ChildManifest::from_path(&processor_manifest_path).unwrap();

        let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/folder-text-to-parquet");
        let output_dir = tempfile::TempDir::new().unwrap();
        let host_parquet_path = output_dir.path().join("split-batch.parquet");

        let monitor_child = match engine.instantiate_child_with_preopens(
            &monitor_component,
            &monitor_manifest,
            None,
            &[FilesystemPreopen {
                host_path: fixture_dir.clone(),
                guest_path: "/input".to_string(),
                mode: FilesystemAccessMode::ReadOnly,
            }],
        ) {
            Ok(child) => child,
            Err(error) if error.to_string().contains("no export `drain` found") => {
                return;
            }
            Err(error) => panic!("monitor instantiate failed: {}", error),
        };

        let processor_child = engine
            .instantiate_child_with_preopens(
                &processor_component,
                &processor_manifest,
                None,
                &[
                    FilesystemPreopen {
                        host_path: fixture_dir,
                        guest_path: "/input".to_string(),
                        mode: FilesystemAccessMode::ReadOnly,
                    },
                    FilesystemPreopen {
                        host_path: output_dir.path().to_path_buf(),
                        guest_path: "/output".to_string(),
                        mode: FilesystemAccessMode::ReadWrite,
                    },
                ],
            )
            .unwrap();

        let monitor_response = monitor_child
            .handle(&ChildRequest {
                action: "scan".into(),
                payload: serde_json::json!({"folder_path": "/input"}),
            })
            .unwrap();
        assert_eq!(
            monitor_response
                .payload
                .get("processed_files")
                .and_then(|v| v.as_u64()),
            Some(4),
            "monitor should emit four file.found events"
        );

        let processor_response = match processor_child.handle(&ChildRequest {
            action: "process-discovered".into(),
            payload: serde_json::json!({
                "output_path": "/output/split-batch.parquet",
                "limit": 64,
            }),
        }) {
            Ok(response) => response,
            Err(error)
                if error
                    .to_string()
                    .contains("unknown action 'process-discovered'") =>
            {
                return;
            }
            Err(error) => panic!("processor handle failed: {}", error),
        };
        let payload = processor_response.payload;

        assert_eq!(
            payload.get("status").and_then(|v| v.as_str()),
            Some("ok"),
            "processor should return ok"
        );
        assert_eq!(
            payload
                .get("processed_records")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            4,
            "first split should preserve four discovered fixture records"
        );
        assert!(
            payload
                .get("acked_through")
                .and_then(|v| v.as_u64())
                .is_some(),
            "processor should ack consumed file.found events"
        );

        let state_keys = payload
            .get("state_keys")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect::<Vec<_>>();
        assert_eq!(
            state_keys.len(),
            3,
            "duplicate file should still dedupe to three record:* keys"
        );

        assert!(
            host_parquet_path.exists(),
            "expected split parquet file at {}",
            host_parquet_path.display()
        );
    });
}

#[test]
fn folder_text_to_parquet_six_child_pipeline_composes_via_events() {
    let Some(monitor_wasm_path) = file_system_monitor_component_path() else {
        return;
    };
    let Some(extractor_wasm_path) = content_extractor_component_path() else {
        return;
    };
    let Some(enforcer_wasm_path) = schema_enforcer_component_path() else {
        return;
    };
    let Some(dedup_wasm_path) = dedup_filter_component_path() else {
        return;
    };
    let Some(writer_wasm_path) = parquet_writer_component_path() else {
        return;
    };
    let Some(catalog_wasm_path) = lakehouse_catalog_component_path() else {
        return;
    };

    with_temp_patina_home(|_| {
        let engine = ChildEngine::new().unwrap();

        let monitor_wasm_bytes = std::fs::read(&monitor_wasm_path).unwrap();
        let monitor_component = engine.load_component(&monitor_wasm_bytes).unwrap();
        let monitor_manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("children/file-system-monitor/child.toml");
        let monitor_manifest = ChildManifest::from_path(&monitor_manifest_path).unwrap();

        let extractor_wasm_bytes = std::fs::read(&extractor_wasm_path).unwrap();
        let extractor_component = engine.load_component(&extractor_wasm_bytes).unwrap();
        let extractor_manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("children/content-extractor/child.toml");
        let extractor_manifest = ChildManifest::from_path(&extractor_manifest_path).unwrap();

        let enforcer_wasm_bytes = std::fs::read(&enforcer_wasm_path).unwrap();
        let enforcer_component = engine.load_component(&enforcer_wasm_bytes).unwrap();
        let enforcer_manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("children/schema-enforcer/child.toml");
        let enforcer_manifest = ChildManifest::from_path(&enforcer_manifest_path).unwrap();

        let dedup_wasm_bytes = std::fs::read(&dedup_wasm_path).unwrap();
        let dedup_component = engine.load_component(&dedup_wasm_bytes).unwrap();
        let dedup_manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("children/dedup-filter/child.toml");
        let dedup_manifest = ChildManifest::from_path(&dedup_manifest_path).unwrap();

        let writer_wasm_bytes = std::fs::read(&writer_wasm_path).unwrap();
        let writer_component = engine.load_component(&writer_wasm_bytes).unwrap();
        let writer_manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("children/parquet-writer/child.toml");
        let writer_manifest = ChildManifest::from_path(&writer_manifest_path).unwrap();

        let catalog_wasm_bytes = std::fs::read(&catalog_wasm_path).unwrap();
        let catalog_component = engine.load_component(&catalog_wasm_bytes).unwrap();
        let catalog_manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("children/lakehouse-catalog/child.toml");
        let catalog_manifest = ChildManifest::from_path(&catalog_manifest_path).unwrap();

        let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/folder-text-to-parquet");
        let output_dir = tempfile::TempDir::new().unwrap();
        let host_parquet_path = output_dir.path().join("six-child-batch.parquet");

        let monitor_child = match engine.instantiate_child_with_preopens(
            &monitor_component,
            &monitor_manifest,
            None,
            &[FilesystemPreopen {
                host_path: fixture_dir.clone(),
                guest_path: "/input".to_string(),
                mode: FilesystemAccessMode::ReadOnly,
            }],
        ) {
            Ok(child) => child,
            Err(error) if error.to_string().contains("no export `drain` found") => {
                return;
            }
            Err(error) => panic!("monitor instantiate failed: {}", error),
        };

        let extractor_child = match engine.instantiate_child_with_preopens(
            &extractor_component,
            &extractor_manifest,
            None,
            &[FilesystemPreopen {
                host_path: fixture_dir,
                guest_path: "/input".to_string(),
                mode: FilesystemAccessMode::ReadOnly,
            }],
        ) {
            Ok(child) => child,
            Err(error) if error.to_string().contains("no export `drain` found") => {
                return;
            }
            Err(error) => panic!("extractor instantiate failed: {}", error),
        };

        let enforcer_child =
            match engine.instantiate_child(&enforcer_component, &enforcer_manifest, None) {
                Ok(child) => child,
                Err(error) if error.to_string().contains("no export `drain` found") => {
                    return;
                }
                Err(error) => panic!("enforcer instantiate failed: {}", error),
            };

        let dedup_child = match engine.instantiate_child(&dedup_component, &dedup_manifest, None) {
            Ok(child) => child,
            Err(error) if error.to_string().contains("no export `drain` found") => {
                return;
            }
            Err(error) => panic!("dedup instantiate failed: {}", error),
        };

        let writer_child = match engine.instantiate_child_with_preopens(
            &writer_component,
            &writer_manifest,
            None,
            &[FilesystemPreopen {
                host_path: output_dir.path().to_path_buf(),
                guest_path: "/lake".to_string(),
                mode: FilesystemAccessMode::ReadWrite,
            }],
        ) {
            Ok(child) => child,
            Err(error) if error.to_string().contains("no export `drain` found") => {
                return;
            }
            Err(error) => panic!("writer instantiate failed: {}", error),
        };

        let catalog_child =
            match engine.instantiate_child(&catalog_component, &catalog_manifest, None) {
                Ok(child) => child,
                Err(error) if error.to_string().contains("no export `drain` found") => {
                    return;
                }
                Err(error) => panic!("catalog instantiate failed: {}", error),
            };

        let registry = ChildRegistry::new();
        registry.register_knowledge(monitor_child).unwrap();
        registry.register_knowledge(extractor_child).unwrap();
        registry.register_knowledge(enforcer_child).unwrap();
        registry.register_knowledge(dedup_child).unwrap();
        registry.register_knowledge(writer_child).unwrap();
        registry.register_knowledge(catalog_child).unwrap();

        let last_offset = |stream: &str| {
            events_subscribe(stream, None, 1_000_000)
                .unwrap_or_default()
                .last()
                .map(|event| event.offset)
        };

        let file_found_before = last_offset("file.found");
        let record_extracted_before = last_offset("record.extracted");
        let record_validated_before = last_offset("record.validated");
        let record_ready_before = last_offset("record.ready");
        let file_written_before = last_offset("file.written");

        let monitor_response = registry
            .handle(
                "file-system-monitor",
                &ChildRequest {
                    action: "scan".into(),
                    payload: serde_json::json!({"folder_path": "/input"}),
                },
            )
            .unwrap();
        assert_eq!(
            monitor_response
                .payload
                .get("processed_files")
                .and_then(|v| v.as_u64()),
            Some(4)
        );

        let extractor_response = registry
            .handle(
                "content-extractor",
                &ChildRequest {
                    action: "extract-found".into(),
                    payload: serde_json::json!({
                        "limit": 64,
                        "after_offset": file_found_before,
                    }),
                },
            )
            .unwrap();
        assert_eq!(
            extractor_response
                .payload
                .get("processed_records")
                .and_then(|v| v.as_u64()),
            Some(4)
        );
        assert!(
            extractor_response
                .payload
                .get("acked_through")
                .and_then(|v| v.as_u64())
                .is_some(),
            "content-extractor should ack consumed file.found events"
        );

        let enforcer_response = registry
            .handle(
                "schema-enforcer",
                &ChildRequest {
                    action: "enforce-schema".into(),
                    payload: serde_json::json!({
                        "limit": 64,
                        "after_offset": record_extracted_before,
                    }),
                },
            )
            .unwrap();
        assert_eq!(
            enforcer_response
                .payload
                .get("validated_records")
                .and_then(|v| v.as_u64()),
            Some(4)
        );
        assert_eq!(
            enforcer_response
                .payload
                .get("rejected_records")
                .and_then(|v| v.as_u64()),
            Some(0)
        );

        let dedup_response = registry
            .handle(
                "dedup-filter",
                &ChildRequest {
                    action: "filter-dedup".into(),
                    payload: serde_json::json!({
                        "limit": 64,
                        "after_offset": record_validated_before,
                    }),
                },
            )
            .unwrap();
        assert_eq!(
            dedup_response
                .payload
                .get("ready_records")
                .and_then(|v| v.as_u64()),
            Some(3)
        );
        assert_eq!(
            dedup_response
                .payload
                .get("duplicate_records")
                .and_then(|v| v.as_u64()),
            Some(1)
        );

        let writer_response = registry
            .handle(
                "parquet-writer",
                &ChildRequest {
                    action: "write-records".into(),
                    payload: serde_json::json!({
                        "output_root": "/lake",
                        "output_path": "six-child-batch.parquet",
                        "limit": 64,
                        "after_offset": record_ready_before,
                    }),
                },
            )
            .unwrap();
        assert_eq!(
            writer_response
                .payload
                .get("processed_records")
                .and_then(|v| v.as_u64()),
            Some(3)
        );
        let state_keys = writer_response
            .payload
            .get("state_keys")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect::<Vec<_>>();
        assert_eq!(state_keys.len(), 3);
        assert!(host_parquet_path.exists());

        let catalog_response = registry
            .handle(
                "lakehouse-catalog",
                &ChildRequest {
                    action: "register-written".into(),
                    payload: serde_json::json!({
                        "limit": 64,
                        "after_offset": file_written_before,
                    }),
                },
            )
            .unwrap();
        assert_eq!(
            catalog_response
                .payload
                .get("registered_files")
                .and_then(|v| v.as_u64()),
            Some(1)
        );

        let evolved_catalog_response = registry
            .handle(
                "lakehouse-catalog",
                &ChildRequest {
                    action: "register-written".into(),
                    payload: serde_json::json!({
                        "limit": 64,
                        "after_offset": file_written_before,
                        "add_nullable_columns": ["source_owner"],
                    }),
                },
            )
            .unwrap();
        assert_eq!(
            evolved_catalog_response
                .payload
                .get("schema_version")
                .and_then(|v| v.as_u64()),
            Some(2),
            "schema evolution should bump schema version"
        );
        let evolved_columns = evolved_catalog_response
            .payload
            .get("schema")
            .and_then(|v| v.get("columns"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut has_source_owner = false;
        for column in evolved_columns {
            if column.get("name").and_then(|v| v.as_str()) == Some("source_owner") {
                has_source_owner = column.get("nullable").and_then(|v| v.as_bool()) == Some(true);
                break;
            }
        }
        assert!(
            has_source_owner,
            "schema evolution should add nullable source_owner column"
        );
        assert_eq!(
            evolved_catalog_response
                .payload
                .get("catalog_keys")
                .and_then(|v| v.as_array())
                .map(|keys| keys.len())
                .unwrap_or(0),
            1,
            "schema evolution should preserve existing catalog file entries"
        );

        let db = duckdb::Connection::open_in_memory()
            .expect("open in-memory duckdb for parquet verification");
        let row_count: u64 = db
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM read_parquet('{}')",
                    host_parquet_path.to_string_lossy()
                ),
                [],
                |row| row.get(0),
            )
            .expect("query parquet row count");
        assert_eq!(
            row_count, 3,
            "dedup-filter should reduce output rows to three"
        );

        let file_found_after = last_offset("file.found");
        let record_extracted_after = last_offset("record.extracted");
        let record_validated_after = last_offset("record.validated");
        let record_ready_after = last_offset("record.ready");
        let file_written_after = last_offset("file.written");

        let extractor_rerun = registry
            .handle(
                "content-extractor",
                &ChildRequest {
                    action: "extract-found".into(),
                    payload: serde_json::json!({
                        "limit": 64,
                        "after_offset": file_found_after,
                    }),
                },
            )
            .unwrap();
        assert_eq!(
            extractor_rerun
                .payload
                .get("processed_records")
                .and_then(|v| v.as_u64()),
            Some(0),
            "checkpoint restart should not reprocess file.found offsets"
        );

        let enforcer_rerun = registry
            .handle(
                "schema-enforcer",
                &ChildRequest {
                    action: "enforce-schema".into(),
                    payload: serde_json::json!({
                        "limit": 64,
                        "after_offset": record_extracted_after,
                    }),
                },
            )
            .unwrap();
        assert_eq!(
            enforcer_rerun
                .payload
                .get("validated_records")
                .and_then(|v| v.as_u64()),
            Some(0),
            "checkpoint restart should not reprocess record.extracted offsets"
        );

        let dedup_rerun = registry
            .handle(
                "dedup-filter",
                &ChildRequest {
                    action: "filter-dedup".into(),
                    payload: serde_json::json!({
                        "limit": 64,
                        "after_offset": record_validated_after,
                    }),
                },
            )
            .unwrap();
        assert_eq!(
            dedup_rerun
                .payload
                .get("ready_records")
                .and_then(|v| v.as_u64()),
            Some(0),
            "checkpoint restart should not reprocess record.validated offsets"
        );

        let catalog_rerun = registry
            .handle(
                "lakehouse-catalog",
                &ChildRequest {
                    action: "register-written".into(),
                    payload: serde_json::json!({
                        "limit": 64,
                        "after_offset": file_written_after,
                    }),
                },
            )
            .unwrap();
        assert_eq!(
            catalog_rerun
                .payload
                .get("registered_files")
                .and_then(|v| v.as_u64()),
            Some(0),
            "checkpoint restart should not reprocess file.written offsets"
        );

        let writer_rerun = registry
            .handle(
                "parquet-writer",
                &ChildRequest {
                    action: "write-records".into(),
                    payload: serde_json::json!({
                        "output_root": "/lake",
                        "output_path": "six-child-rerun.parquet",
                        "limit": 64,
                        "after_offset": record_ready_after,
                    }),
                },
            )
            .unwrap();
        assert_eq!(
            writer_rerun
                .payload
                .get("processed_records")
                .and_then(|v| v.as_u64()),
            Some(0),
            "checkpoint restart should not reprocess record.ready offsets"
        );

        let conn = patina::eventlog::open_events_db().unwrap();
        let mut stmt = conn
            .prepare("SELECT data FROM eventlog WHERE event_type = 'measure.metric' ORDER BY seq")
            .unwrap();
        let metric_rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let expected_actions = [
            "scan",
            "extract-found",
            "enforce-schema",
            "filter-dedup",
            "write-records",
            "register-written",
        ]
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
        let mut throughput = 0_u64;
        let mut success = 0_u64;
        let mut errors = 0_u64;
        for raw in metric_rows {
            let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
            if value.get("source").and_then(|v| v.as_str()) != Some("mother") {
                continue;
            }
            if value.get("scope").and_then(|v| v.as_str()) != Some("child-handle-boundary") {
                continue;
            }

            let mut action: Option<String> = None;
            for label in value
                .get("labels")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
            {
                let pair = label.as_array().cloned().unwrap_or_default();
                if pair.len() == 2 && pair.first().and_then(|v| v.as_str()) == Some("action") {
                    action = pair
                        .get(1)
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string);
                }
            }
            if !action
                .as_deref()
                .is_some_and(|name| expected_actions.contains(name))
            {
                continue;
            }

            match value.get("name").and_then(|v| v.as_str()) {
                Some("mother_handle_throughput") => throughput += 1,
                Some("mother_handle_success") => success += 1,
                Some("mother_handle_error") => errors += 1,
                _ => {}
            }
        }

        assert!(throughput >= 6, "expected mother throughput measurements");
        assert!(success >= 6, "expected mother success measurements");
        assert_eq!(
            errors, 0,
            "expected zero mother handle errors for successful pipeline run"
        );

        let mut dedup_duplicate_output_rate = None;
        let mut provenance_completeness = None;
        for raw in stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        {
            let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
            let source = value.get("source").and_then(|v| v.as_str());
            let name = value.get("name").and_then(|v| v.as_str());
            let metric_value = value.get("value").and_then(|v| v.as_f64());
            if source == Some("dedup-filter") && name == Some("duplicate_output_rate_pct") {
                dedup_duplicate_output_rate = metric_value;
            }
            if source == Some("schema-enforcer") && name == Some("provenance_completeness_pct") {
                provenance_completeness = metric_value;
            }
        }
        assert_eq!(
            dedup_duplicate_output_rate,
            Some(0.0),
            "expected dedup duplicate output rate measurement to be 0%"
        );
        assert_eq!(
            provenance_completeness,
            Some(100.0),
            "expected full provenance completeness measurement"
        );
    });
}
