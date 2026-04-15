use super::*;

impl ServerState {
    pub(super) fn typed_interface_candidates(interface_name: &str) -> Vec<String> {
        if interface_name.contains('@') {
            vec![interface_name.to_string()]
        } else {
            vec![
                interface_name.to_string(),
                format!("{}@0.1.0", interface_name),
            ]
        }
    }

    pub(super) fn resolve_child_instance<'a>(
        manifest: &'a mother_crate::pando::PandoManifest,
        instance: &str,
    ) -> Option<&'a mother_crate::pando::PandoChild> {
        manifest
            .children
            .iter()
            .find(|child| child.id.as_deref().unwrap_or(&child.name) == instance)
    }

    pub(super) fn validate_typed_composition(
        &self,
        manifest: &mother_crate::pando::PandoManifest,
    ) -> Result<()> {
        let Some(composition) = &manifest.composition else {
            return Ok(());
        };
        if composition
            .wiring
            .iter()
            .any(|rule| matches!(rule, mother_crate::pando::PandoWiring::Typed(_)))
        {
            let _ = self.compose_typed_component(manifest)?;
        }
        Ok(())
    }

    pub(super) fn component_wasm_path(&self, name: &str) -> Result<PathBuf> {
        if let Some((wasm_path, _)) = self.registry.child_paths(name) {
            return Ok(wasm_path);
        }

        let stem = name.replace('-', "_");
        let mut candidates = vec![
            patina::paths::patina_home()
                .join("children")
                .join(format!("patina_ai_child_{stem}.wasm")),
            patina::paths::patina_home()
                .join("pando-adapters")
                .join(format!("{name}.wasm")),
            patina::paths::patina_home()
                .join("pando-adapters")
                .join(format!("patina_ai_adapter_{stem}.wasm")),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/wasm32-wasip2/debug")
                .join(format!("patina_ai_child_{stem}.wasm")),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/wasm32-wasip2/debug")
                .join(format!("patina_ai_adapter_{stem}.wasm")),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/wasm32-wasip2/release")
                .join(format!("patina_ai_child_{stem}.wasm")),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/wasm32-wasip2/release")
                .join(format!("patina_ai_adapter_{stem}.wasm")),
        ];

        candidates.dedup();
        for candidate in candidates {
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        anyhow::bail!("component wasm not found for '{}'", name)
    }

    pub(super) fn compose_typed_component(
        &self,
        manifest: &mother_crate::pando::PandoManifest,
    ) -> Result<Vec<u8>> {
        let Some(composition) = &manifest.composition else {
            anyhow::bail!("typed composition requested without [composition]");
        };

        let typed_rules = composition
            .wiring
            .iter()
            .filter_map(|rule| match rule {
                mother_crate::pando::PandoWiring::Typed(typed) => Some(typed),
                mother_crate::pando::PandoWiring::Legacy(_) => None,
            })
            .collect::<Vec<_>>();
        if typed_rules.is_empty() {
            anyhow::bail!("typed composition requested without typed wiring rules");
        }

        let mut graph = CompositionGraph::new();
        let mut instance_ids = HashMap::new();

        for child in &manifest.children {
            let instance_name = child.id.as_deref().unwrap_or(&child.name);
            let wasm_path = self.component_wasm_path(&child.name)?;

            let package_name = format!(
                "patina:{}:{}",
                manifest.pando.name,
                child.id.as_deref().unwrap_or(&child.name)
            );
            let package = Package::from_file(&package_name, None, &wasm_path, graph.types_mut())
                .map_err(|e| anyhow::anyhow!("loading typed child '{}': {}", child.name, e))?;
            let package_id = graph
                .register_package(package)
                .map_err(|e| anyhow::anyhow!("registering typed child '{}': {}", child.name, e))?;
            let instance_id = graph.instantiate(package_id);
            instance_ids.insert(instance_name.to_string(), instance_id);
        }

        for rule in typed_rules {
            let policy = rule.delivery_policy();
            let policy_label = match policy {
                mother_crate::pando::PandoDeliveryPolicy::Required => "required",
                mother_crate::pando::PandoDeliveryPolicy::BestEffort => "best-effort",
                mother_crate::pando::PandoDeliveryPolicy::DeadLetter => "dead-letter",
            };

            let Some(from_id) = instance_ids.get(&rule.from) else {
                audit::emit_typed_wiring_audit(
                    &rule.from,
                    &rule.to,
                    &rule.toy,
                    "DENY",
                    &format!("unknown from instance (policy={})", policy_label),
                );
                if matches!(policy, mother_crate::pando::PandoDeliveryPolicy::Required) {
                    anyhow::bail!(
                        "typed composition wiring references unknown from instance '{}'",
                        rule.from
                    );
                }
                continue;
            };
            let from_id = *from_id;
            let mut try_wire = |target_id, toy: &str| -> Option<String> {
                let candidates = Self::typed_interface_candidates(toy);
                for candidate in &candidates {
                    let export_id = match graph.alias_instance_export(from_id, candidate) {
                        Ok(id) => id,
                        Err(_) => continue,
                    };
                    if graph
                        .set_instantiation_argument(target_id, candidate, export_id)
                        .is_ok()
                    {
                        return Some((*candidate).to_string());
                    }
                }
                None
            };

            let Some(to_id) = instance_ids.get(&rule.to).copied() else {
                audit::emit_typed_wiring_audit(
                    &rule.from,
                    &rule.to,
                    &rule.toy,
                    "DENY",
                    &format!("unknown to instance (policy={})", policy_label),
                );
                match policy {
                    mother_crate::pando::PandoDeliveryPolicy::Required => {
                        anyhow::bail!(
                            "typed composition wiring references unknown to instance '{}'",
                            rule.to
                        );
                    }
                    mother_crate::pando::PandoDeliveryPolicy::BestEffort => continue,
                    mother_crate::pando::PandoDeliveryPolicy::DeadLetter => {
                        if let Some(dead_letter) = composition.dead_letter.as_ref() {
                            if let Some(dead_letter_id) =
                                instance_ids.get(&dead_letter.child).copied()
                            {
                                let dead_letter_toy =
                                    dead_letter.toy.as_deref().unwrap_or(&rule.toy);
                                let mut rerouted_interface: Option<String> = None;
                                for candidate in &Self::typed_interface_candidates(dead_letter_toy)
                                {
                                    let export_id =
                                        match graph.alias_instance_export(from_id, candidate) {
                                            Ok(id) => id,
                                            Err(_) => continue,
                                        };
                                    if graph
                                        .set_instantiation_argument(
                                            dead_letter_id,
                                            candidate,
                                            export_id,
                                        )
                                        .is_ok()
                                    {
                                        rerouted_interface = Some((*candidate).to_string());
                                        break;
                                    }
                                }
                                if let Some(interface) = rerouted_interface {
                                    audit::emit_typed_wiring_audit(
                                        &rule.from,
                                        &dead_letter.child,
                                        dead_letter_toy,
                                        "GRANT",
                                        &format!(
                                            "dead-letter reroute succeeded via '{}' after unknown to instance",
                                            interface
                                        ),
                                    );
                                } else {
                                    audit::emit_typed_wiring_audit(
                                        &rule.from,
                                        &dead_letter.child,
                                        dead_letter_toy,
                                        "DENY",
                                        "dead-letter reroute failed after unknown to instance",
                                    );
                                }
                            } else {
                                audit::emit_typed_wiring_audit(
                                    &rule.from,
                                    &dead_letter.child,
                                    &rule.toy,
                                    "DENY",
                                    "dead-letter policy requested but dead-letter child instance is unknown",
                                );
                            }
                        } else {
                            audit::emit_typed_wiring_audit(
                                &rule.from,
                                &rule.to,
                                &rule.toy,
                                "DENY",
                                "dead-letter policy requested but [composition.dead-letter] is missing",
                            );
                        }
                        continue;
                    }
                }
            };

            let matched_interface = try_wire(to_id, &rule.toy);
            if matched_interface.is_none() {
                let from_child = Self::resolve_child_instance(manifest, &rule.from)
                    .map(|child| child.name.clone())
                    .unwrap_or_else(|| rule.from.clone());
                let to_child = Self::resolve_child_instance(manifest, &rule.to)
                    .map(|child| child.name.clone())
                    .unwrap_or_else(|| rule.to.clone());
                let reason = format!(
                    "no compatible export/import interface found (policy={})",
                    policy_label
                );
                audit::emit_typed_wiring_audit(&rule.from, &rule.to, &rule.toy, "DENY", &reason);

                match policy {
                    mother_crate::pando::PandoDeliveryPolicy::Required => {
                        anyhow::bail!(
                            "typed wiring failed for '{}' -> '{}' on '{}' (from child '{}', to child '{}')",
                            rule.from,
                            rule.to,
                            rule.toy,
                            from_child,
                            to_child
                        );
                    }
                    mother_crate::pando::PandoDeliveryPolicy::BestEffort => continue,
                    mother_crate::pando::PandoDeliveryPolicy::DeadLetter => {
                        if let Some(dead_letter) = composition.dead_letter.as_ref() {
                            if let Some(dead_letter_id) =
                                instance_ids.get(&dead_letter.child).copied()
                            {
                                let dead_letter_toy =
                                    dead_letter.toy.as_deref().unwrap_or(&rule.toy);
                                let mut rerouted_interface: Option<String> = None;
                                for candidate in &Self::typed_interface_candidates(dead_letter_toy)
                                {
                                    let export_id =
                                        match graph.alias_instance_export(from_id, candidate) {
                                            Ok(id) => id,
                                            Err(_) => continue,
                                        };
                                    if graph
                                        .set_instantiation_argument(
                                            dead_letter_id,
                                            candidate,
                                            export_id,
                                        )
                                        .is_ok()
                                    {
                                        rerouted_interface = Some((*candidate).to_string());
                                        break;
                                    }
                                }
                                if let Some(interface) = rerouted_interface {
                                    audit::emit_typed_wiring_audit(
                                        &rule.from,
                                        &dead_letter.child,
                                        dead_letter_toy,
                                        "GRANT",
                                        &format!(
                                            "dead-letter reroute succeeded via '{}' after primary route failure",
                                            interface
                                        ),
                                    );
                                } else {
                                    audit::emit_typed_wiring_audit(
                                        &rule.from,
                                        &dead_letter.child,
                                        dead_letter_toy,
                                        "DENY",
                                        "dead-letter reroute failed after primary route failure",
                                    );
                                }
                            } else {
                                audit::emit_typed_wiring_audit(
                                    &rule.from,
                                    &dead_letter.child,
                                    &rule.toy,
                                    "DENY",
                                    "dead-letter policy requested but dead-letter child instance is unknown",
                                );
                            }
                        } else {
                            audit::emit_typed_wiring_audit(
                                &rule.from,
                                &rule.to,
                                &rule.toy,
                                "DENY",
                                "dead-letter policy requested but [composition.dead-letter] is missing",
                            );
                        }
                        continue;
                    }
                }
            }

            let grant_reason = matched_interface
                .map(|interface| {
                    format!(
                        "wired via interface '{}' (policy={})",
                        interface, policy_label
                    )
                })
                .unwrap_or_else(|| format!("wired (policy={})", policy_label));
            audit::emit_typed_wiring_audit(&rule.from, &rule.to, &rule.toy, "GRANT", &grant_reason);
        }

        if let Some(entry) = &composition.entry {
            let Some(entry_id) = instance_ids.get(&entry.child) else {
                audit::emit_typed_wiring_audit(
                    &entry.child,
                    "__entry__",
                    &entry.toy,
                    "DENY",
                    "entry references unknown child instance",
                );
                anyhow::bail!(
                    "typed composition entry references unknown child instance '{}'",
                    entry.child
                );
            };
            let candidates = Self::typed_interface_candidates(&entry.toy);
            let mut exported = false;
            for candidate in &candidates {
                let export_id = match graph.alias_instance_export(*entry_id, candidate) {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                if graph.export(export_id, candidate).is_ok() {
                    exported = true;
                    audit::emit_typed_wiring_audit(
                        &entry.child,
                        "__entry__",
                        &entry.toy,
                        "GRANT",
                        &format!("entry export via interface '{}'", candidate),
                    );
                    break;
                }
            }
            if !exported {
                audit::emit_typed_wiring_audit(
                    &entry.child,
                    "__entry__",
                    &entry.toy,
                    "DENY",
                    "entry export interface not found",
                );
                anyhow::bail!(
                    "typed composition entry export failed for instance '{}' toy '{}'",
                    entry.child,
                    entry.toy
                );
            }
        }

        graph.encode(EncodeOptions::default()).map_err(|e| {
            anyhow::anyhow!(
                "encoding typed composition '{}': {}",
                manifest.pando.name,
                e
            )
        })
    }

    pub(super) fn execute_typed_composition(
        &self,
        manifest: &mother_crate::pando::PandoManifest,
    ) -> Result<()> {
        let (engine, component) = self.load_typed_component(manifest)?;

        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        composed_bindings::CatalogRunner::add_to_linker::<
            composed_bindings::HostState,
            wasmtime::component::HasSelf<composed_bindings::HostState>,
        >(&mut linker, |state| state)?;

        let source_host = std::env::var("PATINA_PANDO_FOLDER").unwrap_or_else(|_| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .to_string_lossy()
                .to_string()
        });
        let output_host = std::env::var("PATINA_PANDO_OUTPUT").unwrap_or_else(|_| {
            patina::paths::patina_home()
                .join("pandos")
                .join(&manifest.pando.name)
                .join("output")
                .to_string_lossy()
                .to_string()
        });
        std::fs::create_dir_all(&output_host)?;

        let mut wasi = wasmtime_wasi::WasiCtxBuilder::new();
        wasi.preopened_dir(
            Path::new(&source_host),
            "/input",
            wasmtime_wasi::DirPerms::READ,
            wasmtime_wasi::FilePerms::READ,
        )?;
        wasi.preopened_dir(
            Path::new(&output_host),
            "/output",
            wasmtime_wasi::DirPerms::READ | wasmtime_wasi::DirPerms::MUTATE,
            wasmtime_wasi::FilePerms::READ | wasmtime_wasi::FilePerms::WRITE,
        )?;

        let mut pando_config = HashMap::new();
        pando_config.insert("folder_path".to_string(), "/input".to_string());

        let mut store = Store::new(
            &engine,
            composed_bindings::HostState {
                wasi: wasi.build(),
                wasi_table: ResourceTable::new(),
                runtime: self.runtime_store.clone(),
                config: pando_config,
            },
        );

        let instance =
            composed_bindings::CatalogRunner::instantiate(&mut store, &component, &linker)?;
        let run_result = instance.patina_pando_catalog().call_run(&mut store)?;
        run_result
            .map(|_| ())
            .map_err(|error| anyhow::anyhow!("typed pando execution failed: {}", error))
    }

    pub(super) fn load_typed_component(
        &self,
        manifest: &mother_crate::pando::PandoManifest,
    ) -> Result<(wasmtime::Engine, Component)> {
        let composed_bytes = self.compose_typed_component(manifest)?;
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config)?;
        let component = Component::new(&engine, &composed_bytes)?;
        Ok((engine, component))
    }
}
