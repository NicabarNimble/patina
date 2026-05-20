use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize)]
struct ContextQueryParams {
    topic: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ScryQueryParams {
    query: Option<String>,
    limit: Option<usize>,
    repo: Option<String>,
    #[serde(default)]
    all_repos: bool,
}

#[derive(Debug, Default, Deserialize)]
struct AssayQueryParams {
    query_type: Option<String>,
    pattern: Option<String>,
    limit: Option<usize>,
    repo: Option<String>,
    #[serde(default)]
    all_repos: bool,
    query: Option<String>,
}

fn parse_query_params<T: serde::de::DeserializeOwned>(params: &str) -> Result<T, String> {
    serde_json::from_str(params).map_err(|e| format!("invalid params: {}", e))
}

fn dispatch_context_query(params: &str) -> Result<String, String> {
    let args: ContextQueryParams = parse_query_params(params)?;
    crate::commands::context::get_project_context(args.topic.as_deref())
        .map_err(|e| format!("context: {}", e))
}

fn dispatch_scry_query(
    query_engine: &mut Option<crate::retrieval::QueryEngine>,
    params: &str,
) -> Result<String, String> {
    let args: ScryQueryParams = parse_query_params(params)?;
    let query_str = args.query.unwrap_or_default();
    if query_str.trim().is_empty() {
        return Err("scry requires 'query' parameter".to_string());
    }

    let limit = args.limit.unwrap_or(10);
    let engine = query_engine.get_or_insert_with(crate::retrieval::QueryEngine::new);
    let options = crate::retrieval::QueryOptions {
        repo: args.repo,
        all_repos: args.all_repos,
        ..Default::default()
    };
    let results = engine
        .query_with_options(&query_str, limit, &options)
        .map_err(|e| format!("scry: {}", e))?;

    // JSON array for structured guest consumption
    let json_results: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "score": r.fused_score,
                "doc_id": r.doc_id,
                "content": r.content,
            })
        })
        .collect();
    serde_json::to_string(&json_results).map_err(|e| format!("serialize: {}", e))
}

fn dispatch_assay_query(params: &str) -> Result<String, String> {
    use crate::commands::assay::{AssayOptions, QueryType};

    let args: AssayQueryParams = parse_query_params(params)?;
    let query_type = args.query_type.as_deref().unwrap_or("inventory");
    let pattern = args.pattern;
    let query = args.query;
    let qt = match query_type {
        "imports" => QueryType::Imports,
        "importers" => QueryType::Importers,
        "functions" => QueryType::Functions,
        "callers" => QueryType::Callers,
        "callees" => QueryType::Callees,
        "derive" => QueryType::Derive,
        "search" => {
            let q = query.or_else(|| pattern.clone()).unwrap_or_default();
            if q.is_empty() {
                return Err("assay search requires 'query' or 'pattern'".into());
            }
            QueryType::Search { query: q }
        }
        "cochange" => {
            let file = pattern.clone().unwrap_or_default();
            if file.is_empty() {
                return Err("assay cochange requires 'pattern'".into());
            }
            QueryType::Cochange { file }
        }
        "belief" => {
            let id = pattern.clone().unwrap_or_default();
            if id.is_empty() {
                return Err("assay belief requires 'pattern'".into());
            }
            QueryType::Belief { id }
        }
        _ => QueryType::Inventory,
    };

    let options = AssayOptions {
        query_type: qt,
        pattern,
        limit: args.limit.unwrap_or(50),
        json: true,
        repo: args.repo,
        all_repos: args.all_repos,
        ..Default::default()
    };

    crate::commands::assay::execute_query(&options).map_err(|e| format!("assay: {}", e))
}

/// Build a query dispatch closure for children with query grants.
///
/// Returns None if the child has no host_query grants. Otherwise,
/// returns a closure that dispatches to context/scry/assay engines.
pub(crate) fn make_query_dispatch(
    manifest: &patina::child::engine::ChildManifest,
) -> Option<patina::child::engine::QueryDispatchFn> {
    if manifest.host_query_kinds.is_empty() {
        return None;
    }

    // Lazy QueryEngine — created on first scry call
    let mut query_engine: Option<crate::retrieval::QueryEngine> = None;

    Some(Box::new(move |kind: &str, params: &str| match kind {
        "context" => dispatch_context_query(params),
        "scry" => dispatch_scry_query(&mut query_engine, params),
        "assay" => dispatch_assay_query(params),
        _ => Err(format!("unknown query kind: {}", kind)),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scry_query_requires_query_field() {
        let mut query_engine: Option<crate::retrieval::QueryEngine> = None;
        let error = dispatch_scry_query(&mut query_engine, "{}")
            .expect_err("missing query should fail closed");
        assert!(error.contains("scry requires 'query' parameter"), "{error}");
    }

    #[test]
    fn assay_search_requires_query_or_pattern() {
        let error = dispatch_assay_query(r#"{"query_type":"search"}"#)
            .expect_err("search without query/pattern should fail closed");
        assert!(
            error.contains("assay search requires 'query' or 'pattern'"),
            "{error}"
        );
    }

    #[test]
    fn assay_cochange_requires_pattern() {
        let error = dispatch_assay_query(r#"{"query_type":"cochange"}"#)
            .expect_err("cochange without pattern should fail closed");
        assert!(
            error.contains("assay cochange requires 'pattern'"),
            "{error}"
        );
    }

    #[test]
    fn assay_belief_requires_pattern() {
        let error = dispatch_assay_query(r#"{"query_type":"belief"}"#)
            .expect_err("belief without pattern should fail closed");
        assert!(error.contains("assay belief requires 'pattern'"), "{error}");
    }

    #[test]
    fn invalid_json_params_fail_closed() {
        let error =
            dispatch_context_query("not-json").expect_err("invalid params should fail closed");
        assert!(error.contains("invalid params:"), "{error}");
    }
}

fn install_child_package(
    package_path: &Path,
    wasm_arg: Option<&str>,
    force: bool,
    preserve_local_scopes: bool,
) -> Result<()> {
    let package_path = package_path
        .canonicalize()
        .with_context(|| format!("resolve child package path {}", package_path.display()))?;
    let child_toml = package_path.join("child.toml");
    if !child_toml.exists() {
        anyhow::bail!(
            "child package missing child.toml at {}",
            child_toml.display()
        );
    }

    let manifest = patina::child::engine::ChildManifest::from_path(&child_toml)
        .with_context(|| format!("load child manifest {}", child_toml.display()))?;
    let wasm_path = resolve_child_wasm(&package_path, &manifest.name, wasm_arg)?;

    let install_dir = patina::paths::child::command_children_dir();
    std::fs::create_dir_all(&install_dir)?;
    let child_dir = install_dir.join(&manifest.name);
    let dest_wasm = install_dir.join(format!("{}.wasm", manifest.name));
    let dest_toml = install_dir.join(format!("{}.toml", manifest.name));

    if !force && (dest_wasm.exists() || dest_toml.exists() || child_dir.exists()) {
        anyhow::bail!(
            "child '{}' is already installed; rerun with --force to overwrite",
            manifest.name
        );
    }

    std::fs::copy(&wasm_path, &dest_wasm).with_context(|| {
        format!(
            "copy component {} -> {}",
            wasm_path.display(),
            dest_wasm.display()
        )
    })?;

    let manifest_text = if preserve_local_scopes && dest_toml.exists() {
        preserve_existing_scope_additions(&std::fs::read_to_string(&child_toml)?, &dest_toml)?
    } else {
        std::fs::read_to_string(&child_toml)?
    };
    std::fs::write(&dest_toml, manifest_text)
        .with_context(|| format!("write installed manifest {}", dest_toml.display()))?;

    if child_dir.exists() {
        std::fs::remove_dir_all(&child_dir)
            .with_context(|| format!("remove existing {}", child_dir.display()))?;
    }
    std::fs::create_dir_all(&child_dir)?;
    copy_optional_child_dir(&package_path, &child_dir, "skills")?;
    copy_optional_child_dir(&package_path, &child_dir, "wit")?;
    copy_optional_child_dir(&package_path, &child_dir, "wit-contract")?;

    println!("Installed child '{}':", manifest.name);
    println!("  wasm:     {}", dest_wasm.display());
    println!("  manifest: {}", dest_toml.display());
    println!("  package:  {}", child_dir.display());
    Ok(())
}

fn resolve_child_wasm(
    package_path: &Path,
    child_name: &str,
    wasm_arg: Option<&str>,
) -> Result<PathBuf> {
    if let Some(path) = wasm_arg {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
        anyhow::bail!("component wasm not found at {}", path.display());
    }

    let artifact_name = format!("{}.wasm", child_name.replace('-', "_"));
    let candidates = [
        package_path
            .join("target/wasm32-wasip1/release")
            .join(&artifact_name),
        package_path
            .join("target/wasm32-wasip2/release")
            .join(&artifact_name),
        std::env::current_dir()?
            .join("target/wasm32-wasip1/release")
            .join(&artifact_name),
        std::env::current_dir()?
            .join("target/wasm32-wasip2/release")
            .join(&artifact_name),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    anyhow::bail!(
        "could not find built component for '{}'; pass --wasm <path>",
        child_name
    );
}

fn copy_optional_child_dir(package_path: &Path, child_dir: &Path, name: &str) -> Result<()> {
    let source = package_path.join(name);
    if source.exists() {
        copy_dir_recursive(&source, &child_dir.join(name))?;
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

fn preserve_existing_scope_additions(
    new_manifest: &str,
    existing_manifest_path: &Path,
) -> Result<String> {
    let mut new_value: toml::Value = new_manifest.parse()?;
    let existing_manifest = std::fs::read_to_string(existing_manifest_path)?;
    let existing_value: toml::Value = existing_manifest.parse()?;

    if let Some(existing_scopes) = existing_value
        .get("needs")
        .and_then(|needs| needs.get("scopes"))
        .and_then(|scopes| scopes.as_table())
    {
        let new_scopes = new_value
            .as_table_mut()
            .and_then(|root| root.get_mut("needs"))
            .and_then(|needs| needs.as_table_mut())
            .and_then(|needs| needs.get_mut("scopes"))
            .and_then(|scopes| scopes.as_table_mut());
        if let Some(new_scopes) = new_scopes {
            merge_missing_tables(new_scopes, existing_scopes);
        }
    }

    Ok(toml::to_string_pretty(&new_value)?)
}

fn merge_missing_tables(
    target: &mut toml::map::Map<String, toml::Value>,
    source: &toml::map::Map<String, toml::Value>,
) {
    for (key, source_value) in source {
        match (target.get_mut(key), source_value.as_table()) {
            (Some(toml::Value::Table(target_table)), Some(source_table)) => {
                merge_missing_tables(target_table, source_table);
            }
            (None, _) => {
                target.insert(key.clone(), source_value.clone());
            }
            _ => {}
        }
    }
}

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

pub(crate) fn dispatch(command: crate::ChildCommands) -> Result<()> {
    match command {
        crate::ChildCommands::List => crate::commands::child::execute_list()?,
        crate::ChildCommands::Init {
            name,
            world,
            legacy,
            build,
            release,
        } => {
            let world: patina::child::engine::ChildKind = world.parse()?;
            let lane = if legacy {
                patina::child::scaffold::ScaffoldLane::Legacy
            } else {
                patina::child::scaffold::ScaffoldLane::Typed
            };
            let cwd = std::env::current_dir()?;
            let project_dir = patina::child::scaffold::scaffold(&cwd, &name, &world, lane)?;

            let profile = if release { "release" } else { "debug" };
            let artifact = project_dir
                .join(format!("target/wasm32-wasip2/{}", profile))
                .join(format!("{}.wasm", name.replace('-', "_")));

            println!(
                "Created {} {} child: {}",
                if legacy { "legacy" } else { "typed" },
                world,
                project_dir.display()
            );
            println!();
            println!("  cd {}", name);
            if release {
                println!("  cargo build --target wasm32-wasip2 --release");
            } else {
                println!("  cargo build --target wasm32-wasip2");
            }
            if legacy {
                println!();
                println!(
                    "Legacy scaffold lane is maintenance-only. Prefer default typed lane for new children."
                );
            }
            println!();
            println!("Artifact will be at: {}", artifact.display());

            if build {
                // Proactive rustup check before attempting the build
                let has_target = std::process::Command::new("rustup")
                    .args(["target", "list", "--installed"])
                    .output()
                    .map(|o| {
                        String::from_utf8_lossy(&o.stdout)
                            .lines()
                            .any(|l| l.trim() == "wasm32-wasip2")
                    })
                    .unwrap_or(false);

                if !has_target {
                    eprintln!("Missing wasm32-wasip2 target. Install it:");
                    eprintln!("  rustup target add wasm32-wasip2");
                    std::process::exit(1);
                }

                println!();
                println!("Building ({})...", profile);
                let mut cargo_args = vec!["build", "--target", "wasm32-wasip2"];
                if release {
                    cargo_args.push("--release");
                }

                let status = std::process::Command::new("cargo")
                    .args(&cargo_args)
                    .current_dir(&project_dir)
                    .status();

                match status {
                    Ok(s) if s.success() => {
                        println!("Built: {}", artifact.display());
                    }
                    Ok(s) => {
                        eprintln!("Build failed (exit code {})", s.code().unwrap_or(-1));
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Failed to run cargo: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        crate::ChildCommands::Install {
            path,
            wasm,
            force,
            no_preserve_local_scopes,
        } => {
            install_child_package(
                Path::new(&path),
                wasm.as_deref(),
                force,
                !no_preserve_local_scopes,
            )?;
        }
        crate::ChildCommands::Run { name, args } => {
            let command_children_dir = patina::paths::child::command_children_dir();
            let wasm_path = command_children_dir.join(format!("{}.wasm", name));
            let toml_path = command_children_dir.join(format!("{}.toml", name));

            if !wasm_path.exists() {
                anyhow::bail!(
                    "child '{}' not found at {}\nInstall: cp {}.wasm {}",
                    name,
                    wasm_path.display(),
                    name,
                    command_children_dir.display()
                );
            }

            let manifest = if toml_path.exists() {
                patina::child::engine::ChildManifest::from_path(&toml_path)?
            } else {
                anyhow::bail!(
                    "child manifest not found at {}\nKnowledge-child and pipeline children require a .toml manifest",
                    toml_path.display()
                );
            };

            let wasm_bytes = std::fs::read(&wasm_path)?;

            // Auto-detect world from manifest and dispatch
            match &manifest.world {
                patina::child::engine::ChildKind::Child => {
                    let action = args.first().map(|s| s.as_str()).unwrap_or("health");
                    let payload_str = args.get(1).map(|s| s.as_str()).unwrap_or("{}");

                    let engine = patina::child::engine::ChildEngine::new()?;
                    let component = engine.load_component(&wasm_bytes)?;
                    let query_fn = make_query_dispatch(&manifest);
                    let preopens = current_project_preopens(&manifest);
                    let mut child = engine.instantiate_child_with_preopens(
                        &component, &manifest, query_fn, &preopens,
                    )?;

                    use patina::mother::MotherHost;
                    struct CliHost;
                    impl MotherHost for CliHost {
                        fn log(&self, child: &str, message: &str) {
                            eprintln!("[{}] {}", child, message);
                        }
                    }
                    child.on_load(&CliHost)?;

                    if action == "health" {
                        let health = child.health();
                        println!("{:?}", health);
                    } else if action == "tick" {
                        println!("{}", serde_json::to_string_pretty(&child.tick())?);
                    } else if action == "drain" {
                        let limit = payload_str.parse::<u32>().unwrap_or(64);
                        match child.drain(limit) {
                            Ok(events) => {
                                println!("{}", serde_json::to_string_pretty(&events)?)
                            }
                            Err(e) => {
                                eprintln!("error: {}", e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        let request = patina::mother::ChildRequest {
                            action: action.to_string(),
                            payload: serde_json::from_str(payload_str)
                                .unwrap_or(serde_json::Value::String(payload_str.to_string())),
                        };
                        match child.handle(&request) {
                            Ok(response) => {
                                println!("{}", serde_json::to_string_pretty(&response.payload)?);
                            }
                            Err(e) => {
                                eprintln!("error: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                }
                patina::child::engine::ChildKind::Pipeline => {
                    let request = args.first().map(|s| s.as_str()).unwrap_or("{}");
                    let engine = patina::child::engine::PipelineEngine::new()?;
                    let component = engine.load_component(&wasm_bytes)?;
                    let response = engine.handle(&component, &name, request)?;
                    println!("{}", response);
                }
            }
        }
        crate::ChildCommands::Call {
            name,
            operation_id,
            args_json,
        } => {
            let command_children_dir = patina::paths::child::command_children_dir();
            let wasm_path = command_children_dir.join(format!("{}.wasm", name));
            let toml_path = command_children_dir.join(format!("{}.toml", name));

            if !wasm_path.exists() {
                anyhow::bail!(
                    "child '{}' not found at {}\nInstall: cp {}.wasm {}",
                    name,
                    wasm_path.display(),
                    name,
                    command_children_dir.display()
                );
            }

            let manifest = if toml_path.exists() {
                patina::child::engine::ChildManifest::from_path(&toml_path)?
            } else {
                anyhow::bail!(
                    "child manifest not found at {}\nTyped child calls require a .toml manifest",
                    toml_path.display()
                );
            };

            if manifest.world != patina::child::engine::ChildKind::Child {
                anyhow::bail!(
                    "child '{}' is world '{}' and does not support typed child call",
                    name,
                    manifest.world
                );
            }

            let args: serde_json::Value = serde_json::from_str(&args_json)
                .map_err(|e| anyhow::anyhow!("invalid args_json: {}", e))?;

            let wasm_bytes = std::fs::read(&wasm_path)?;
            let engine = patina::child::engine::ChildEngine::new()?;
            let component = engine.load_component(&wasm_bytes)?;
            let query_fn = make_query_dispatch(&manifest);
            let preopens = current_project_preopens(&manifest);
            let mut child = engine
                .instantiate_child_with_preopens(&component, &manifest, query_fn, &preopens)?;

            use patina::mother::MotherHost;
            struct CliHost;
            impl MotherHost for CliHost {
                fn log(&self, child: &str, message: &str) {
                    eprintln!("[{}] {}", child, message);
                }
            }
            child.on_load(&CliHost)?;

            let request = patina::mother::ChildCallRequest {
                operation_id,
                args,
                correlation: None,
            };
            match child.call(&request) {
                Ok(response) => {
                    println!("{}", serde_json::to_string_pretty(&response.payload)?)
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod child_install_tests {
    use super::*;

    #[test]
    fn merge_missing_tables_preserves_local_nested_scope() {
        let mut new_scopes: toml::map::Map<String, toml::Value> = toml::toml! {
            [filesystem]
            path = "/"
        };
        let existing_scopes: toml::map::Map<String, toml::Value> = toml::toml! {
            [filesystem]
            path = "/"
            [filesystem.project]
            path = "/repo"
            mode = "read-write"
        };

        merge_missing_tables(&mut new_scopes, &existing_scopes);
        assert_eq!(
            new_scopes
                .get("filesystem")
                .and_then(|v| v.get("project"))
                .and_then(|v| v.get("mode"))
                .and_then(|v| v.as_str()),
            Some("read-write")
        );
    }

    #[test]
    fn resolve_child_wasm_requires_artifact_when_missing() {
        let temp = tempfile::tempdir().unwrap();
        let error = resolve_child_wasm(temp.path(), "demo-child", None)
            .expect_err("missing component should fail closed");
        assert!(error.to_string().contains("could not find built component"));
    }
}
