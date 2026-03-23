use anyhow::Result;

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
    let manifest = patina::child::engine::ChildManifest::from_path(manifest_path)?;
    let relationship_listens = parse_relationship_listens(manifest_path)?;
    let wasm_bytes = std::fs::read(wasm_path)?;
    match manifest.world {
        patina::child::engine::ChildKind::KnowledgeChild => {
            let engine = patina::child::engine::KnowledgeChildEngine::new()?;
            let component = engine.load_component(&wasm_bytes)?;
            let child = engine.instantiate_child(&component, &manifest, None)?;
            let name = child.name().to_string();
            Ok(mother_crate::daemon_bootstrap::LoadedChild::Knowledge {
                child,
                name,
                subscribed_streams: manifest.subscribed_streams.clone(),
                relationship_listens,
            })
        }
        patina::child::engine::ChildKind::MotherChild => {
            let engine = patina::child::engine::MotherChildEngine::new()?;
            let component = engine.load_component(&wasm_bytes)?;
            let child = engine.instantiate_child(&component, &manifest, None)?;
            let name = child.name().to_string();
            Ok(mother_crate::daemon_bootstrap::LoadedChild::Legacy { child, name })
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
