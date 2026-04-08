use anyhow::Result;
use std::time::Instant;

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
    let manifest = patina::child::engine::ChildManifest::from_path(manifest_path)?;
    tracing::info!(
        event = "startup.child.loader.manifest.success",
        child = child_label,
        duration_ms = parse_started.elapsed().as_millis() as u64,
        "mother child loader manifest parsed"
    );

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
            let child = engine.instantiate_child(&component, &manifest, None)?;
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
}
