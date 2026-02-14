//! Mother-child world — bindgen, PluginEngine, WasmChild adapter.

use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::Store;

use super::{wasm_engine, GrantedCapabilities, PluginManifest};
use crate::mother::{ChildHealth, ChildRequest, ChildResponse, MotherHost, Toy};

// =========================================================================
// URL validation — data-level sanitization per [[sanitize-at-data-level]]
// =========================================================================

/// Validate and parse an HTTP URL for domain-allowlisted access.
///
/// Returns the extracted domain on success. Enforces:
/// - HTTPS only (no plaintext HTTP)
/// - No IP addresses (IPv4 or IPv6)
/// - No localhost
///
/// Pure function — testable independently of wasmtime.
pub(super) fn validate_http_url(url: &str) -> Result<String, String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("invalid URL: {}", e))?;

    // HTTPS only
    if parsed.scheme() != "https" {
        return Err(format!("only HTTPS allowed, got '{}'", parsed.scheme()));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| "no host in URL".to_string())?;

    // No localhost
    if host == "localhost" {
        return Err("localhost not allowed".to_string());
    }

    // No IP addresses (IPv4 or IPv6)
    // host_str() returns brackets for IPv6 (e.g., "[::1]") — strip them
    let bare_host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    if bare_host.parse::<std::net::IpAddr>().is_ok() {
        return Err("IP addresses not allowed".to_string());
    }

    Ok(bare_host.to_string())
}

// =========================================================================
// Bindgen — generates types from WIT definitions
// =========================================================================

/// Generated types + HostState live together so bindgen's HasData/Host
/// trait resolution works correctly. The generated MotherChild type
/// stays internal — WasmChild bridges to our crate::mother::MotherChild trait.
mod bindings {
    /// State passed to WASM plugins via Store<HostState>.
    /// Contains WASI context (wasm32-wasip2 components always import basic WASI),
    /// plugin name for log prefix, and HTTP client for domain-allowlisted access.
    pub struct HostState {
        pub plugin_name: String,
        pub wasi: wasmtime_wasi::WasiCtx,
        pub wasi_table: wasmtime::component::ResourceTable,
        /// Resolved capabilities for call-time gating.
        pub grants: super::GrantedCapabilities,
        /// Pre-configured HTTP client with cross-domain redirect rejection.
        pub http_client: reqwest::blocking::Client,
    }

    // WasiView is required for wasmtime-wasi to satisfy WASI imports
    impl wasmtime_wasi::WasiView for HostState {
        fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
            wasmtime_wasi::WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.wasi_table,
            }
        }
    }

    wasmtime::component::bindgen!({
        path: "wit/mother-child/",
        world: "mother-child",
    });

    // Implement the generated Host trait for patina:host/log
    impl patina::host::log::Host for HostState {
        fn log(&mut self, level: patina::host::log::LogLevel, message: String) {
            let level_str = match level {
                patina::host::log::LogLevel::Debug => "DEBUG",
                patina::host::log::LogLevel::Info => "INFO",
                patina::host::log::LogLevel::Warn => "WARN",
                patina::host::log::LogLevel::Error => "ERROR",
            };
            eprintln!("[plugin:{}] {}: {}", self.plugin_name, level_str, message);
        }
    }

    // patina:host/types only defines types (no functions) — empty Host trait
    impl patina::host::types::Host for HostState {}

    // patina:host/http — domain-allowlisted HTTP access.
    //
    // Defense in depth: domains are validated at load time (check_capabilities)
    // AND at call time (grants.http_domains check below). URLs are sanitized
    // by validate_http_url (HTTPS-only, no IPs, no localhost). Cross-domain
    // redirects are rejected by the reqwest client's redirect policy.
    impl patina::host::http::Host for HostState {
        fn http_post(
            &mut self,
            url: String,
            body: String,
            content_type: String,
        ) -> Result<patina::host::http::HttpResponse, String> {
            let domain = super::validate_http_url(&url)?;
            if !self.grants.http_domains.contains(&domain) {
                return Err(format!(
                    "domain '{}' not in allowlist for plugin '{}'",
                    domain, self.plugin_name
                ));
            }
            let response = self
                .http_client
                .post(&url)
                .header("Content-Type", &content_type)
                .body(body)
                .send()
                .map_err(|e| format!("HTTP POST failed: {}", e))?;
            let status = response.status().as_u16();
            let resp_body = response.text().map_err(|e| format!("read body: {}", e))?;
            Ok(patina::host::http::HttpResponse {
                status,
                body: resp_body,
            })
        }

        fn http_get(&mut self, url: String) -> Result<patina::host::http::HttpResponse, String> {
            let domain = super::validate_http_url(&url)?;
            if !self.grants.http_domains.contains(&domain) {
                return Err(format!(
                    "domain '{}' not in allowlist for plugin '{}'",
                    domain, self.plugin_name
                ));
            }
            let response = self
                .http_client
                .get(&url)
                .send()
                .map_err(|e| format!("HTTP GET failed: {}", e))?;
            let status = response.status().as_u16();
            let resp_body = response.text().map_err(|e| format!("read body: {}", e))?;
            Ok(patina::host::http::HttpResponse {
                status,
                body: resp_body,
            })
        }
    }
}

use bindings::HostState;

// =========================================================================
// PluginEngine
// =========================================================================

/// Shared wasmtime infrastructure for loading and running WASM plugins.
pub struct PluginEngine {
    linker: Linker<HostState>,
}

impl PluginEngine {
    /// Create a new PluginEngine with host functions registered.
    ///
    /// Create once per process and reuse for all plugin loading. The
    /// underlying wasmtime::Engine is a process-wide singleton (OnceLock),
    /// but Linker setup (WASI + host functions) runs on each call.
    /// In daemon mode, daemon.rs creates one PluginEngine and passes it
    /// to load_wasm_child(). CLI command plugins (Phase 2) will need to
    /// decide whether to share the daemon's engine or create a fresh one.
    pub fn new() -> Result<Self> {
        let mut linker = Linker::new(wasm_engine());

        // Add WASI to linker — wasm32-wasip2 components always import basic WASI
        // (stdio, env, clocks) even for pure-computation code.
        // Using sync linker — no async runtime.
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;

        // Add our custom host functions (patina:host/log, etc.)
        bindings::MotherChild::add_to_linker::<HostState, wasmtime::component::HasSelf<HostState>>(
            &mut linker,
            |s| s,
        )?;
        Ok(Self { linker })
    }

    /// Load and parse a plugin manifest from plugin.toml.
    pub fn load_manifest(path: &Path) -> Result<PluginManifest> {
        PluginManifest::from_path(path)
    }

    /// Load a WASM component from bytes.
    pub fn load_component(&self, wasm: &[u8]) -> Result<Component> {
        PluginManifest::load_component(wasm)
    }

    /// Check that a plugin's requested capabilities are granted.
    ///
    /// Phase 1: host_log, host_layer are always granted. All others denied.
    /// Phase 2: host_query validated — kinds must be known.
    /// Future: reads from ~/.patina/plugin-config/grants.toml.
    pub fn check_capabilities(manifest: &PluginManifest) -> Result<()> {
        // Boolean capabilities that are always granted (no config needed)
        let auto_granted = ["host_log", "host_layer"];

        let denied: Vec<&str> = manifest
            .capabilities
            .iter()
            .filter(|cap| !auto_granted.contains(&cap.as_str()))
            .map(|s| s.as_str())
            .collect();

        if !denied.is_empty() {
            anyhow::bail!(
                "plugin '{}' requests capabilities not granted: {}",
                manifest.name,
                denied.join(", ")
            );
        }

        // Load-time validation: host_query kinds must be known
        const KNOWN_QUERY_KINDS: &[&str] = &["scry", "context", "assay"];
        let unknown: Vec<&str> = manifest
            .host_query_kinds
            .iter()
            .filter(|k| !KNOWN_QUERY_KINDS.contains(&k.as_str()))
            .map(|s| s.as_str())
            .collect();

        if !unknown.is_empty() {
            anyhow::bail!(
                "plugin '{}' requests unknown query kinds: {}",
                manifest.name,
                unknown.join(", ")
            );
        }

        // Load-time validation: host_http domains must be valid
        for domain in &manifest.host_http_domains {
            if domain.is_empty() {
                anyhow::bail!(
                    "plugin '{}' has empty HTTP domain in host_http",
                    manifest.name
                );
            }
            if !domain.is_ascii() {
                anyhow::bail!(
                    "plugin '{}' has non-ASCII HTTP domain '{}' in host_http",
                    manifest.name,
                    domain
                );
            }
            if domain.contains('/') {
                anyhow::bail!(
                    "plugin '{}' has path component in HTTP domain '{}' in host_http",
                    manifest.name,
                    domain
                );
            }
        }

        Ok(())
    }

    /// Instantiate a MotherChild from a WASM component + manifest.
    /// Returns Box<dyn MotherChild> for ChildRegistry compatibility.
    pub fn instantiate_child(
        &self,
        component: &Component,
        manifest: &PluginManifest,
    ) -> Result<Box<dyn crate::mother::MotherChild>> {
        // Check capabilities before instantiation
        Self::check_capabilities(manifest)?;

        // Build resolved capabilities for call-time gating
        let grants = manifest.granted_capabilities();

        // Build HTTP client with cross-domain redirect rejection.
        // Per spec: if a response redirects to a different domain, reject it
        // (prevents allowlist bypass via open redirectors).
        let http_client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.url().host_str() != attempt.previous().last().and_then(|u| u.host_str())
                {
                    attempt.stop()
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .map_err(|e| anyhow::anyhow!("build HTTP client: {}", e))?;

        // Minimal WASI context — no filesystem access, no env inheritance.
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().build();
        let host_state = HostState {
            plugin_name: manifest.name.clone(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            grants,
            http_client,
        };
        let mut store = Store::new(wasm_engine(), host_state);

        let instance = bindings::MotherChild::instantiate(&mut store, component, &self.linker)?;

        // Initialize the plugin (must be called before any other export)
        instance.call_init(&mut store)?;

        // Get the child name from the WASM module
        let name = instance.call_name(&mut store)?;

        Ok(Box::new(WasmChild {
            name,
            allowed_toy_commands: manifest.allowed_toy_commands.clone(),
            inner: Mutex::new(WasmChildInner { store, instance }),
        }))
    }
}

// =========================================================================
// WasmChild adapter — wraps WASM instance as native MotherChild
// =========================================================================

/// Adapter: wraps a WASM component instance as a MotherChild.
///
/// Both store and instance live behind a single Mutex. This is the WASM
/// isolation boundary — no `unsafe` needed. Mutex<T> is Sync when T is Send,
/// and WasmChildInner is Send because both Store<HostState> and
/// bindings::MotherChild are Send. We already acquire the lock on every
/// call, so there's zero performance cost vs the previous layout.
struct WasmChild {
    name: String,
    allowed_toy_commands: Vec<String>,
    inner: Mutex<WasmChildInner>,
}

/// Interior state behind the Mutex — store and instance together.
struct WasmChildInner {
    store: Store<HostState>,
    instance: bindings::MotherChild,
}

impl crate::mother::MotherChild for WasmChild {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
        // Host capabilities come through WASM imports (patina:host/log),
        // not the Rust MotherHost reference.
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmChildInner { store, instance } = &mut *inner;
        match instance.call_on_load(store)? {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("WASM on_load failed: {}", e)),
        }
    }

    fn on_unload(&mut self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmChildInner { store, instance } = &mut *inner;
        let _ = instance.call_on_unload(store);
    }

    fn health(&self) -> ChildHealth {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmChildInner { store, instance } = &mut *inner;
        match instance.call_health(store) {
            Ok(h) => {
                let reason = h.reason.unwrap_or_default();
                match h.status {
                    bindings::patina::host::types::HealthStatus::Healthy => ChildHealth::Healthy,
                    bindings::patina::host::types::HealthStatus::Degraded => {
                        ChildHealth::Degraded(if reason.is_empty() {
                            "degraded".into()
                        } else {
                            reason
                        })
                    }
                    bindings::patina::host::types::HealthStatus::Unhealthy => {
                        ChildHealth::Unhealthy(if reason.is_empty() {
                            "unhealthy".into()
                        } else {
                            reason
                        })
                    }
                }
            }
            Err(e) => ChildHealth::Unhealthy(format!("WASM call failed: {}", e)),
        }
    }

    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmChildInner { store, instance } = &mut *inner;
        let payload_json = serde_json::to_string(&request.payload)?;
        let result = instance.call_handle(store, &request.action, &payload_json)?;
        match result {
            Ok(json) => Ok(ChildResponse {
                payload: serde_json::from_str(&json)?,
            }),
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }

    fn tick(&mut self) -> Vec<Toy> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmChildInner { store, instance } = &mut *inner;
        match instance.call_tick(store) {
            Ok(wasm_toys) => wasm_toys
                .into_iter()
                .filter_map(|t| {
                    let toy = Toy {
                        name: t.name,
                        command: t.command,
                        args: t.args,
                    };
                    if self.allowed_toy_commands.contains(&toy.command) {
                        Some(toy)
                    } else {
                        eprintln!(
                            "[plugin:{}] toy '{}' denied: command '{}' not in allowed list {:?}",
                            self.name, toy.name, toy.command, self.allowed_toy_commands
                        );
                        None
                    }
                })
                .collect(),
            Err(e) => {
                eprintln!("[plugin:{}] tick failed: {}", self.name, e);
                vec![]
            }
        }
    }
}
