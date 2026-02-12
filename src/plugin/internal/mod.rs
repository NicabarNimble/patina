//! wasmtime guts — bindgen, Engine singleton, WasmChild adapter.

use std::path::Path;
use std::sync::OnceLock;

use anyhow::Result;
use wasmtime::component::Component;
use wasmtime::{Config, Engine};

mod command;
mod mother_child;

#[cfg(test)]
mod tests;

pub use command::CommandEngine;
pub use mother_child::PluginEngine;

// =========================================================================
// Engine singleton (OnceLock pattern from Zed)
// =========================================================================

/// Shared wasmtime engine — singleton per process.
pub(super) fn wasm_engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.wasm_component_model(true);
        // NO config.async_support(true) — sync-first
        Engine::new(&config).expect("failed to create wasmtime engine")
    })
}

// =========================================================================
// Plugin manifest (plugin.toml)
// =========================================================================

/// Parsed plugin manifest from plugin.toml.
#[derive(Debug)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub world: String,
    pub patina_min: String,
    pub capabilities: Vec<String>,
    /// Toy commands this plugin is allowed to request (from [capabilities.toys].commands).
    /// Empty means no toys allowed.
    pub allowed_toy_commands: Vec<String>,
    pub provides: PluginProvides,
}

/// What the plugin provides to the system.
#[derive(Debug)]
pub struct PluginProvides {
    pub child: Option<String>,
    pub commands: Vec<String>,
}

impl PluginManifest {
    /// Parse a plugin manifest from a TOML file.
    pub(super) fn from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let table: toml::Table = content.parse()?;

        let plugin = table
            .get("plugin")
            .and_then(|v| v.as_table())
            .ok_or_else(|| anyhow::anyhow!("missing [plugin] section"))?;

        let name = plugin
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing plugin.name"))?
            .to_string();

        let version = plugin
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        let description = plugin
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let world = plugin
            .get("world")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing plugin.world"))?
            .to_string();

        let patina_min = plugin
            .get("patina_min")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        // Parse capabilities
        let cap_table = table.get("capabilities").and_then(|v| v.as_table());
        let capabilities = cap_table
            .map(|cap| {
                cap.iter()
                    .filter(|(_, v)| v.as_bool() == Some(true))
                    .map(|(k, _)| k.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Parse [capabilities.toys].commands — allowed toy commands
        let allowed_toy_commands = cap_table
            .and_then(|cap| cap.get("toys"))
            .and_then(|v| v.as_table())
            .and_then(|toys| toys.get("commands"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Parse provides
        let provides_table = table.get("provides").and_then(|v| v.as_table());
        let child = provides_table
            .and_then(|p| p.get("child"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let commands = provides_table
            .and_then(|p| p.get("commands"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            name,
            version,
            description,
            world,
            patina_min,
            capabilities,
            allowed_toy_commands,
            provides: PluginProvides { child, commands },
        })
    }

    /// Load a WASM component from bytes using the shared engine.
    pub(super) fn load_component(wasm: &[u8]) -> Result<Component> {
        Component::new(wasm_engine(), wasm)
    }
}
