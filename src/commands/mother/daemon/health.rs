use super::*;

impl ServerState {
    pub(super) fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    pub(super) fn set_child_warmup_state(&self, state: &str, last_error: Option<String>) {
        let mut guard = self
            .child_warmup_state
            .write()
            .unwrap_or_else(|e| e.into_inner());
        guard.state = state.to_string();
        guard.last_error = last_error;
    }

    pub(super) fn current_memory_status(&self) -> mother_crate::http_api::MemoryStatus {
        let max_rss_bytes = process_max_rss_bytes();
        let soft_limit_bytes = self.memory_soft_limit_bytes;
        let usage = max_rss_bytes;

        let pressure = match (usage, soft_limit_bytes) {
            (Some(used), Some(limit)) if used > limit => "high".to_string(),
            (Some(_), _) => "ok".to_string(),
            (None, Some(_)) => "unknown".to_string(),
            (None, None) => "ok".to_string(),
        };

        mother_crate::http_api::MemoryStatus {
            rss_bytes: None,
            max_rss_bytes,
            soft_limit_bytes,
            pressure,
        }
    }

    pub(super) fn ensure_memory_allows_warmup(&self) -> Result<()> {
        let memory = self.current_memory_status();
        ensure_memory_status_allows_warmup(&memory)
    }

    pub(super) fn installed_children(&self) -> HashSet<String> {
        let children_dir = patina::paths::child::children_dir();
        installed_child_names_from_dir(&children_dir)
    }

    pub(super) fn live_children(&self) -> HashSet<String> {
        self.registry
            .health_all()
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    pub(super) fn reload_pando_registry(&self) -> Result<()> {
        let native = self
            .native_commands
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let installed_children = self.installed_children();
        let live_children = self.live_children();
        let registry = mother_crate::pando::build_registry(
            &self.pandos_root,
            &native,
            &self.aliases,
            &installed_children,
            &live_children,
        )?;
        *self
            .pando_registry
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = registry;
        Ok(())
    }

    pub(super) fn current_pando_state(&self) -> patina_protocol::PandoRegistryState {
        let registry = self
            .pando_registry
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        patina_protocol::PandoRegistryState {
            protocol_version: patina_protocol::PANDO_REGISTRY_PROTOCOL_VERSION,
            pandos: registry
                .pandos
                .into_iter()
                .map(|entry| patina_protocol::PandoStateEntry {
                    name: entry.name,
                    status: match entry.status {
                        mother_crate::pando::PandoLifecycleStatus::Registered => {
                            patina_protocol::PandoStatus::Registered
                        }
                        mother_crate::pando::PandoLifecycleStatus::Ready => {
                            patina_protocol::PandoStatus::Ready
                        }
                        mother_crate::pando::PandoLifecycleStatus::Live => {
                            patina_protocol::PandoStatus::Live
                        }
                        mother_crate::pando::PandoLifecycleStatus::Degraded => {
                            patina_protocol::PandoStatus::Degraded
                        }
                        mother_crate::pando::PandoLifecycleStatus::Error => {
                            patina_protocol::PandoStatus::Error
                        }
                    },
                    commands: entry.commands,
                    aliases: entry.aliases,
                    child_count: entry.child_count,
                })
                .collect(),
        }
    }
}

pub(super) fn ensure_memory_status_allows_warmup(
    memory: &mother_crate::http_api::MemoryStatus,
) -> Result<()> {
    if memory.pressure == "high" {
        let used = memory.max_rss_bytes.or(memory.rss_bytes).unwrap_or(0);
        let limit = memory.soft_limit_bytes.unwrap_or(0);
        return Err(anyhow::Error::new(
            mother_crate::http_api::LifecycleError::resource_exhausted(format!(
                "memory pressure high (usage={} limit={}); warmup denied",
                used, limit
            )),
        ));
    }
    Ok(())
}

pub(super) fn read_current_project_uid() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let uid = std::fs::read_to_string(patina::paths::project::uid_path(&cwd)).ok()?;
    let uid = uid.trim();

    let is_legacy = uid.len() == 8
        && uid
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
    let is_modern = uid.len() == 37
        && uid.starts_with("puid_")
        && uid[5..]
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());

    if is_legacy || is_modern {
        Some(uid.to_string())
    } else {
        None
    }
}

pub(super) fn file_size_if_exists(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}

pub(super) fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u64>().ok())
}

pub(super) fn resolve_memory_soft_limit_bytes() -> Option<u64> {
    env_u64("PATINA_MOTHER_MEMORY_SOFT_LIMIT_BYTES").or_else(|| {
        env_u64("PATINA_MOTHER_MEMORY_SOFT_LIMIT_MB").map(|mb| mb.saturating_mul(1024 * 1024))
    })
}

pub(super) fn process_max_rss_bytes() -> Option<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let result = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if result != 0 {
        return None;
    }
    let usage = unsafe { usage.assume_init() };
    #[cfg(target_os = "linux")]
    let bytes = (usage.ru_maxrss as i128).saturating_mul(1024) as u64;
    #[cfg(not(target_os = "linux"))]
    let bytes = usage.ru_maxrss as u64;

    if bytes == 0 {
        None
    } else {
        Some(bytes)
    }
}

pub(super) fn installed_child_names_from_dir(children_dir: &Path) -> HashSet<String> {
    if !children_dir.exists() {
        return HashSet::new();
    }

    let mut installed = HashSet::new();
    if let Ok(entries) = std::fs::read_dir(children_dir) {
        for entry in entries.flatten() {
            let wasm_path = entry.path();
            if wasm_path.extension().and_then(|ext| ext.to_str()) != Some("wasm") {
                continue;
            }
            let manifest_path = wasm_path.with_extension("toml");
            if !manifest_path.exists() {
                continue;
            }

            match patina::child::engine::ChildManifest::from_path(&manifest_path) {
                Ok(manifest) => {
                    installed.insert(manifest.name);
                }
                Err(error) => {
                    tracing::warn!(
                        manifest_path = %manifest_path.display(),
                        %error,
                        "failed to parse child manifest for installed-child identity"
                    );
                }
            }
        }
    }

    installed
}
