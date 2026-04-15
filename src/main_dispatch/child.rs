use anyhow::Result;

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

    Some(Box::new(move |kind: &str, params: &str| {
        let args: serde_json::Value =
            serde_json::from_str(params).map_err(|e| format!("invalid params: {}", e))?;

        match kind {
            "context" => {
                let topic = args.get("topic").and_then(|v| v.as_str());
                crate::commands::context::get_project_context(topic)
                    .map_err(|e| format!("context: {}", e))
            }
            "scry" => {
                let query_str = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                if query_str.is_empty() {
                    return Err("scry requires 'query' parameter".to_string());
                }
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                let all_repos = args
                    .get("all_repos")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let repo = args.get("repo").and_then(|v| v.as_str()).map(String::from);

                let engine = query_engine.get_or_insert_with(crate::retrieval::QueryEngine::new);
                let options = crate::retrieval::QueryOptions {
                    repo,
                    all_repos,
                    ..Default::default()
                };
                let results = engine
                    .query_with_options(query_str, limit, &options)
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
            "assay" => {
                use crate::commands::assay::{AssayOptions, QueryType};

                let query_type = args
                    .get("query_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("inventory");
                let pattern = args
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
                let all_repos = args
                    .get("all_repos")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let repo = args.get("repo").and_then(|v| v.as_str()).map(String::from);
                let query = args.get("query").and_then(|v| v.as_str()).map(String::from);

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
                    limit,
                    json: true,
                    repo,
                    all_repos,
                    ..Default::default()
                };

                crate::commands::assay::execute_query(&options).map_err(|e| format!("assay: {}", e))
            }
            _ => Err(format!("unknown query kind: {}", kind)),
        }
    }))
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
                    let mut child = engine.instantiate_child(&component, &manifest, query_fn)?;

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
            let mut child = engine.instantiate_child(&component, &manifest, query_fn)?;

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
