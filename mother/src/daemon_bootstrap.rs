use anyhow::Result;
use std::path::Path;

use crate::{registry::ChildRegistry, KnowledgeChild, KnowledgeRuntimeStore, MotherChild};

pub enum LoadedChild {
    Legacy {
        child: Box<dyn MotherChild>,
        name: String,
    },
    Knowledge {
        child: Box<dyn KnowledgeChild>,
        name: String,
        subscribed_streams: Vec<String>,
        relationship_listens: Vec<String>,
    },
}

pub fn register_builtin_children(registry: &mut ChildRegistry) -> Result<()> {
    registry
        .register(Box::new(crate::secrets::SecretsCacheChild::new()))
        .expect("failed to register secrets child");
    registry
        .register(Box::new(crate::session_writer::SessionWriterChild::new()))
        .expect("failed to register session-writer child");
    registry
        .register(Box::new(crate::static_child::StaticChild::new(
            "spec-manager",
        )))
        .expect("failed to register spec-manager child marker");
    registry
        .register(Box::new(crate::static_child::StaticChild::new("doctor")))
        .expect("failed to register doctor child marker");
    registry
        .register(Box::new(crate::static_child::StaticChild::new(
            "lake-manager",
        )))
        .expect("failed to register lake-manager child marker");
    Ok(())
}

pub fn register_loaded_child(
    registry: &mut ChildRegistry,
    runtime: &KnowledgeRuntimeStore,
    loaded: LoadedChild,
    legacy_migration: bool,
) -> Result<Option<String>> {
    match loaded {
        LoadedChild::Legacy { child, name } => {
            if legacy_migration {
                registry.register_legacy(child)?;
                Ok(Some(format!("loaded legacy migration child: {}", name)))
            } else {
                Ok(Some(format!(
                    "skipping legacy child {} (use --legacy-migration to load mother-child plugins)",
                    name
                )))
            }
        }
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
    legacy_migration: bool,
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
                    Ok(loaded) => {
                        match register_loaded_child(registry, runtime, loaded, legacy_migration) {
                            Ok(Some(message)) => eprintln!("[mother] {}", message),
                            Ok(None) => {}
                            Err(error) => {
                                eprintln!("[mother] skipping {}: {}", path.display(), error)
                            }
                        }
                    }
                    Err(error) => {
                        eprintln!("[mother] failed to load {}: {}", path.display(), error);
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
                eprintln!("[mother] orphaned manifest (no .wasm): {}", path.display());
            }
        }
    }
}
