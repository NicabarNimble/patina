use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::{registry::ChildRegistry, Child, KnowledgeRuntimeStore};

pub enum LoadedChild {
    Knowledge {
        child: Box<dyn Child>,
        name: String,
        wasm_path: PathBuf,
        manifest_path: PathBuf,
        subscribed_streams: Vec<String>,
        relationship_listens: Vec<String>,
    },
}

fn loaded_child_name(loaded: &LoadedChild) -> &str {
    match loaded {
        LoadedChild::Knowledge { name, .. } => name,
    }
}

pub fn register_loaded_child(
    registry: &ChildRegistry,
    runtime: &KnowledgeRuntimeStore,
    loaded: LoadedChild,
) -> Result<Option<String>> {
    match loaded {
        LoadedChild::Knowledge {
            child,
            name,
            wasm_path,
            manifest_path,
            subscribed_streams,
            relationship_listens,
        } => {
            let mut routes: std::collections::HashSet<String> =
                subscribed_streams.into_iter().collect();
            routes.extend(relationship_listens);
            let routing_table = routes.into_iter().collect::<Vec<_>>();
            runtime.ensure_subscriptions(&name, &routing_table)?;
            registry.register_knowledge_with_paths(child, wasm_path, manifest_path)?;
            Ok(Some(format!("loaded knowledge WASM child: {}", name)))
        }
    }
}

pub fn load_children_from_dir<F>(
    children_dir: &Path,
    registry: &ChildRegistry,
    runtime: &KnowledgeRuntimeStore,
    mut loader: F,
) -> usize
where
    F: FnMut(&Path, &Path) -> Result<LoadedChild>,
{
    let mut loaded_count = 0usize;
    if !children_dir.exists() {
        return loaded_count;
    }

    if let Ok(entries) = std::fs::read_dir(children_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
                let manifest_path = path.with_extension("toml");
                tracing::info!(
                    event = "startup.child.discovery.begin",
                    wasm_path = %path.display(),
                    manifest_path = %manifest_path.display(),
                    "mother child discovery begin"
                );
                let started = Instant::now();
                match loader(&path, &manifest_path) {
                    Ok(loaded) => {
                        let child_name = loaded_child_name(&loaded).to_string();
                        match register_loaded_child(registry, runtime, loaded) {
                            Ok(Some(message)) => {
                                loaded_count += 1;
                                tracing::info!(
                                    event = "startup.child.discovery.success",
                                    child = %child_name,
                                    wasm_path = %path.display(),
                                    manifest_path = %manifest_path.display(),
                                    duration_ms = started.elapsed().as_millis() as u64,
                                    %message,
                                    "mother child discovery success"
                                )
                            }
                            Ok(None) => {
                                loaded_count += 1;
                                tracing::info!(
                                    event = "startup.child.discovery.success",
                                    child = %child_name,
                                    wasm_path = %path.display(),
                                    manifest_path = %manifest_path.display(),
                                    duration_ms = started.elapsed().as_millis() as u64,
                                    "mother child discovery success"
                                )
                            }
                            Err(error) => {
                                tracing::warn!(
                                    event = "startup.child.discovery.failure",
                                    child = %child_name,
                                    wasm_path = %path.display(),
                                    manifest_path = %manifest_path.display(),
                                    duration_ms = started.elapsed().as_millis() as u64,
                                    %error,
                                    "skipping child"
                                )
                            }
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            event = "startup.child.discovery.failure",
                            wasm_path = %path.display(),
                            manifest_path = %manifest_path.display(),
                            duration_ms = started.elapsed().as_millis() as u64,
                            %error,
                            "failed to load child"
                        );
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

    loaded_count
}
