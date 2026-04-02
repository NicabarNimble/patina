use anyhow::Result;
use std::path::Path;

use crate::{registry::ChildRegistry, KnowledgeChild, KnowledgeRuntimeStore};

pub enum LoadedChild {
    Knowledge {
        child: Box<dyn KnowledgeChild>,
        name: String,
        subscribed_streams: Vec<String>,
        relationship_listens: Vec<String>,
    },
}

pub fn register_loaded_child(
    registry: &mut ChildRegistry,
    runtime: &KnowledgeRuntimeStore,
    loaded: LoadedChild,
) -> Result<Option<String>> {
    match loaded {
        LoadedChild::Knowledge {
            child,
            name,
            subscribed_streams,
            relationship_listens,
        } => {
            let mut routes: std::collections::HashSet<String> =
                subscribed_streams.into_iter().collect();
            routes.extend(relationship_listens);
            let routing_table = routes.into_iter().collect::<Vec<_>>();
            runtime.ensure_subscriptions(&name, &routing_table)?;
            registry.register_knowledge(child)?;
            Ok(Some(format!("loaded knowledge WASM child: {}", name)))
        }
    }
}

pub fn load_children_from_dir<F>(
    children_dir: &Path,
    registry: &mut ChildRegistry,
    runtime: &KnowledgeRuntimeStore,
    mut loader: F,
) where
    F: FnMut(&Path, &Path) -> Result<LoadedChild>,
{
    if !children_dir.exists() {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(children_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
                let manifest_path = path.with_extension("toml");
                match loader(&path, &manifest_path) {
                    Ok(loaded) => match register_loaded_child(registry, runtime, loaded) {
                        Ok(Some(message)) => tracing::info!(%message, "child loaded"),
                        Ok(None) => {}
                        Err(error) => {
                            tracing::warn!(path = %path.display(), %error, "skipping child")
                        }
                    },
                    Err(error) => {
                        tracing::warn!(path = %path.display(), %error, "failed to load child");
                    }
                }
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir(children_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml")
                && !path.with_extension("wasm").exists()
            {
                tracing::warn!(path = %path.display(), "orphaned manifest (no .wasm)");
            }
        }
    }
}
