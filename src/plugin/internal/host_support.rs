//! Shared host trait logic — single implementation for all worlds.
//!
//! Each WASM world generates separate Rust types via bindgen, but the
//! logic behind host function implementations is identical. This module
//! centralizes that logic so security-sensitive changes happen in one
//! place, not 3-4x across worlds.
//!
//! F1 fix: eliminates ~700 lines of duplicated host trait logic.
//! F2 fix: path traversal protection in count_layer_files.

use std::path::PathBuf;

use super::command::QueryDispatchFn;
use super::{CredentialMapping, GrantedCapabilities, InjectionLocation, QueryScope};

// =========================================================================
// Log host support
// =========================================================================

/// Shared log implementation — formats and emits plugin log messages.
pub(super) fn log(plugin_name: &str, level_str: &str, message: &str) {
    eprintln!("[plugin:{}] {}: {}", plugin_name, level_str, message);
}

// =========================================================================
// Layer host support
// =========================================================================

pub(super) fn find_project_root(project_root: &Option<PathBuf>) -> Option<String> {
    project_root
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
}

pub(super) fn read_config(project_root: &Option<PathBuf>) -> Result<String, String> {
    let root = project_root
        .as_ref()
        .ok_or_else(|| "no project root".to_string())?;
    let config =
        crate::project::load_with_migration(root).map_err(|e| format!("load config: {}", e))?;
    serde_json::to_string(&config).map_err(|e| format!("serialize config: {}", e))
}

pub(super) fn detect_environment() -> Result<String, String> {
    let env =
        crate::environment::Environment::detect().map_err(|e| format!("detect env: {}", e))?;
    serde_json::to_string(&env).map_err(|e| format!("serialize env: {}", e))
}

pub(super) fn get_stored_tools(project_root: &Option<PathBuf>) -> Vec<String> {
    let root = match project_root.as_ref() {
        Some(r) => r,
        None => return vec![],
    };
    let config = match crate::project::load_with_migration(root) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    config
        .environment
        .map(|e| e.detected_tools)
        .unwrap_or_default()
}

/// Count `.md` files in a layer subdirectory.
///
/// F2 fix: rejects path traversal attempts (`../`, absolute paths).
/// Returns 0 on invalid input — no information leak, no error message
/// that confirms the traversal was attempted.
pub(super) fn count_layer_files(project_root: &Option<PathBuf>, subdir: &str) -> u32 {
    let root = match project_root.as_ref() {
        Some(r) => r,
        None => return 0,
    };
    // F2 FIX: sanitize subdir — reject path traversal
    let sub = std::path::Path::new(subdir);
    if sub.components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir | std::path::Component::RootDir
        )
    }) {
        return 0; // silent reject — no information leak
    }
    let path = root.join("layer").join(sub);
    if let Ok(entries) = std::fs::read_dir(path) {
        entries
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .count() as u32
    } else {
        0
    }
}

pub(super) fn get_project_uid(project_root: &Option<PathBuf>) -> Option<String> {
    let root = project_root.as_ref()?;
    crate::project::get_uid(root)
}

pub(super) fn check_adapter_version(
    project_root: &Option<PathBuf>,
    adapter_name: &str,
) -> Result<Option<String>, String> {
    let root = project_root
        .as_ref()
        .ok_or_else(|| "no project root".to_string())?;
    let adapter = crate::adapters::get_adapter(adapter_name);
    adapter
        .check_for_updates(root)
        .map(|opt| opt.map(|(current, _)| current))
        .map_err(|e| format!("adapter check: {}", e))
}

// =========================================================================
// Query host support
// =========================================================================

/// Keys in query params that are host-controlled and must not be
/// set by plugins. The lib strips these before dispatch when the
/// plugin's scope doesn't grant them.
const SCOPE_RESERVED_KEYS: &[&str] = &["all_repos", "repo", "project_root", "db_path"];

/// Sanitize query params by stripping scope-reserved keys.
///
/// Called before dispatching to the binary callback.
/// Testable independently of wasmtime infrastructure.
pub(super) fn sanitize_query_params(params: &str, scope: &QueryScope) -> String {
    let mut args: serde_json::Value = match serde_json::from_str(params) {
        Ok(v) => v,
        Err(_) => return params.to_string(),
    };

    if matches!(scope, QueryScope::AllRepos) {
        // AllRepos scope: params pass through unmodified
        return params.to_string();
    }

    // CurrentProject: strip all scope-reserved keys
    if let Some(obj) = args.as_object_mut() {
        for key in SCOPE_RESERVED_KEYS {
            obj.remove(*key);
        }
    }

    serde_json::to_string(&args).unwrap_or_else(|_| params.to_string())
}

/// Capability-gated query dispatch.
///
/// Defense in depth: kinds are validated at load time (check_capabilities)
/// AND at call time (grants.query_kinds check below). Query scope is
/// enforced at call time — all_repos requires AllRepos scope.
pub(super) fn query(
    plugin_name: &str,
    grants: &GrantedCapabilities,
    query_fn: &mut Option<QueryDispatchFn>,
    kind: &str,
    params: &str,
) -> Result<String, String> {
    // Call-time gating: kind must be in granted set
    if !grants.query_kinds.contains(kind) {
        return Err(format!(
            "query kind '{}' not granted for plugin '{}'",
            kind, plugin_name
        ));
    }

    // Scope enforcement: deny all_repos explicitly, then sanitize.
    if let Ok(args) = serde_json::from_str::<serde_json::Value>(params) {
        let all_repos = args
            .get("all_repos")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if all_repos && !matches!(grants.query_scope, QueryScope::AllRepos) {
            return Err("all_repos not allowed: plugin query_scope is current_project".to_string());
        }
        if all_repos {
            eprintln!("[plugin:{}] query: all_repos=true (audit)", plugin_name);
        }
    }

    // Sanitize: strip scope-reserved keys so callback can't bypass policy
    let sanitized_params = sanitize_query_params(params, &grants.query_scope);

    // Delegate to binary-provided dispatch function
    let query_fn = query_fn
        .as_mut()
        .ok_or_else(|| "query dispatch not configured".to_string())?;
    query_fn(kind, &sanitized_params)
}

// =========================================================================
// HTTP host support
// =========================================================================

/// Build an HTTP client with cross-domain redirect rejection.
///
/// Delegates to `patina_pipe::http_proxy::build_http_client()`.
pub(crate) fn build_http_client() -> anyhow::Result<reqwest::blocking::Client> {
    patina_pipe::http_proxy::build_http_client()
}

/// Validate and parse an HTTP URL for domain-allowlisted access.
///
/// Delegates to `patina_pipe::http_proxy::validate_http_url()`.
pub(crate) fn validate_http_url(url: &str) -> Result<String, String> {
    patina_pipe::http_proxy::validate_http_url(url)
}

/// Result of an HTTP operation — plain types for cross-world portability.
pub(super) struct HttpResult {
    pub status: u16,
    pub body: String,
}

/// Check if a plugin is granted access to a specific secret.
///
/// Reads `~/.patina/plugin-config/secret-grants.toml`. Format:
/// ```toml
/// [my-plugin]
/// secrets = ["github-token"]
/// ```
///
/// Returns true only if the file exists, the plugin is listed, and the
/// secret is in the plugin's `secrets` array. Denies by default.
pub(super) fn check_secret_grant(plugin_name: &str, secret_name: &str) -> bool {
    let grants_path = crate::paths::plugin::secret_grants_path();
    let content = match std::fs::read_to_string(&grants_path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!(
                "[plugin:{}] no secret-grants.toml — run 'patina plugin grant {} {}' to allow",
                plugin_name, plugin_name, secret_name
            );
            return false;
        }
    };
    let table: toml::Table = match content.parse() {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "[plugin:{}] failed to parse secret-grants.toml: {}",
                plugin_name, e
            );
            return false;
        }
    };
    let plugin_section = match table.get(plugin_name).and_then(|v| v.as_table()) {
        Some(t) => t,
        None => {
            eprintln!(
                "[plugin:{}] not listed in secret-grants.toml — run 'patina plugin grant {} {}' to allow",
                plugin_name, plugin_name, secret_name
            );
            return false;
        }
    };
    let allowed = plugin_section
        .get("secrets")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|v| v.as_str() == Some(secret_name)))
        .unwrap_or(false);
    if !allowed {
        eprintln!(
            "[plugin:{}] secret '{}' not granted — run 'patina plugin grant {} {}' to allow",
            plugin_name, secret_name, plugin_name, secret_name
        );
    }
    allowed
}

/// Resolve a credential for a domain: check grants, decrypt from vault, return value.
///
/// Returns None if no mapping, not granted, secret missing, or decryption fails.
/// Logs warnings for each denial — never errors out.
fn resolve_credential(
    plugin_name: &str,
    grants: &GrantedCapabilities,
    domain: &str,
) -> Option<(String, String)> {
    let mapping = grants.credential_mappings.get(domain)?;

    // Secret grants gate: check user-maintained allowlist before decrypting
    if !check_secret_grant(plugin_name, &mapping.secret_name) {
        return None;
    }

    match crate::secrets::get_global_secret(&mapping.secret_name) {
        Ok(Some(value)) => Some((mapping.secret_name.clone(), value)),
        Ok(None) => {
            eprintln!(
                "[plugin:{}] secret '{}' not found in vault, sending unauthenticated",
                plugin_name, mapping.secret_name
            );
            None
        }
        Err(e) => {
            eprintln!(
                "[plugin:{}] failed to decrypt secret '{}': {}, sending unauthenticated",
                plugin_name, mapping.secret_name, e
            );
            None
        }
    }
}

/// Inject credential into a request builder based on the mapping's location.
pub(crate) fn inject_credential(
    builder: reqwest::blocking::RequestBuilder,
    mapping: &CredentialMapping,
    value: &str,
) -> reqwest::blocking::RequestBuilder {
    match mapping.location {
        InjectionLocation::Bearer => builder.header("Authorization", format!("Bearer {}", value)),
    }
}

/// Scan response body for leaked credential values, replacing with [REDACTED].
///
/// Delegates to `patina_pipe::http_proxy::leak_check()`.
pub(crate) fn leak_check(body: &str, secret_name: &str, secret_value: &str) -> String {
    patina_pipe::http_proxy::leak_check(body, secret_name, secret_value)
}

// =========================================================================
// Measure host support
// =========================================================================

// Single source of truth — shared vocabulary from patina-pipe.
use patina_pipe::measure::VALID_VERBS;

/// Record a measurement event from a plugin.
///
/// Validates verb, checks metrics are numeric JSON, writes to eventlog
/// with source overridden to the plugin name (security: plugins can't
/// impersonate core).
pub(super) fn record_measurement(
    _project_root: &Option<PathBuf>,
    plugin_name: &str,
    verb: &str,
    tool: &str,
    mode: &str,
    metrics_json: &str,
) -> Result<(), String> {
    // Validate verb
    if !VALID_VERBS.contains(&verb) {
        return Err(format!(
            "invalid verb '{}': must be one of {:?}",
            verb, VALID_VERBS
        ));
    }

    // Validate metrics_json is a JSON object with numeric values
    let metrics: serde_json::Value =
        serde_json::from_str(metrics_json).map_err(|e| format!("invalid metrics JSON: {}", e))?;

    let obj = metrics
        .as_object()
        .ok_or_else(|| "metrics must be a JSON object".to_string())?;

    for (key, value) in obj {
        if !value.is_number() {
            return Err(format!("metric '{}' must be numeric, got {}", key, value));
        }
    }

    // Open events.db — measure.* events live in events.db, not patina.db.
    // Core tools use eventlog::open_events_db(); plugins must use the same path.
    let conn = crate::eventlog::open_events_db().map_err(|e| format!("open events.db: {}", e))?;

    // Build event data — source is always the plugin name
    let event_data = serde_json::json!({
        "verb": verb,
        "tool": tool,
        "mode": mode,
        "metrics": metrics,
        "source": plugin_name,
    });

    let event_type = format!("measure.{}", verb);
    let source_id = format!("plugin:{}:{}:{}", plugin_name, tool, mode);
    let timestamp = chrono::Utc::now().to_rfc3339();

    crate::eventlog::insert_event(
        &conn,
        &event_type,
        &timestamp,
        &source_id,
        None,
        &event_data.to_string(),
    )
    .map_err(|e| format!("insert measurement event: {}", e))?;

    Ok(())
}

// =========================================================================
// Emit host support
// =========================================================================

/// Validate emit parameters and resolve the event_type from cached schema.
///
/// Pure validation — no disk I/O. Schemas are parsed once at plugin load
/// time and cached on GrantedCapabilities.schema_facts.
///
/// Checks:
/// 1. Schema exists in cached facts (was declared in manifest + installed)
/// 2. Fact-type exists in schema (returns its event_type)
/// 3. Data is valid JSON
pub(super) fn validate_emit(
    schema_facts: &std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    plugin_name: &str,
    schema: &str,
    fact_type: &str,
    data: &str,
) -> Result<String, String> {
    // 1. Schema must exist in cache (parsed at load time from manifest + disk)
    let facts = schema_facts.get(schema).ok_or_else(|| {
        format!(
            "schema '{}' not available for plugin '{}' (not declared or not installed)",
            schema, plugin_name
        )
    })?;

    // 2. Fact-type must exist in schema — resolve to event_type
    let event_type = facts
        .get(fact_type)
        .ok_or_else(|| format!("fact-type '{}' not found in schema '{}'", fact_type, schema))?;

    // 3. Validate data is valid JSON
    let _: serde_json::Value =
        serde_json::from_str(data).map_err(|e| format!("invalid JSON data: {}", e))?;

    Ok(event_type.clone())
}

/// Emit a structured fact to the project eventlog.
///
/// Validates via cached schema facts (zero disk I/O), writes plugin data
/// directly to events.db. Provenance is carried by source_id ("plugin:<name>"),
/// schema by event_type (e.g., "github.issue"). No wrapper — data shape matches
/// what downstream consumers expect.
///
/// data-architecture-v3 will add explicit provenance/schema columns; until then
/// source_id and event_type carry the signal.
///
/// FROZEN LEGACY PATH — WASM facts bypass the broker routing engine.
/// Native children route through broker::routing::validate_fact() which provides
/// content-hash dedup, manifest schema validation, and transactional cursor writes.
/// This direct-write path exists only for the forge WASM plugin. No new WASM children
/// may use this path — all new children must be native and route through the broker.
/// See DESIGN.md §5 (wasm-routing-resolved).
pub(super) fn emit_fact(
    schema_facts: &std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    plugin_name: &str,
    schema: &str,
    fact_type: &str,
    data: &str,
) -> Result<u64, String> {
    let event_type = validate_emit(schema_facts, plugin_name, schema, fact_type, data)?;

    let conn = crate::eventlog::open_events_db().map_err(|e| format!("open events.db: {}", e))?;

    let timestamp = chrono::Utc::now().to_rfc3339();
    let source_id = format!("plugin:{}", plugin_name);

    // Write plugin data directly — no wrapper envelope.
    // Provenance: 'external' — plugin-emitted facts are external evidence.
    // Schema: event_type = "<schema>.<fact>" (e.g., "github.issue")
    conn.execute(
        "INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data, provenance)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            &event_type,
            &timestamp,
            &source_id,
            Option::<&str>::None,
            data,
            "external"
        ],
    )
    .map_err(|e| format!("insert event: {}", e))?;

    let seq = conn.last_insert_rowid() as u64;
    Ok(seq)
}

/// Domain-allowlisted HTTP POST.
///
/// Defense in depth: domains are validated at load time (check_capabilities)
/// AND at call time (grants.http_domains check). URLs are sanitized by
/// validate_http_url. Cross-domain redirects rejected by client policy.
pub(super) fn http_post(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    url: &str,
    body: &str,
    content_type: &str,
) -> Result<HttpResult, String> {
    let domain = validate_http_url(url)?;
    if !grants.http_domains.contains(&domain) {
        return Err(format!(
            "domain '{}' not in allowlist for plugin '{}'",
            domain, plugin_name
        ));
    }

    // Credential injection: look up mapping, decrypt, inject header
    let credential = resolve_credential(plugin_name, grants, &domain);

    let mut request = http_client
        .post(url)
        .header("Content-Type", content_type)
        .body(body.to_string());

    if let Some((ref _name, ref value)) = credential {
        if let Some(mapping) = grants.credential_mappings.get(&domain) {
            request = inject_credential(request, mapping, value);
        }
    }

    let response = request
        .send()
        .map_err(|e| format!("HTTP POST failed: {}", e))?;
    let status = response.status().as_u16();
    let resp_body = response.text().map_err(|e| format!("read body: {}", e))?;

    // Leak detection: scan response for injected credential value
    let resp_body = match credential {
        Some((ref secret_name, ref secret_value)) => {
            leak_check(&resp_body, secret_name, secret_value)
        }
        None => resp_body,
    };

    Ok(HttpResult {
        status,
        body: resp_body,
    })
}

/// Domain-allowlisted HTTP GET.
pub(super) fn http_get(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    url: &str,
) -> Result<HttpResult, String> {
    let domain = validate_http_url(url)?;
    if !grants.http_domains.contains(&domain) {
        return Err(format!(
            "domain '{}' not in allowlist for plugin '{}'",
            domain, plugin_name
        ));
    }

    // Credential injection: look up mapping, decrypt, inject header
    let credential = resolve_credential(plugin_name, grants, &domain);

    let mut request = http_client.get(url);

    if let Some((ref _name, ref value)) = credential {
        if let Some(mapping) = grants.credential_mappings.get(&domain) {
            request = inject_credential(request, mapping, value);
        }
    }

    let response = request
        .send()
        .map_err(|e| format!("HTTP GET failed: {}", e))?;
    let status = response.status().as_u16();
    let resp_body = response.text().map_err(|e| format!("read body: {}", e))?;

    // Leak detection: scan response for injected credential value
    let resp_body = match credential {
        Some((ref secret_name, ref secret_value)) => {
            leak_check(&resp_body, secret_name, secret_value)
        }
        None => resp_body,
    };

    Ok(HttpResult {
        status,
        body: resp_body,
    })
}
