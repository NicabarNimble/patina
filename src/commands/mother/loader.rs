use anyhow::Result;
use std::time::Instant;

use super::audit;
use super::integrity;

fn current_project_preopens(
    manifest: &patina::child::engine::ChildManifest,
) -> Vec<patina::child::engine::FilesystemPreopen> {
    if !manifest.toys.filesystem {
        return Vec::new();
    }

    patina::session::SessionManager::find_project_root()
        .map(|root| vec![patina::child::engine::FilesystemPreopen::project_read_write(root)])
        .unwrap_or_default()
}

pub(super) fn parse_relationship_listens(manifest_path: &std::path::Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(manifest_path)?;
    let table: toml::Table = content.parse()?;

    let listens = table
        .get("relationships")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("listens"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(listens)
}

pub(super) fn load_wasm_child(
    wasm_path: &std::path::Path,
    manifest_path: &std::path::Path,
) -> Result<mother_crate::daemon_bootstrap::LoadedChild> {
    let child_label = wasm_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unknown");

    tracing::info!(
        event = "startup.child.loader.begin",
        child = child_label,
        wasm_path = %wasm_path.display(),
        manifest_path = %manifest_path.display(),
        "mother child loader begin"
    );

    let parse_started = Instant::now();
    integrity::verify_hash_file(manifest_path)?;
    integrity::verify_hash_file(wasm_path)?;
    let manifest = patina::child::engine::ChildManifest::from_path(manifest_path)?;
    tracing::info!(
        event = "startup.child.loader.manifest.success",
        child = child_label,
        duration_ms = parse_started.elapsed().as_millis() as u64,
        "mother child loader manifest parsed"
    );

    for decision in
        audit::evaluate_outside_capability_decisions(&manifest.world, &manifest.capabilities)
    {
        audit::emit_outside_capability_audit(
            &manifest.name,
            &decision.capability,
            decision.outcome,
            &decision.reason,
        );
    }

    let relationship_listens = parse_relationship_listens(manifest_path)?;

    let read_started = Instant::now();
    let wasm_bytes = std::fs::read(wasm_path)?;
    tracing::info!(
        event = "startup.child.loader.read.success",
        child = child_label,
        wasm_bytes = wasm_bytes.len(),
        duration_ms = read_started.elapsed().as_millis() as u64,
        "mother child loader wasm read"
    );

    match manifest.world {
        patina::child::engine::ChildKind::Child => {
            let engine_started = Instant::now();
            let engine = patina::child::engine::ChildEngine::new()?;
            tracing::info!(
                event = "startup.child.loader.engine.success",
                child = child_label,
                duration_ms = engine_started.elapsed().as_millis() as u64,
                "mother child loader engine ready"
            );

            let component_started = Instant::now();
            let component = engine.load_component(&wasm_bytes)?;
            tracing::info!(
                event = "startup.child.loader.component.success",
                child = child_label,
                duration_ms = component_started.elapsed().as_millis() as u64,
                "mother child loader component ready"
            );

            let instantiate_started = Instant::now();
            let preopens = current_project_preopens(&manifest);
            let child = match engine
                .instantiate_child_with_preopens(&component, &manifest, None, &preopens)
            {
                Ok(child) => child,
                Err(error) => {
                    audit::emit_outside_capability_audit(
                        &manifest.name,
                        "manifest.capability-check",
                        "DENY",
                        &error.to_string(),
                    );
                    return Err(error);
                }
            };
            tracing::info!(
                event = "startup.child.loader.instantiate.success",
                child = child_label,
                duration_ms = instantiate_started.elapsed().as_millis() as u64,
                "mother child loader instantiate success"
            );

            let name = child.name().to_string();
            Ok(mother_crate::daemon_bootstrap::LoadedChild::Knowledge {
                child,
                name,
                wasm_path: wasm_path.to_path_buf(),
                manifest_path: manifest_path.to_path_buf(),
                subscribed_streams: manifest.subscribed_streams.clone(),
                relationship_listens,
            })
        }
        other => anyhow::bail!(
            "child manifest world '{}' is not loadable by the daemon child loader",
            other
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::Path;

    fn with_temp_project<T>(f: impl FnOnce(&Path) -> T) -> T {
        let _guard = patina::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::TempDir::new().unwrap();
        let project_root = temp.path().join("project");
        std::fs::create_dir_all(&project_root).unwrap();

        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&project_root)));
        std::env::set_current_dir(old_cwd).unwrap();

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    fn latest_grant_payload() -> Value {
        let conn = patina::eventlog::open_events_db().expect("open events db");
        let data: String = conn
            .query_row(
                "SELECT data FROM eventlog WHERE event_type = 'mother.grant' ORDER BY seq DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("query mother.grant event");
        serde_json::from_str(&data).expect("parse mother.grant payload")
    }

    fn grant_payloads_for_child(child_name: &str) -> Vec<Value> {
        let conn = patina::eventlog::open_events_db().expect("open events db");
        let mut stmt = conn
            .prepare("SELECT data FROM eventlog WHERE event_type = 'mother.grant' ORDER BY seq ASC")
            .expect("prepare grant payload query");
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query grant payload rows");

        rows.filter_map(|row| {
            let data = row.ok()?;
            let payload: Value = serde_json::from_str(&data).ok()?;
            (payload.get("child").and_then(|v| v.as_str()) == Some(child_name)).then_some(payload)
        })
        .collect()
    }

    fn fixture_path(rel: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
    }

    #[test]
    fn parse_relationship_listens_from_manifest() {
        let temp = tempfile::TempDir::new().unwrap();
        let manifest = temp.path().join("child.toml");
        std::fs::write(
            &manifest,
            r#"
[child]
name = "child"
kind = "knowledge-child"

[relationships]
emits = ["x"]
listens = ["data-ingested", "belief.changed"]
"#,
        )
        .unwrap();

        let listens = parse_relationship_listens(&manifest).unwrap();
        assert_eq!(
            listens,
            vec!["data-ingested".to_string(), "belief.changed".to_string()]
        );
    }

    #[test]
    fn parse_relationship_listens_defaults_empty() {
        let temp = tempfile::TempDir::new().unwrap();
        let manifest = temp.path().join("child.toml");
        std::fs::write(
            &manifest,
            r#"
[child]
name = "child"
kind = "knowledge-child"
"#,
        )
        .unwrap();

        let listens = parse_relationship_listens(&manifest).unwrap();
        assert!(listens.is_empty());
    }

    #[test]
    fn load_wasm_child_emits_outside_deny_audit_for_disallowed_capability() {
        with_temp_project(|project_root| {
            let wasm_path = project_root.join("bad-child.wasm");
            let manifest_path = project_root.join("bad-child.toml");

            std::fs::write(&wasm_path, b"not-a-real-component").unwrap();
            std::fs::write(
                &manifest_path,
                r#"
[child]
name = "bad-child"
version = "0.1.0"
kind = "child"

[capabilities]
host_unknown = true
"#,
            )
            .unwrap();
            crate::commands::mother::integrity::write_hash_file(&wasm_path).unwrap();
            crate::commands::mother::integrity::write_hash_file(&manifest_path).unwrap();

            let err = match load_wasm_child(&wasm_path, &manifest_path) {
                Ok(_) => panic!("load should fail for invalid capability"),
                Err(error) => error,
            };
            assert!(
                err.to_string().contains("failed to parse WebAssembly")
                    || err.to_string().contains("not allowed for this world"),
                "unexpected error: {}",
                err
            );

            let payload = latest_grant_payload();
            assert_eq!(payload["scope"], "outside");
            assert_eq!(payload["outcome"], "DENY");
            assert_eq!(payload["child"], "bad-child");
            assert_eq!(payload["toy_or_capability"], "host_unknown");
            assert!(payload["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("not allowed"));
        });
    }

    #[test]
    fn load_wasm_child_emits_outside_grant_audit_for_allowed_capability() {
        with_temp_project(|project_root| {
            let wasm_path = project_root.join("grant-child.wasm");
            let manifest_path = project_root.join("grant-child.toml");

            std::fs::write(&wasm_path, b"not-a-real-component").unwrap();
            std::fs::write(
                &manifest_path,
                r#"
[child]
name = "grant-child"
version = "0.1.0"
kind = "child"

[capabilities]
host_log = true
"#,
            )
            .unwrap();
            crate::commands::mother::integrity::write_hash_file(&wasm_path).unwrap();
            crate::commands::mother::integrity::write_hash_file(&manifest_path).unwrap();

            let _ = load_wasm_child(&wasm_path, &manifest_path);

            let payload = latest_grant_payload();
            assert_eq!(payload["scope"], "outside");
            assert_eq!(payload["outcome"], "GRANT");
            assert_eq!(payload["child"], "grant-child");
            assert_eq!(payload["toy_or_capability"], "host_log");
            assert!(payload["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("allowed for world"));
        });
    }

    #[test]
    fn handle_based_service_child_loads_with_allowed_capability() {
        with_temp_project(|project_root| {
            let wasm_path = project_root.join("belief-verifier-child.wasm");
            let manifest_path = project_root.join("belief-verifier-child.toml");
            let fixture_wasm =
                fixture_path("tests/fixtures/mother-service/belief-verifier-child.wasm");
            let fixture_manifest =
                fixture_path("tests/fixtures/mother-service/belief-verifier-child.toml");
            assert!(
                fixture_wasm.exists(),
                "missing fixture {}",
                fixture_wasm.display()
            );
            assert!(
                fixture_manifest.exists(),
                "missing fixture {}",
                fixture_manifest.display()
            );
            std::fs::copy(&fixture_wasm, &wasm_path).expect("copy belief-verifier fixture wasm");
            let mut manifest = std::fs::read_to_string(&fixture_manifest)
                .expect("read belief-verifier fixture manifest");
            manifest.push_str("\n[capabilities]\nhost_log = true\n");
            std::fs::write(&manifest_path, manifest).expect("write allowed manifest");
            crate::commands::mother::integrity::write_hash_file(&wasm_path).unwrap();
            crate::commands::mother::integrity::write_hash_file(&manifest_path).unwrap();

            let loaded = load_wasm_child(&wasm_path, &manifest_path)
                .expect("service child should still load with allowed capabilities");
            match loaded {
                mother_crate::daemon_bootstrap::LoadedChild::Knowledge { .. } => {}
            }

            let payload = latest_grant_payload();
            assert_eq!(payload["scope"], "outside");
            assert_eq!(payload["outcome"], "GRANT");
            assert_eq!(payload["child"], "patina-belief-verifier");
            assert_eq!(payload["toy_or_capability"], "host_log");
        });
    }

    #[test]
    fn handle_based_service_child_stays_fail_closed_for_disallowed_capability() {
        with_temp_project(|project_root| {
            let wasm_path = project_root.join("belief-verifier-child-deny.wasm");
            let manifest_path = project_root.join("belief-verifier-child-deny.toml");
            let fixture_wasm =
                fixture_path("tests/fixtures/mother-service/belief-verifier-child.wasm");
            let fixture_manifest =
                fixture_path("tests/fixtures/mother-service/belief-verifier-child.toml");
            assert!(
                fixture_wasm.exists(),
                "missing fixture {}",
                fixture_wasm.display()
            );
            assert!(
                fixture_manifest.exists(),
                "missing fixture {}",
                fixture_manifest.display()
            );
            std::fs::copy(&fixture_wasm, &wasm_path).expect("copy belief-verifier fixture wasm");
            let mut manifest = std::fs::read_to_string(&fixture_manifest)
                .expect("read belief-verifier fixture manifest");
            manifest.push_str("\n[capabilities]\nhost_unknown = true\n");
            std::fs::write(&manifest_path, manifest).expect("write deny manifest");
            crate::commands::mother::integrity::write_hash_file(&wasm_path).unwrap();
            crate::commands::mother::integrity::write_hash_file(&manifest_path).unwrap();

            let err = match load_wasm_child(&wasm_path, &manifest_path) {
                Ok(_) => panic!("service child should fail for disallowed capability"),
                Err(error) => error,
            };
            assert!(
                err.to_string().contains("not allowed for this world"),
                "unexpected error: {}",
                err
            );

            let payloads = grant_payloads_for_child("patina-belief-verifier");
            assert!(
                payloads.iter().any(|payload| {
                    payload["scope"] == "outside"
                        && payload["outcome"] == "DENY"
                        && payload["toy_or_capability"] == "host_unknown"
                        && payload["reason"]
                            .as_str()
                            .unwrap_or_default()
                            .contains("not allowed")
                }),
                "missing DENY payload for requested disallowed capability: {payloads:?}"
            );
            assert!(
                payloads.iter().any(|payload| {
                    payload["scope"] == "outside"
                        && payload["outcome"] == "DENY"
                        && payload["toy_or_capability"] == "manifest.capability-check"
                        && payload["reason"]
                            .as_str()
                            .unwrap_or_default()
                            .contains("not allowed")
                }),
                "missing DENY payload for manifest capability check failure: {payloads:?}"
            );
        });
    }
}
