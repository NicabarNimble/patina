//! wasmtime guts — bindgen, Engine singleton, WasmChild adapter.

use std::path::Path;
use std::sync::OnceLock;

use anyhow::Result;
use wasmtime::component::Component;
use wasmtime::{Config, Engine};

use crate::mother::GrantedIngressSource;

mod child;
pub(crate) mod host_support;
mod pipeline;

#[cfg(test)]
mod tests;

pub use child::ChildEngine;
pub use child::FilesystemPreopen;
pub use pipeline::PipelineEngine;

/// Query dispatch function type.
///
/// Provided by the binary crate at runtime since query engines
/// (retrieval, commands) live in the binary, not the library.
pub type QueryDispatchFn = Box<dyn FnMut(&str, &str) -> Result<String, String> + Send>;

// =========================================================================
// Child kind enum — parsed from manifest, enforced at load time (F4)
// =========================================================================

/// Known child kinds — parsed from manifest, enforced at load time.
#[derive(Debug, Clone, PartialEq)]
pub enum ChildKind {
    Child,
    Pipeline,
}

impl std::str::FromStr for ChildKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        static KNOWLEDGE_CHILD_KIND_ALIAS_WARNED: OnceLock<()> = OnceLock::new();
        match s {
            "child" => Ok(Self::Child),
            "knowledge-child" => {
                if KNOWLEDGE_CHILD_KIND_ALIAS_WARNED.set(()).is_ok() {
                    tracing::warn!(
                        "child kind 'knowledge-child' is deprecated; use 'child' (alias removed in v0.47.0)"
                    );
                }
                Ok(Self::Child)
            }
            "command" => {
                anyhow::bail!("child kind 'command' is retired; migrate to 'child'")
            }
            "task" => {
                anyhow::bail!("child kind 'task' is retired; migrate to 'child'")
            }
            "pipeline" => Ok(Self::Pipeline),
            "mother-child" => {
                anyhow::bail!("child kind 'mother-child' is retired; migrate to 'child'")
            }
            other => anyhow::bail!("unknown child kind: '{}'", other),
        }
    }
}

impl ChildKind {
    /// Capabilities this world is allowed to declare.
    pub fn allowed_capabilities(&self) -> &[&str] {
        match self {
            Self::Child => &[
                "host_log",
                "host_layer",
                "host_filesystem",
                "host_keyvalue",
                "host_sql",
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
            Self::Child => write!(f, "child"),
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
            Self::Connector => &[ChildKind::Child],
            Self::Grammar => &[ChildKind::Pipeline],
            Self::Extension => &[ChildKind::Child],
            Self::App => &[ChildKind::Child],
        }
    }
}

pub(crate) const AUTO_GRANTED_CAPABILITIES: &[&str] = &[
    "host_log",
    "host_layer",
    "host_filesystem",
    "host_keyvalue",
    "host_sql",
    "host_measure",
    "host_emit",
    "host_http",
    "host_query",
];

pub fn check_capabilities(manifest: &ChildManifest) -> Result<()> {
    let allowed = manifest.world.allowed_capabilities();
    let world_denied: Vec<&str> = manifest
        .capabilities
        .iter()
        .filter(|cap| !allowed.contains(&cap.as_str()))
        .map(|s| s.as_str())
        .collect();

    if !world_denied.is_empty() {
        anyhow::bail!(
            "child '{}' (world '{}') requests capabilities not allowed for this world: {}",
            manifest.name,
            manifest.world,
            world_denied.join(", ")
        );
    }

    let denied: Vec<&str> = manifest
        .capabilities
        .iter()
        .filter(|cap| !AUTO_GRANTED_CAPABILITIES.contains(&cap.as_str()))
        .map(|s| s.as_str())
        .collect();

    if !denied.is_empty() {
        anyhow::bail!(
            "child '{}' requests capabilities not granted: {}",
            manifest.name,
            denied.join(", ")
        );
    }

    const KNOWN_QUERY_KINDS: &[&str] = &["scry", "context", "assay"];
    let unknown: Vec<&str> = manifest
        .host_query_kinds
        .iter()
        .filter(|k| !KNOWN_QUERY_KINDS.contains(&k.as_str()))
        .map(|s| s.as_str())
        .collect();

    if !unknown.is_empty() {
        anyhow::bail!(
            "child '{}' requests unknown query kinds: {}",
            manifest.name,
            unknown.join(", ")
        );
    }

    for domain in &manifest.host_http_domains {
        if domain.is_empty() {
            anyhow::bail!(
                "child '{}' has empty HTTP domain in host_http",
                manifest.name
            );
        }
        if !domain.is_ascii() {
            anyhow::bail!(
                "child '{}' has non-ASCII HTTP domain '{}' in host_http",
                manifest.name,
                domain
            );
        }
        if domain.contains('/') {
            anyhow::bail!(
                "child '{}' has path component in HTTP domain '{}' in host_http",
                manifest.name,
                domain
            );
        }
    }

    if manifest.capabilities.contains(&"host_emit".to_string()) && manifest.schemas.is_empty() {
        anyhow::bail!(
            "child '{}' declares host_emit but has no [schemas.*] entries",
            manifest.name
        );
    }

    if manifest.capabilities.contains(&"host_measure".to_string()) {
        for (metric_name, metric) in &manifest.declared_metrics {
            if metric.labels.len() > 10 {
                anyhow::bail!(
                    "child '{}' metric '{}' exceeds max label keys (10)",
                    manifest.name,
                    metric_name
                );
            }
        }
    }

    if manifest.world == ChildKind::Child {
        const KNOWN_STREAMS: &[&str] = &[
            "belief.changed",
            "graph.changed",
            "fact.ingested",
            "session.completed",
            "repo.synced",
            "file.found",
            "file.written",
            "record.extracted",
            "record.validated",
            "record.rejected",
            "record.ready",
            "record.duplicate",
        ];
        for stream in &manifest.subscribed_streams {
            if !KNOWN_STREAMS.contains(&stream.as_str()) {
                anyhow::bail!(
                    "child '{}' requests unknown event stream '{}'",
                    manifest.name,
                    stream
                );
            }
        }

        for source in manifest.ingress_sources.values() {
            host_support::validate_http_url(&source.endpoint).map_err(|error| {
                anyhow::anyhow!(
                    "child '{}' declares invalid ingress source '{}' endpoint '{}': {}",
                    manifest.name,
                    source.name,
                    source.endpoint,
                    error
                )
            })?;
        }
    }

    let http_set: std::collections::HashSet<&str> = manifest
        .host_http_domains
        .iter()
        .map(|s| s.as_str())
        .collect();
    for (domain, mapping) in &manifest.host_secrets {
        if !http_set.contains(domain.as_str()) {
            anyhow::bail!(
                "child '{}': domain '{}' in host_secrets but not in host_http",
                manifest.name,
                domain
            );
        }
        match crate::mother::get_global_secret(&mapping.secret_name) {
            Ok(Some(_)) => {}
            Ok(None) => {
                eprintln!(
                    "[child:{}] warning: secret '{}' not found in vault (domain '{}')",
                    manifest.name, mapping.secret_name, domain
                );
            }
            Err(e) => {
                eprintln!(
                    "[child:{}] warning: could not probe secret '{}': {}",
                    manifest.name, mapping.secret_name, e
                );
            }
        }
    }

    if let Some(ref role) = manifest.role {
        let expected_worlds = role.expected_worlds();
        if !expected_worlds.contains(&manifest.world) {
            eprintln!(
                "[child:{}] warning: role '{}' is unusual for world '{}' (expected: {})",
                manifest.name,
                role,
                manifest.world,
                expected_worlds
                    .iter()
                    .map(|w| w.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    Ok(())
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredMetricType {
    Gauge,
    Counter,
}

#[derive(Debug, Clone)]
pub struct DeclaredMetric {
    pub metric_type: DeclaredMetricType,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemAccessMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemPreopenConfig {
    pub host_path: String,
    pub guest_path: String,
    pub mode: FilesystemAccessMode,
}

// =========================================================================
// Child manifest (child.toml)
// =========================================================================

/// Parsed child manifest from child.toml.
#[derive(Debug, Clone)]
pub struct ChildManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub world: ChildKind,
    /// Child role — what the child is FOR (connector, grammar, extension, app).
    /// None for legacy children that haven't declared a role yet.
    pub role: Option<ChildRole>,
    pub patina_min: String,
    pub capabilities: Vec<String>,
    /// Toy commands this child is allowed to request (from [capabilities.toys].commands).
    /// Empty means no toys allowed.
    pub allowed_toy_commands: Vec<String>,
    /// Query kinds this child is allowed to call (from [capabilities].host_query).
    /// E.g., ["scry", "context", "assay"]. Empty means no query access.
    pub host_query_kinds: Vec<String>,
    /// HTTP domains this child is allowed to access (from [capabilities].host_http).
    /// E.g., ["api.github.com", "hooks.slack.com"]. Empty means no HTTP access.
    pub host_http_domains: Vec<String>,
    /// Credential mappings per domain (from [capabilities.host_secrets]).
    /// Maps domain → secret name + injection location.
    pub host_secrets: std::collections::HashMap<String, CredentialMapping>,
    pub provides: ChildProvides,
    /// Schema packages this child references (from [schemas.<name>].package).
    /// Maps schema name → package string (e.g., "forge" → "patina:schema/forge@1.0.0").
    pub schemas: std::collections::HashMap<String, String>,
    /// Metrics this child is allowed to emit (from [needs.metrics]).
    pub declared_metrics: std::collections::HashMap<String, DeclaredMetric>,
    /// Host filesystem preopens for this child.
    /// Configured from [needs.scopes.filesystem].
    pub filesystem_preopens: Vec<FilesystemPreopenConfig>,
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
    /// Child can only query current project (default).
    #[default]
    CurrentProject,
    /// Child can query all registered repos.
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
    /// Whether child can emit facts to eventlog.
    pub host_emit: bool,
    /// Parsed schema facts cached at load time. Outer key = schema name,
    /// inner key = fact-type name, value = event_type string.
    /// Zero disk reads at emit time — all validation from this cache.
    pub schema_facts: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    /// Declared metric policies from manifest [needs.metrics].
    pub declared_metrics: std::collections::HashMap<String, DeclaredMetric>,
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
    /// Pipeline operations this child handles (e.g., ["parse", "chunk"]).
    pub pipeline_ops: Vec<String>,
    /// Languages (file extensions) this pipeline child claims (e.g., ["zig", "nim"]).
    pub languages: Vec<String>,
}

impl ChildManifest {
    /// Parse a child manifest from a TOML file.
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let table: toml::Table = content.parse()?;

        let child = table
            .get("child")
            .and_then(|v| v.as_table())
            .ok_or_else(|| anyhow::anyhow!("missing [child] section"))?;

        let name = child
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing child.name"))?
            .to_string();

        let version = child
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        let description = child
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let world_str = child
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing child.kind"))?;
        let world = world_str.parse::<ChildKind>()?;

        let role = child
            .get("role")
            .and_then(|v| v.as_str())
            .map(|s| s.parse::<ChildRole>())
            .transpose()?;

        let patina_min = child
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
        let needs_connections = needs_table
            .and_then(|needs| needs.get("connections"))
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
            ("logging", "host_log"),
            ("query", "host_query"),
            ("http", "host_http"),
            ("emit", "host_emit"),
            ("measure", "host_measure"),
            ("keyvalue", "host_keyvalue"),
            ("filesystem", "host_filesystem"),
            ("sql", "host_sql"),
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
        let mut host_http_domains: Vec<String> = host_http_domains;

        // Parse [capabilities.host_secrets] — credential mappings per domain
        let mut host_secrets: std::collections::HashMap<String, CredentialMapping> = cap_table
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

        if let Some(connections) = needs_connections {
            for (connection_name, value) in connections {
                let Some(entry) = value.as_table() else {
                    continue;
                };
                let Some(toy) = entry.get("toy").and_then(|v| v.as_str()) else {
                    continue;
                };
                match toy {
                    "http" => {
                        let record = match crate::connect::load(connection_name) {
                            Ok(record) => record,
                            Err(error) => {
                                eprintln!(
                                    "warning: could not resolve connection '{}' for [needs.connections]: {}",
                                    connection_name, error
                                );
                                continue;
                            }
                        };
                        if !capabilities.iter().any(|cap| cap == "host_http") {
                            capabilities.push("host_http".to_string());
                        }
                        for domain in &record.auth.allowed_domains {
                            if !host_http_domains.iter().any(|d| d == domain) {
                                host_http_domains.push(domain.clone());
                            }
                            if host_secrets.contains_key(domain) {
                                continue;
                            }
                            match &record.auth.injection {
                                crate::connect::InjectionStrategy::Bearer => {
                                    host_secrets.insert(
                                        domain.clone(),
                                        CredentialMapping {
                                            secret_name: record.auth.secret_ref.clone(),
                                            location: InjectionLocation::Bearer,
                                        },
                                    );
                                }
                                _ => {
                                    eprintln!(
                                        "warning: connection '{}' uses unsupported injection strategy for host_secrets; skipping derived mapping",
                                        connection_name
                                    );
                                }
                            }
                        }
                    }
                    "sql" => {
                        // SQL connections are parsed for schema compatibility.
                        // Runtime routing and policy mediation are introduced in later phases.
                    }
                    other => {
                        eprintln!(
                            "warning: unsupported [needs.connections] toy '{}' for connection '{}'; skipping",
                            other, connection_name
                        );
                    }
                }
            }
        }

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

        let declared_metrics = needs_table
            .and_then(|needs| needs.get("metrics"))
            .and_then(|v| v.as_table())
            .map(|metrics_table| {
                metrics_table
                    .iter()
                    .filter_map(|(name, value)| {
                        let table = value.as_table()?;
                        let metric_type = match table.get("type").and_then(|v| v.as_str()) {
                            Some("gauge") => DeclaredMetricType::Gauge,
                            Some("counter") => DeclaredMetricType::Counter,
                            Some(other) => {
                                eprintln!(
                                    "warning: metric '{}' has unknown type '{}', skipping",
                                    name, other
                                );
                                return None;
                            }
                            None => {
                                eprintln!("warning: metric '{}' missing type, skipping", name);
                                return None;
                            }
                        };
                        let labels = table
                            .get("labels")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(ToString::to_string))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        Some((
                            name.clone(),
                            DeclaredMetric {
                                metric_type,
                                labels,
                            },
                        ))
                    })
                    .collect::<std::collections::HashMap<_, _>>()
            })
            .unwrap_or_default();

        let filesystem_preopens = needs_scopes
            .and_then(|scopes| scopes.get("filesystem"))
            .and_then(|v| v.as_table())
            .map(|table| {
                let mut mounts = Vec::new();
                if let Some(path) = table.get("path").and_then(|v| v.as_str()) {
                    let trimmed = path.trim();
                    if !trimmed.is_empty() {
                        mounts.push(FilesystemPreopenConfig {
                            host_path: trimmed.to_string(),
                            guest_path: "/input".to_string(),
                            mode: FilesystemAccessMode::ReadOnly,
                        });
                    }
                }
                if let Some(extra) = table.get("paths").and_then(|v| v.as_array()) {
                    for (idx, value) in extra.iter().enumerate() {
                        if let Some(path) = value.as_str() {
                            let trimmed = path.trim();
                            if !trimmed.is_empty() {
                                mounts.push(FilesystemPreopenConfig {
                                    host_path: trimmed.to_string(),
                                    guest_path: format!("/input-{}", idx + 1),
                                    mode: FilesystemAccessMode::ReadOnly,
                                });
                            }
                        }
                    }
                }
                for (scope_name, scope_value) in table {
                    let Some(scope_table) = scope_value.as_table() else {
                        continue;
                    };
                    let Some(path) = scope_table.get("path").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let trimmed = path.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let mode = match scope_table
                        .get("mode")
                        .and_then(|v| v.as_str())
                        .unwrap_or("read-only")
                    {
                        "read-write" => FilesystemAccessMode::ReadWrite,
                        _ => FilesystemAccessMode::ReadOnly,
                    };
                    mounts.push(FilesystemPreopenConfig {
                        host_path: trimmed.to_string(),
                        guest_path: format!("/{}", scope_name),
                        mode,
                    });
                }
                mounts
            })
            .unwrap_or_default();

        let state_enabled = needs_toys.iter().any(|toy| toy == "keyvalue");
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
            http: needs_toys.iter().any(|toy| toy == "http"),
            events: needs_toys.iter().any(|toy| toy == "events"),
            messaging: needs_toys.iter().any(|toy| toy == "messaging"),
            filesystem: needs_toys.iter().any(|toy| toy == "filesystem"),
            sql: needs_toys.iter().any(|toy| toy == "sql"),
            git: needs_toys.iter().any(|toy| toy == "git"),
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

        if !toys.filesystem && !filesystem_preopens.is_empty() {
            anyhow::bail!(
                "child '{}' declares [needs.scopes.filesystem] but does not grant 'filesystem' in [needs].toys",
                name
            );
        }

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
            declared_metrics,
            filesystem_preopens,
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
    pub fn resolve_child_manifest_path(dir: &Path) -> Option<std::path::PathBuf> {
        let child = dir.join("child.toml");
        if child.exists() {
            return Some(child);
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
            declared_metrics: self.declared_metrics.clone(),
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
