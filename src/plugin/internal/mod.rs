//! wasmtime guts — bindgen, Engine singleton, WasmChild adapter.

use std::path::Path;
use std::sync::OnceLock;

use anyhow::Result;
use wasmtime::component::Component;
use wasmtime::{Config, Engine};

mod command;
mod host_support;
mod mother_child;
mod pipeline;
mod task;

#[cfg(test)]
mod tests;

pub use command::{CommandEngine, QueryDispatchFn};
pub use mother_child::PluginEngine;
pub use pipeline::PipelineEngine;
pub use task::TaskEngine;

// =========================================================================
// Plugin world enum — parsed from manifest, enforced at load time (F4)
// =========================================================================

/// Known plugin worlds — parsed from manifest, enforced at load time.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginWorld {
    MotherChild,
    Command,
    Task,
    Pipeline,
}

impl std::str::FromStr for PluginWorld {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "mother-child" => Ok(Self::MotherChild),
            "command" => Ok(Self::Command),
            "task" => Ok(Self::Task),
            "pipeline" => Ok(Self::Pipeline),
            other => anyhow::bail!("unknown plugin world: '{}'", other),
        }
    }
}

impl PluginWorld {
    /// Capabilities this world is allowed to declare.
    pub fn allowed_capabilities(&self) -> &[&str] {
        match self {
            Self::MotherChild => &[
                "host_log",
                "host_layer",
                "host_query",
                "host_http",
                "host_measure",
            ],
            Self::Command => &["host_log", "host_layer", "host_query", "host_measure"],
            Self::Task => &[
                "host_log",
                "host_layer",
                "host_query",
                "host_http",
                "host_measure",
            ],
            Self::Pipeline => &["host_log"],
        }
    }
}

impl std::fmt::Display for PluginWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MotherChild => write!(f, "mother-child"),
            Self::Command => write!(f, "command"),
            Self::Task => write!(f, "task"),
            Self::Pipeline => write!(f, "pipeline"),
        }
    }
}

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
// Credential injection types
// =========================================================================

/// How to inject a credential into an HTTP request.
#[derive(Debug, Clone)]
pub enum InjectionLocation {
    /// Authorization: Bearer {value}
    Bearer,
}

/// Maps a domain to a vault secret and injection method.
#[derive(Debug, Clone)]
pub struct CredentialMapping {
    pub secret_name: String,
    pub location: InjectionLocation,
}

// =========================================================================
// Plugin manifest (plugin.toml)
// =========================================================================

/// Parsed plugin manifest from plugin.toml.
#[derive(Debug, Clone)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub world: PluginWorld,
    pub patina_min: String,
    pub capabilities: Vec<String>,
    /// Toy commands this plugin is allowed to request (from [capabilities.toys].commands).
    /// Empty means no toys allowed.
    pub allowed_toy_commands: Vec<String>,
    /// Query kinds this plugin is allowed to call (from [capabilities].host_query).
    /// E.g., ["scry", "context", "assay"]. Empty means no query access.
    pub host_query_kinds: Vec<String>,
    /// HTTP domains this plugin is allowed to access (from [capabilities].host_http).
    /// E.g., ["api.github.com", "hooks.slack.com"]. Empty means no HTTP access.
    pub host_http_domains: Vec<String>,
    /// Credential mappings per domain (from [capabilities.host_secrets]).
    /// Maps domain → secret name + injection location.
    pub host_secrets: std::collections::HashMap<String, CredentialMapping>,
    pub provides: PluginProvides,
    /// Schema packages this plugin references (from [schemas.<name>].package).
    /// Maps schema name → package string (e.g., "forge" → "patina:schema/forge@1.0.0").
    pub schemas: std::collections::HashMap<String, String>,
}

// =========================================================================
// Granted capabilities — resolved at load time, checked at call time
// =========================================================================

/// Query scope controls cross-project access.
#[derive(Debug, Clone, Default)]
pub enum QueryScope {
    /// Plugin can only query current project (default).
    #[default]
    CurrentProject,
    /// Plugin can query all registered repos.
    AllRepos,
}

/// Resolved capabilities for runtime gating.
///
/// Built from PluginManifest at load time. Stored on host state
/// so call-time checks are a HashSet lookup, not manifest re-parsing.
#[derive(Debug, Clone, Default)]
pub struct GrantedCapabilities {
    pub query_kinds: std::collections::HashSet<String>,
    pub query_scope: QueryScope,
    pub http_domains: std::collections::HashSet<String>,
    /// Credential mappings: domain → secret name + injection location.
    /// Empty means no credential injection.
    pub credential_mappings: std::collections::HashMap<String, CredentialMapping>,
}

/// What the plugin provides to the system.
#[derive(Debug, Default, Clone)]
pub struct PluginProvides {
    pub child: Option<String>,
    pub commands: Vec<String>,
    /// Pipeline operations this plugin handles (e.g., ["parse", "chunk"]).
    pub pipeline_ops: Vec<String>,
    /// Languages (file extensions) this pipeline plugin claims (e.g., ["zig", "nim"]).
    pub languages: Vec<String>,
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

        let world_str = plugin
            .get("world")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing plugin.world"))?;
        let world = world_str.parse::<PluginWorld>()?;

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

        // Parse [capabilities].host_query — allowed query kinds
        let host_query_kinds = cap_table
            .and_then(|cap| cap.get("host_query"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Parse [capabilities].host_http — allowed HTTP domains
        let host_http_domains = cap_table
            .and_then(|cap| cap.get("host_http"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Parse [capabilities.host_secrets] — credential mappings per domain
        let host_secrets = cap_table
            .and_then(|cap| cap.get("host_secrets"))
            .and_then(|v| v.as_table())
            .map(|secrets_table| {
                secrets_table
                    .iter()
                    .filter_map(|(domain, v)| {
                        let t = v.as_table()?;
                        let secret_name =
                            t.get("secret").and_then(|s| s.as_str())?.to_string();
                        let location_str =
                            t.get("location").and_then(|s| s.as_str()).unwrap_or("bearer");
                        let location = match location_str {
                            "bearer" => InjectionLocation::Bearer,
                            other => {
                                eprintln!(
                                    "warning: unknown injection location '{}' for domain '{}', skipping",
                                    other, domain
                                );
                                return None;
                            }
                        };
                        Some((
                            domain.clone(),
                            CredentialMapping {
                                secret_name,
                                location,
                            },
                        ))
                    })
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

        let pipeline_ops = provides_table
            .and_then(|p| p.get("pipeline_ops"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let languages = provides_table
            .and_then(|p| p.get("languages"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Parse [schemas.<name>].package — schema package references
        let schemas = table
            .get("schemas")
            .and_then(|v| v.as_table())
            .map(|schemas_table| {
                schemas_table
                    .iter()
                    .filter_map(|(name, v)| {
                        v.as_table()
                            .and_then(|t| t.get("package"))
                            .and_then(|p| p.as_str())
                            .map(|pkg| (name.clone(), pkg.to_string()))
                    })
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
            host_query_kinds,
            host_http_domains,
            host_secrets,
            provides: PluginProvides {
                child,
                commands,
                pipeline_ops,
                languages,
            },
            schemas,
        })
    }

    /// Load a WASM component from bytes using the shared engine.
    pub(super) fn load_component(wasm: &[u8]) -> Result<Component> {
        Component::new(wasm_engine(), wasm)
    }

    /// Build resolved capabilities from this manifest.
    ///
    /// Called once at load time. The resulting GrantedCapabilities is
    /// stored on CommandHostState for O(1) call-time checks.
    pub fn granted_capabilities(&self) -> GrantedCapabilities {
        let query_kinds = self.host_query_kinds.iter().cloned().collect();
        let http_domains = self.host_http_domains.iter().cloned().collect();
        let credential_mappings = self.host_secrets.clone();

        // Parse query_scope from capabilities table if present.
        // For now, default to CurrentProject — AllRepos requires explicit opt-in.
        let query_scope = QueryScope::CurrentProject;

        GrantedCapabilities {
            query_kinds,
            query_scope,
            http_domains,
            credential_mappings,
        }
    }
}
