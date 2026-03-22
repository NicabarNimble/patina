//! wasmtime guts — bindgen, Engine singleton, WasmChild adapter.

use std::path::Path;
use std::sync::OnceLock;

use anyhow::Result;
use wasmtime::component::Component;
use wasmtime::{Config, Engine};

use crate::mother::GrantedIngressSource;

mod command;
pub(crate) mod host_support;
mod knowledge_child;
mod mother_child;
mod pipeline;
mod task;

#[cfg(test)]
mod tests;

pub use command::{CommandEngine, QueryDispatchFn};
pub use knowledge_child::KnowledgeChildEngine as ChildEngine;
pub use knowledge_child::KnowledgeChildEngine;
pub use mother_child::MotherChildEngine;
pub use pipeline::PipelineEngine;
pub use task::TaskEngine;

/// Legacy alias kept during CV6 migration.
pub type PluginEngine = MotherChildEngine;

// =========================================================================
// Child kind enum — parsed from manifest, enforced at load time (F4)
// =========================================================================

/// Known child kinds — parsed from manifest, enforced at load time.
#[derive(Debug, Clone, PartialEq)]
pub enum ChildKind {
    KnowledgeChild,
    MotherChild,
    Command,
    Task,
    Pipeline,
}

impl std::str::FromStr for ChildKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "knowledge-child" => Ok(Self::KnowledgeChild),
            "mother-child" => Ok(Self::MotherChild),
            "command" => Ok(Self::Command),
            "task" => Ok(Self::Task),
            "pipeline" => Ok(Self::Pipeline),
            other => anyhow::bail!("unknown child kind: '{}'", other),
        }
    }
}

impl ChildKind {
    /// Capabilities this world is allowed to declare.
    pub fn allowed_capabilities(&self) -> &[&str] {
        match self {
            Self::KnowledgeChild => &[
                "host_log",
                "host_query",
                "host_http",
                "host_measure",
                "host_emit",
            ],
            Self::MotherChild => &[
                "host_log",
                "host_layer",
                "host_query",
                "host_http",
                "host_measure",
                "host_emit",
            ],
            Self::Command => &["host_log", "host_layer", "host_query", "host_measure"],
            Self::Task => &[
                "host_log",
                "host_layer",
                "host_query",
                "host_http",
                "host_measure",
                "host_emit",
            ],
            Self::Pipeline => &["host_log"],
        }
    }
}

impl std::fmt::Display for ChildKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KnowledgeChild => write!(f, "knowledge-child"),
            Self::MotherChild => write!(f, "mother-child"),
            Self::Command => write!(f, "command"),
            Self::Task => write!(f, "task"),
            Self::Pipeline => write!(f, "pipeline"),
        }
    }
}

// =========================================================================
// Child role enum — parsed from manifest, describes purpose (F4)
// =========================================================================

/// Known child roles — what the child is FOR (orthogonal to kind).
#[derive(Debug, Clone, PartialEq)]
pub enum ChildRole {
    Connector,
    Grammar,
    Extension,
    App,
}

impl std::str::FromStr for ChildRole {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "connector" => Ok(Self::Connector),
            "grammar" => Ok(Self::Grammar),
            "extension" => Ok(Self::Extension),
            "app" => Ok(Self::App),
            other => anyhow::bail!("unknown child role: '{}'", other),
        }
    }
}

impl ChildRole {
    /// Worlds where this role is typically used.
    /// Used for validation warnings — not enforcement.
    pub fn expected_worlds(&self) -> &[ChildKind] {
        match self {
            Self::Connector => &[
                ChildKind::KnowledgeChild,
                ChildKind::MotherChild,
                ChildKind::Task,
            ],
            Self::Grammar => &[ChildKind::Pipeline],
            Self::Extension => &[ChildKind::Command, ChildKind::Task],
            Self::App => &[
                ChildKind::KnowledgeChild,
                ChildKind::MotherChild,
                ChildKind::Task,
            ],
        }
    }
}

impl std::fmt::Display for ChildRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connector => write!(f, "connector"),
            Self::Grammar => write!(f, "grammar"),
            Self::Extension => write!(f, "extension"),
            Self::App => write!(f, "app"),
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
// Child manifest (child.toml; legacy plugin.toml supported)
// =========================================================================

/// Parsed child manifest from child.toml (legacy plugin.toml supported).
#[derive(Debug, Clone)]
pub struct ChildManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub world: ChildKind,
    /// Plugin role — what the plugin is FOR (connector, grammar, extension, app).
    /// None for legacy plugins that haven't declared a role yet.
    pub role: Option<ChildRole>,
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
    pub provides: ChildProvides,
    /// Schema packages this plugin references (from [schemas.<name>].package).
    /// Maps schema name → package string (e.g., "forge" → "patina:schema/forge@1.0.0").
    pub schemas: std::collections::HashMap<String, String>,
    pub state_enabled: bool,
    pub checkpoint_streams: Vec<String>,
    pub lake_names: Vec<String>,
    pub ingress_sources: std::collections::HashMap<String, GrantedIngressSource>,
    pub subscribed_streams: Vec<String>,
    pub task_intent_names: Vec<String>,
    pub task_intents: Vec<crate::mother::TaskIntentKind>,
    pub graph_read: bool,
    pub graph_write_actions: Vec<String>,
    pub belief_read: bool,
    pub belief_write_actions: Vec<String>,
    pub toys: crate::mother::GrantedToys,
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
/// Built from ChildManifest at load time. Stored on host state
/// so call-time checks are a HashSet lookup, not manifest re-parsing.
#[derive(Debug, Clone, Default)]
pub struct GrantedCapabilities {
    pub query_kinds: std::collections::HashSet<String>,
    pub query_scope: QueryScope,
    pub http_domains: std::collections::HashSet<String>,
    /// Credential mappings: domain → secret name + injection location.
    /// Empty means no credential injection.
    pub credential_mappings: std::collections::HashMap<String, CredentialMapping>,
    /// Whether plugin can emit facts to eventlog.
    pub host_emit: bool,
    /// Parsed schema facts cached at load time. Outer key = schema name,
    /// inner key = fact-type name, value = event_type string.
    /// Zero disk reads at emit time — all validation from this cache.
    pub schema_facts: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    pub state_enabled: bool,
    pub checkpoint_streams: std::collections::HashSet<String>,
    pub lake_names: std::collections::HashSet<String>,
    pub subscribed_streams: std::collections::HashSet<String>,
    pub task_intents: std::collections::HashSet<crate::mother::TaskIntentKind>,
    pub graph_read: bool,
    pub graph_write_actions: std::collections::HashSet<String>,
    pub belief_read: bool,
    pub belief_write_actions: std::collections::HashSet<String>,
    pub toys: crate::mother::GrantedToys,
}

/// What the child provides to the system.
#[derive(Debug, Default, Clone)]
pub struct ChildProvides {
    pub child: Option<String>,
    pub commands: Vec<String>,
    /// Pipeline operations this plugin handles (e.g., ["parse", "chunk"]).
    pub pipeline_ops: Vec<String>,
    /// Languages (file extensions) this pipeline plugin claims (e.g., ["zig", "nim"]).
    pub languages: Vec<String>,
}

impl ChildManifest {
    /// Parse a child manifest from a TOML file.
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let table: toml::Table = content.parse()?;

        let (plugin, section_label) =
            if let Some(child) = table.get("child").and_then(|v| v.as_table()) {
                (child, "child")
            } else if let Some(plugin) = table.get("plugin").and_then(|v| v.as_table()) {
                eprintln!("deprecated manifest section '[plugin]'; use '[child]'");
                (plugin, "plugin")
            } else {
                anyhow::bail!("missing [child] or [plugin] section");
            };

        let name = plugin
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing {section_label}.name"))?
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

        let world_str = if let Some(kind) = plugin.get("kind").and_then(|v| v.as_str()) {
            kind
        } else if let Some(world) = plugin.get("world").and_then(|v| v.as_str()) {
            eprintln!("deprecated manifest key 'world'; use 'kind'");
            world
        } else {
            anyhow::bail!("missing {section_label}.kind");
        };
        let world = world_str.parse::<ChildKind>()?;

        if path.file_name().and_then(|n| n.to_str()) == Some("plugin.toml") {
            eprintln!("deprecated manifest filename 'plugin.toml'; use 'child.toml'");
        }

        let role = plugin
            .get("role")
            .and_then(|v| v.as_str())
            .map(|s| s.parse::<ChildRole>())
            .transpose()?;

        let patina_min = plugin
            .get("patina_min")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        let needs_table = table.get("needs").and_then(|v| v.as_table());
        let needs_toys = needs_table
            .and_then(|needs| needs.get("toys"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let needs_scopes = needs_table
            .and_then(|needs| needs.get("scopes"))
            .and_then(|v| v.as_table());

        // Parse capabilities
        let cap_table = table.get("capabilities").and_then(|v| v.as_table());
        let mut capabilities: Vec<String> = cap_table
            .map(|cap| {
                cap.iter()
                    .filter(|(_, v)| v.as_bool() == Some(true))
                    .map(|(k, _)| k.clone())
                    .collect()
            })
            .unwrap_or_default();
        for (toy, capability) in [
            ("log", "host_log"),
            ("query", "host_query"),
            ("http", "host_http"),
            ("emit", "host_emit"),
            ("measure", "host_measure"),
            ("layer", "host_layer"),
        ] {
            if needs_toys.iter().any(|entry| entry == toy)
                && !capabilities.iter().any(|cap| cap == capability)
            {
                capabilities.push(capability.to_string());
            }
        }

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
        let host_http_domains: Vec<String> = host_http_domains;

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

        let state_enabled = needs_toys.iter().any(|toy| toy == "state");
        let checkpoint_streams = needs_scopes
            .and_then(|scopes| scopes.get("checkpoint"))
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("streams"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let lake_names = needs_scopes
            .and_then(|scopes| scopes.get("lake"))
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("names"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let ingress_sources = needs_scopes
            .and_then(|scopes| scopes.get("ingress"))
            .and_then(|v| v.as_table())
            .map(|ingress| {
                ingress
                    .iter()
                    .filter_map(|(name, value)| {
                        let table = value.as_table()?;
                        let endpoint = table.get("endpoint")?.as_str()?.to_string();
                        Some((
                            name.clone(),
                            GrantedIngressSource {
                                name: name.clone(),
                                endpoint,
                            },
                        ))
                    })
                    .collect::<std::collections::HashMap<_, _>>()
            })
            .unwrap_or_default();
        let subscribed_streams = needs_scopes
            .and_then(|scopes| scopes.get("events"))
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("subscribe"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let task_intent_names = needs_scopes
            .and_then(|scopes| scopes.get("task"))
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("intents"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let task_intents = task_intent_names
            .iter()
            .filter_map(|v| crate::mother::TaskIntentKind::parse(v))
            .collect::<Vec<_>>();
        let graph_read = needs_scopes
            .and_then(|scopes| scopes.get("graph"))
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("read"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let graph_write_actions = needs_scopes
            .and_then(|scopes| scopes.get("graph"))
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("write"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let belief_read = needs_scopes
            .and_then(|scopes| scopes.get("belief"))
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("read"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let belief_write_actions = needs_scopes
            .and_then(|scopes| scopes.get("belief"))
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("write"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let toys = crate::mother::GrantedToys {
            fetch: needs_toys.iter().any(|toy| toy == "fetch"),
            lake_names: lake_names.iter().cloned().collect(),
            ingress_sources: ingress_sources.clone(),
            connector: needs_toys.iter().any(|toy| toy == "connector"),
            github: needs_toys.iter().any(|toy| toy == "github"),
            session: needs_toys.iter().any(|toy| toy == "session"),
            query: needs_toys.iter().any(|toy| toy == "query"),
            measure: needs_toys.iter().any(|toy| toy == "measure"),
            graph: needs_toys.iter().any(|toy| toy == "graph"),
            belief: needs_toys.iter().any(|toy| toy == "belief"),
        };

        Ok(Self {
            name,
            version,
            description,
            world,
            role,
            patina_min,
            capabilities,
            allowed_toy_commands,
            host_query_kinds,
            host_http_domains,
            host_secrets,
            provides: ChildProvides {
                child,
                commands,
                pipeline_ops,
                languages,
            },
            schemas,
            state_enabled,
            checkpoint_streams,
            lake_names,
            ingress_sources,
            subscribed_streams,
            task_intent_names,
            task_intents,
            graph_read,
            graph_write_actions,
            belief_read,
            belief_write_actions,
            toys,
        })
    }

    /// Resolve canonical child manifest path from a child directory.
    ///
    /// Canonical path is `child.toml`; legacy `plugin.toml` is read for
    /// compatibility and emits a deprecation warning.
    pub fn resolve_child_manifest_path(dir: &Path) -> Option<std::path::PathBuf> {
        let child = dir.join("child.toml");
        if child.exists() {
            return Some(child);
        }
        let legacy = dir.join("plugin.toml");
        if legacy.exists() {
            eprintln!("deprecated manifest filename 'plugin.toml'; use 'child.toml'");
            return Some(legacy);
        }
        None
    }

    /// Load a WASM component from bytes using the shared engine.
    pub(super) fn load_component(wasm: &[u8]) -> Result<Component> {
        Component::new(wasm_engine(), wasm)
    }

    /// Build resolved capabilities from this manifest.
    ///
    /// Called once at load time. The resulting GrantedCapabilities is
    /// stored on CommandHostState for O(1) call-time checks.
    ///
    /// Schema facts are parsed from disk here — zero disk reads at emit time.
    pub fn granted_capabilities(&self) -> GrantedCapabilities {
        let query_kinds = self.host_query_kinds.iter().cloned().collect();
        let http_domains = self.host_http_domains.iter().cloned().collect();
        let mut http_domains: std::collections::HashSet<String> = http_domains;
        for source in self.ingress_sources.values() {
            if let Ok(domain) = self::host_support::validate_http_url(&source.endpoint) {
                http_domains.insert(domain);
            }
        }
        let credential_mappings = self.host_secrets.clone();

        // Parse query_scope from capabilities table if present.
        // For now, default to CurrentProject — AllRepos requires explicit opt-in.
        let query_scope = QueryScope::CurrentProject;

        let host_emit = self.capabilities.contains(&"host_emit".to_string());

        // Parse schemas at load time: schema name → { fact-type → event_type }
        let schema_facts = Self::parse_schema_facts(&self.schemas);

        GrantedCapabilities {
            query_kinds,
            query_scope,
            http_domains,
            credential_mappings,
            host_emit,
            schema_facts,
            state_enabled: self.state_enabled,
            checkpoint_streams: self.checkpoint_streams.iter().cloned().collect(),
            lake_names: self.lake_names.iter().cloned().collect(),
            subscribed_streams: self.subscribed_streams.iter().cloned().collect(),
            task_intents: self.task_intents.iter().copied().collect(),
            graph_read: self.graph_read,
            graph_write_actions: self.graph_write_actions.iter().cloned().collect(),
            belief_read: self.belief_read,
            belief_write_actions: self.belief_write_actions.iter().cloned().collect(),
            toys: self.toys.clone(),
        }
    }

    /// Parse schema.toml files from disk and cache fact→event_type mappings.
    ///
    /// Called once at load time. Returns empty map for schemas that can't
    /// be found or parsed (load-time validation in check_capabilities
    /// will warn separately).
    fn parse_schema_facts(
        schemas: &std::collections::HashMap<String, String>,
    ) -> std::collections::HashMap<String, std::collections::HashMap<String, String>> {
        let project_root = crate::session::SessionManager::find_project_root().ok();
        let root = match project_root {
            Some(ref r) => r,
            None => return std::collections::HashMap::new(),
        };

        schemas
            .keys()
            .filter_map(|schema_name| {
                let schema_path = root
                    .join(".patina/schemas")
                    .join(schema_name)
                    .join("schema.toml");
                let content = std::fs::read_to_string(&schema_path).ok()?;
                let table: toml::Table = content.parse().ok()?;
                let facts = table.get("facts")?.as_array()?;

                let fact_map: std::collections::HashMap<String, String> = facts
                    .iter()
                    .filter_map(|f| {
                        let t = f.as_table()?;
                        let name = t.get("name")?.as_str()?.to_string();
                        let event_type = t.get("event_type")?.as_str()?.to_string();
                        Some((name, event_type))
                    })
                    .collect();

                Some((schema_name.clone(), fact_map))
            })
            .collect()
    }
}
