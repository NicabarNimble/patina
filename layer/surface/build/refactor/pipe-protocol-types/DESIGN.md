# Design: Pipe Protocol Types — Shared Crate + Manifest Format

## Approach

New workspace member `crates/patina-pipe-types/` with shared types
for pipe protocol. Both WASM children (via patina-sdk) and native
children (via patina-pipe) depend on this crate. The crate defines
**what** pipe protocol messages contain. Transport crates define
**how** they travel.

After the types crate ships, patina-sdk renames its modules from
`host_*` to semantic names and re-exports patina-pipe-types.

## 1. Crate Structure

```
crates/patina-pipe-types/
  Cargo.toml
  src/
    lib.rs              # re-exports all public types
    fact.rs             # Fact, FetchResult
    error.rs            # PipeError enum
    capabilities.rs     # Capabilities, Status, HealthStatus
    config.rs           # FetchParams, AuthConfig, InitializeParams
    canonical.rs        # canonical_json(), content_hash()
    manifest.rs         # ChildManifest (child.toml parser)
```

### 1.1 Cargo.toml

```toml
[package]
name = "patina-pipe-types"
version = "0.1.0"
edition = "2021"
description = "Shared types for Patina pipe protocol"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
blake3 = "1"
toml = "0.8"
```

No `secrecy` crate — see AuthConfig section for rationale.

Workspace Cargo.toml adds:
```toml
[workspace]
members = [".", "plugins/sdk", ..., "crates/patina-pipe-types"]
```

## 2. Type Definitions

### 2.1 fact.rs — Fact and FetchResult

```rust
use serde::{Deserialize, Serialize};

/// A structured fact emitted by a child through pipe protocol.
///
/// The child provides schema, fact_type, and data. The transport
/// layer adds content_hash (computed from canonical JSON) and
/// signature (stub until persona-federation ships).
///
/// This struct represents the wire format — what appears in pipe/fact
/// JSON-RPC notifications. Host-side fields (seq, timestamp,
/// source_id, provenance) are added by Mother when writing to
/// events.db; they are NOT part of the pipe protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    /// Schema namespace (e.g., "github", "forge", "slack").
    pub schema: String,

    /// Fact type within the schema (e.g., "issue", "pull-request").
    pub fact_type: String,

    /// JSON payload conforming to the schema definition.
    /// Stored as Value for canonical serialization.
    pub data: serde_json::Value,

    /// blake3 hash over canonical JSON of `data`.
    /// Format: "blake3:<hex>". Used for dedup across sources/nodes.
    /// Computed by the transport layer, not by child code.
    pub content_hash: String,

    /// ed25519 signature over content_hash using persona keypair.
    /// Empty string until persona-federation ships keypair infra.
    #[serde(default)]
    pub signature: String,
}

/// Summary returned as the JSON-RPC result of a pipe/fetch call.
/// The actual facts are delivered as pipe/fact notifications during
/// the fetch, not in this response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    /// Number of facts emitted during this fetch.
    pub emitted: u64,

    /// Opaque cursor for incremental fetching.
    /// Child sets this to whatever value makes sense for the source
    /// (timestamp, page token, etag, sequence number). Mother stores
    /// it and passes it back as `since` in the next FetchParams.
    /// None means "no cursor update" (child didn't advance).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}
```

### 2.2 error.rs — PipeError

Design constraint: **JSON-RPC codes are transport detail, not public
API.** Callers match on the variant, not the code. Codes live in the
transport serialization layer (patina-pipe's `send_error()`,
patina-sdk's host error mapping).

```rust
use serde::{Deserialize, Serialize};

/// Error categories for pipe protocol.
///
/// These are the Rust-side error types that child implementations
/// return. The transport layer (patina-pipe or patina-sdk) maps
/// these to JSON-RPC error codes on the wire:
///   Transient  → -32001
///   Fatal      → -32002
///   RateLimited → -32003
///   Partial    → -32004
///
/// Callers (Mother, child authors) match on the variant. They never
/// see or use the numeric codes directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PipeError {
    /// Transient failure — retry after backoff.
    /// Network timeout, service unavailable, temporary API error.
    Transient {
        message: String,
        /// Suggested wait before retry. None = use default backoff.
        #[serde(skip_serializing_if = "Option::is_none")]
        retry_after_ms: Option<u64>,
    },

    /// Fatal failure — do not retry.
    /// Bad credentials, schema mismatch, invalid config, 404.
    Fatal {
        message: String,
    },

    /// Rate limited — source API is throttling.
    /// Mother should wait the specified duration before retrying.
    RateLimited {
        message: String,
        /// How long to wait before retrying (from source API).
        retry_after_ms: u64,
    },

    /// Partial success — some facts emitted before failure.
    /// Mother keeps what it received, retries the remainder.
    Partial {
        message: String,
        /// Number of facts successfully emitted before the error.
        emitted: u64,
    },
}

impl std::fmt::Display for PipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transient { message, .. } => write!(f, "transient: {}", message),
            Self::Fatal { message } => write!(f, "fatal: {}", message),
            Self::RateLimited { message, retry_after_ms } => {
                write!(f, "rate limited: {} (retry after {}ms)", message, retry_after_ms)
            }
            Self::Partial { message, emitted } => {
                write!(f, "partial ({} emitted): {}", emitted, message)
            }
        }
    }
}

impl std::error::Error for PipeError {}

/// JSON-RPC error code mapping — used by transport layers only.
/// Not part of the public API for child authors.
impl PipeError {
    pub fn jsonrpc_code(&self) -> i32 {
        match self {
            Self::Transient { .. } => -32001,
            Self::Fatal { .. } => -32002,
            Self::RateLimited { .. } => -32003,
            Self::Partial { .. } => -32004,
        }
    }

    /// Reconstruct PipeError from JSON-RPC error code + data.
    /// Used by Mother when reading errors from child stdio.
    pub fn from_jsonrpc(code: i32, message: String, data: Option<serde_json::Value>) -> Self {
        match code {
            -32001 => Self::Transient {
                message,
                retry_after_ms: data
                    .as_ref()
                    .and_then(|d| d.get("retry_after_ms"))
                    .and_then(|v| v.as_u64()),
            },
            -32003 => Self::RateLimited {
                message,
                retry_after_ms: data
                    .as_ref()
                    .and_then(|d| d.get("retry_after_ms"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(60_000),
            },
            -32004 => Self::Partial {
                message,
                emitted: data
                    .as_ref()
                    .and_then(|d| d.get("emitted"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
            },
            _ => Self::Fatal { message },
        }
    }
}
```

### 2.3 capabilities.rs — Capabilities and Status

```rust
use serde::{Deserialize, Serialize};

/// Capabilities declared by a child during pipe/initialize.
///
/// Tells Mother what this child can do. Mother uses this to
/// validate fetch requests (don't ask for "prs" if child doesn't
/// declare it) and to route facts (only accept schemas the child
/// declared).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// Provider name (e.g., "github", "slack").
    pub provider: String,

    /// Data types this child can fetch (e.g., ["issues", "prs"]).
    pub data_types: Vec<String>,

    /// Whether this child supports incremental fetching via cursor.
    pub supports_incremental: bool,
}

/// Health status returned by pipe/health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Latency of the health check itself, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Degraded,
    Down,
}
```

### 2.4 config.rs — FetchParams, AuthConfig, InitializeParams

Design constraint: **Credentials use zeroize-on-drop.** We use
`Zeroizing<String>` from the `zeroize` crate (already in the
dependency tree via `age`) rather than adding a new `secrecy` dep.

However, `Zeroizing<String>` cannot derive `Serialize`/`Deserialize`
directly. Since FetchParams crosses the JSON-RPC wire (Mother
serializes, child deserializes), the auth token must be serializable.

**Resolution:** The types crate uses plain `String` for the token
field. The security boundary is **not** the child process (which
must read the token to use it). The security boundary is:
1. Mother zeroes the token after serializing to child's stdin
2. Native children are ephemeral (poll mode) — process exits, memory
   freed
3. Stream-mode children hold the token for their lifetime anyway
4. OS sandbox prevents exfiltration

The `Zeroizing` wrapper applies in **Mother's** code (where it
decrypts from vault, holds briefly, serializes to stdin, then drops).
Not in the shared types crate.

```rust
use serde::{Deserialize, Serialize};

/// Parameters sent by Mother with pipe/initialize.
///
/// Delivers config + credentials to the child at startup.
/// Child stores what it needs and uses it for subsequent
/// pipe/fetch and pipe/health calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    /// Protocol version. Currently "1.0".
    pub protocol_version: String,

    /// Auth configuration for the child's external API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
}

/// Auth configuration delivered via pipe/initialize.
///
/// Mother decrypts the credential from vault and passes it here.
/// The child uses it for API calls. The child never stores it to
/// disk (OS sandbox prevents filesystem writes anyway).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Auth token (e.g., GitHub PAT or OAuth token).
    pub token: String,

    /// Provider name (e.g., "github"). Matches connection config.
    pub provider: String,
}

/// Parameters sent by Mother with pipe/fetch.
///
/// Tells the child what to fetch and provides the cursor from
/// the previous fetch (if any).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchParams {
    /// Data types to fetch (subset of child's capabilities).
    /// e.g., ["issues", "prs"]. Empty = fetch all declared types.
    pub types: Vec<String>,

    /// Opaque cursor from the previous fetch result.
    /// Child interprets this however it wants (timestamp, page
    /// token, etag). None = full fetch (no prior state).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,

    /// Maximum number of items to fetch. 0 = no limit.
    #[serde(default)]
    pub limit: u64,

    /// Provider-specific parameters (e.g., owner, repo for GitHub).
    #[serde(default)]
    pub params: serde_json::Value,
}
```

### 2.5 canonical.rs — Canonical JSON and Content Hashing

Design constraint: **Document the double-serialization.** Data flows:
child serializes struct -> JSON string -> parsed to Value ->
re-serialized with sorted keys for hashing. This is a conscious
trade-off for a simpler emit API.

Edge case decisions:
- **Floats:** serialize with Rust's default float formatting (no
  trailing zeros on integers-as-floats, standard precision for true
  floats). JSON floats that round-trip through `serde_json::Value`
  use f64 — same precision on all platforms.
- **Unicode:** no normalization. Canonical form preserves the exact
  Unicode code points. NFC/NFD differences produce different hashes.
  This is correct — the data IS different if the bytes differ.
- **Duplicate keys:** serde_json::Value uses Map which last-wins on
  duplicate keys. Since we hash the Value (not raw input), duplicates
  are resolved before hashing.

```rust
use serde_json::Value;

/// Serialize a JSON value with deterministic key ordering.
///
/// Rules (from pipe-architecture DESIGN.md §1.4):
/// 1. Object keys sorted lexicographically (Unicode code point order)
/// 2. No whitespace between tokens
/// 3. Numbers: Rust's default serde_json formatting
/// 4. Strings: minimal escaping (serde_json default)
/// 5. Null, booleans: literal
///
/// Cost: re-serialization. The child already serialized to JSON,
/// Mother parsed it to Value, and now we re-serialize with sorted
/// keys. For 100K facts this is measurable (~ms per fact for large
/// payloads). Acceptable for correctness of content addressing.
/// Profile if it becomes a bottleneck — the fix would be a streaming
/// canonical serializer, not a different API.
pub fn canonical_json(value: &Value) -> Vec<u8> {
    let mut buf = Vec::new();
    write_canonical(&mut buf, value);
    buf
}

/// blake3 hash over canonical JSON bytes.
/// Returns "blake3:<hex>" format for storage and comparison.
pub fn content_hash(value: &Value) -> String {
    let canonical = canonical_json(value);
    let hash = blake3::hash(&canonical);
    format!("blake3:{}", hash.to_hex())
}

/// Recursive canonical serializer.
fn write_canonical(buf: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Null => buf.extend_from_slice(b"null"),
        Value::Bool(b) => {
            if *b { buf.extend_from_slice(b"true") }
            else { buf.extend_from_slice(b"false") }
        }
        Value::Number(n) => {
            let s = n.to_string();
            buf.extend_from_slice(s.as_bytes());
        }
        Value::String(s) => {
            // Use serde_json's string escaping
            let escaped = serde_json::to_string(s).unwrap_or_default();
            buf.extend_from_slice(escaped.as_bytes());
        }
        Value::Array(arr) => {
            buf.push(b'[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 { buf.push(b','); }
                write_canonical(buf, item);
            }
            buf.push(b']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            buf.push(b'{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 { buf.push(b','); }
                let escaped_key = serde_json::to_string(key).unwrap_or_default();
                buf.extend_from_slice(escaped_key.as_bytes());
                buf.push(b':');
                write_canonical(buf, &map[*key]);
            }
            buf.push(b'}');
        }
    }
}
```

**Test fixtures for cross-runtime verification:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorted_keys() {
        let v = json!({"z": 1, "a": 2, "m": 3});
        let c = String::from_utf8(canonical_json(&v)).unwrap();
        assert_eq!(c, r#"{"a":2,"m":3,"z":1}"#);
    }

    #[test]
    fn nested_sorted() {
        let v = json!({"b": {"z": 1, "a": 2}, "a": 1});
        let c = String::from_utf8(canonical_json(&v)).unwrap();
        assert_eq!(c, r#"{"a":1,"b":{"a":2,"z":1}}"#);
    }

    #[test]
    fn array_preserves_order() {
        let v = json!([3, 1, 2]);
        let c = String::from_utf8(canonical_json(&v)).unwrap();
        assert_eq!(c, "[3,1,2]");
    }

    #[test]
    fn content_hash_deterministic() {
        let v = json!({"title": "test", "number": 42});
        let h1 = content_hash(&v);
        let h2 = content_hash(&v);
        assert_eq!(h1, h2);
        assert!(h1.starts_with("blake3:"));
    }

    #[test]
    fn key_order_irrelevant_for_hash() {
        let v1 = json!({"a": 1, "b": 2});
        let v2 = json!({"b": 2, "a": 1});
        assert_eq!(content_hash(&v1), content_hash(&v2));
    }

    #[test]
    fn string_escaping() {
        let v = json!({"msg": "hello\nworld"});
        let c = String::from_utf8(canonical_json(&v)).unwrap();
        assert_eq!(c, r#"{"msg":"hello\nworld"}"#);
    }

    #[test]
    fn null_and_bool() {
        let v = json!({"a": null, "b": true, "c": false});
        let c = String::from_utf8(canonical_json(&v)).unwrap();
        assert_eq!(c, r#"{"a":null,"b":true,"c":false}"#);
    }

    #[test]
    fn empty_containers() {
        assert_eq!(String::from_utf8(canonical_json(&json!({}))).unwrap(), "{}");
        assert_eq!(String::from_utf8(canonical_json(&json!([]))).unwrap(), "[]");
    }

    /// Cross-runtime fixture: these exact inputs must produce
    /// these exact hashes. If a WASM child and a native child
    /// both emit the same data, they must get the same hash.
    #[test]
    fn cross_runtime_fixtures() {
        let issue = json!({
            "number": 42,
            "title": "Fix the thing",
            "state": "open"
        });
        let h = content_hash(&issue);
        assert!(h.starts_with("blake3:"));
        assert_eq!(h.len(), 6 + 64); // "blake3:" + 64 hex chars
    }
}
```

## 3. Child Manifest (child.toml)

Native children use `child.toml`. WASM children keep `plugin.toml`.
Mother's manifest loader reads the top-level section name to
determine the format: `[child]` = native path, `[plugin]` = WASM path.

### 3.1 child.toml Format Specification

```toml
[child]
name = "github-connector"      # kebab-case, unique per installation
version = "0.1.0"              # semver
description = "GitHub issues and PRs via REST API"
type = "connector"             # connector | transport | lakehouse | transform
runtime = "native"             # native | wasm
lifecycle = "poll"             # poll | stream | manual

[capabilities]
data_types = ["issues", "prs", "comments", "reviews"]
supports_incremental = true

[domains]
allowed = ["api.github.com"]   # network domains this child may contact

[auth]
required = true                # does this child need credentials?
provider = "github"            # which connection provides them

[schemas.github]
package = "patina:schema/github@1.0.0"  # schema package reference
```

### 3.2 manifest.rs — ChildManifest Parser

```rust
use std::collections::HashMap;
use std::path::Path;

/// Runtime type for a child.
#[derive(Debug, Clone, PartialEq)]
pub enum ChildRuntime {
    Native,
    Wasm,
}

/// Lifecycle mode for a child.
#[derive(Debug, Clone, PartialEq)]
pub enum ChildLifecycle {
    Poll,
    Stream,
    Manual,
}

/// Child type (what it does).
#[derive(Debug, Clone, PartialEq)]
pub enum ChildType {
    Connector,
    Transport,
    Lakehouse,
    Transform,
}

/// Parsed child manifest from child.toml.
///
/// Parallel to PluginManifest for WASM children. Mother's loader
/// reads whichever format is present.
#[derive(Debug, Clone)]
pub struct ChildManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub child_type: ChildType,
    pub runtime: ChildRuntime,
    pub lifecycle: ChildLifecycle,
    pub data_types: Vec<String>,
    pub supports_incremental: bool,
    pub allowed_domains: Vec<String>,
    pub auth_required: bool,
    pub auth_provider: Option<String>,
    pub schemas: HashMap<String, String>,
}

impl ChildManifest {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("read child.toml: {}", e))?;
        Self::from_str(&content)
    }

    pub fn from_str(content: &str) -> Result<Self, String> {
        let table: toml::Table = content.parse()
            .map_err(|e| format!("parse child.toml: {}", e))?;

        let child = table.get("child")
            .and_then(|v| v.as_table())
            .ok_or("missing [child] section")?;

        let name = child.get("name")
            .and_then(|v| v.as_str())
            .ok_or("missing child.name")?
            .to_string();

        let version = child.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0").to_string();

        let description = child.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("").to_string();

        let child_type = match child.get("type").and_then(|v| v.as_str()) {
            Some("connector") => ChildType::Connector,
            Some("transport") => ChildType::Transport,
            Some("lakehouse") => ChildType::Lakehouse,
            Some("transform") => ChildType::Transform,
            Some(other) => return Err(format!("unknown child type: '{}'", other)),
            None => return Err("missing child.type".into()),
        };

        let runtime = match child.get("runtime").and_then(|v| v.as_str()) {
            Some("native") => ChildRuntime::Native,
            Some("wasm") => ChildRuntime::Wasm,
            Some(other) => return Err(format!("unknown runtime: '{}'", other)),
            None => return Err("missing child.runtime".into()),
        };

        let lifecycle = match child.get("lifecycle").and_then(|v| v.as_str()) {
            Some("poll") => ChildLifecycle::Poll,
            Some("stream") => ChildLifecycle::Stream,
            Some("manual") => ChildLifecycle::Manual,
            Some(other) => return Err(format!("unknown lifecycle: '{}'", other)),
            None => ChildLifecycle::Poll,
        };

        let caps = table.get("capabilities").and_then(|v| v.as_table());
        let data_types = caps
            .and_then(|c| c.get("data_types"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let supports_incremental = caps
            .and_then(|c| c.get("supports_incremental"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let allowed_domains = table.get("domains")
            .and_then(|v| v.as_table())
            .and_then(|d| d.get("allowed"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let auth_table = table.get("auth").and_then(|v| v.as_table());
        let auth_required = auth_table
            .and_then(|a| a.get("required"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let auth_provider = auth_table
            .and_then(|a| a.get("provider"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let schemas = table.get("schemas")
            .and_then(|v| v.as_table())
            .map(|st| {
                st.iter().filter_map(|(name, v)| {
                    v.as_table()
                        .and_then(|t| t.get("package"))
                        .and_then(|p| p.as_str())
                        .map(|pkg| (name.clone(), pkg.to_string()))
                }).collect()
            })
            .unwrap_or_default();

        Ok(Self {
            name, version, description, child_type, runtime, lifecycle,
            data_types, supports_incremental, allowed_domains,
            auth_required, auth_provider, schemas,
        })
    }
}
```

## 4. SDK Rename

patina-sdk gains patina-pipe-types as a dependency and renames its
modules. This is a breaking change for existing WASM plugins (forge).

### 4.1 Module Rename Mapping

| Old path | New path | Notes |
|----------|----------|-------|
| `patina_sdk::mother_child::host_emit` | `patina_sdk::mother_child::emit` | Module rename only |
| `patina_sdk::mother_child::host_http` | `patina_sdk::mother_child::fetch` | Module rename only |
| `patina_sdk::mother_child::host_log` | `patina_sdk::mother_child::log` | Module rename only |
| `patina_sdk::mother_child::host_query` | `patina_sdk::mother_child::query` | Module rename only |

### 4.2 Import Changes in Forge Plugin

```rust
// OLD (plugins/forge/src/lib.rs)
use patina_sdk::mother_child::host_log;

// NEW
use patina_sdk::mother_child::log;

// OLD (plugins/forge/src/github.rs)
use patina_sdk::mother_child::{host_emit, host_http, host_log};

// NEW
use patina_sdk::mother_child::{emit, fetch, log};
```

Function call changes:
- `host_log::log(level, msg)` → `log::log(level, msg)`
- `host_emit::emit_fact(s, ft, d)` → `emit::emit_fact(s, ft, d)`
- `host_http::get(url)` → `fetch::get(url)`
- `host_http::post(url, body, ct)` → `fetch::post(url, body, ct)`

### 4.3 patina-sdk Re-exports

patina-sdk adds to its Cargo.toml:
```toml
[dependencies]
patina-pipe-types = { path = "../../crates/patina-pipe-types" }
```

And in its lib.rs:
```rust
pub use patina_pipe_types;
```

### 4.4 SDK Location

patina-sdk lives at `plugins/sdk/` (workspace member `plugins/sdk`).
It does NOT move. Only internal module names change. The `[package]`
name stays `patina-sdk`.

Module files to rename:
```
plugins/sdk/src/mother_child/
    host_emit.rs    → emit.rs
    host_http.rs    → fetch.rs
    host_log.rs     → log.rs
    host_query.rs   → query.rs
    mod.rs          → update re-exports
```

## 5. Dependency Diagram

```
patina-pipe-types (new, zero external deps beyond serde/blake3)
  ├── patina-sdk (existing WASM SDK, adds dep on pipe-types)
  │   └── plugins/forge/ (existing WASM child, updated imports)
  ├── patina-pipe (new, native transport — spec: pipe-native-transport)
  │   └── children/github-connector/ (new — spec: github-connector)
  └── patina (main binary, uses pipe-types for Mother-side validation)
      └── src/broker/ (new — spec: mother-broker)
```

## 6. What This Spec Does NOT Touch

- WIT definitions (`wit/deps/patina-host/host.wit`) — unchanged.
  The WIT interface names (`emit`, `http`, `log`) already use the
  short names. Only the Rust SDK module names change.
- Host trait impls (`src/plugin/internal/mother_child.rs`) — unchanged.
  The WASM host functions still call `host_support::emit_fact()` etc.
  Only the SDK-side names visible to plugin authors change.
- `src/plugin/internal/host_support.rs` — unchanged. emit_fact,
  validate_emit stay where they are. Mother-broker will later extract
  the validation logic to use pipe-types, but that's a separate spec.

## Commits

1. `pipe-types: add patina-pipe-types crate with Fact, PipeError, Capabilities, FetchParams`
   — Create crate, workspace member, all type definitions from
   sections 2.1-2.4.

2. `pipe-types: implement canonical_json() and content_hash()`
   — canonical.rs with recursive sorted-key serializer, blake3
   hashing, test fixtures for cross-runtime verification.

3. `pipe-types: add ChildManifest parser for child.toml`
   — manifest.rs with child.toml format parser, tests.

4. `sdk: rename host_* modules to semantic names`
   — patina-sdk module renames (host_emit→emit, host_http→fetch,
   host_log→log, host_query→query). Add patina-pipe-types dep.
   Re-export pipe types.

5. `forge: update imports to use renamed SDK modules`
   — plugins/forge/ import changes. No logic changes. Verify
   `cargo build --release` passes for entire workspace.

## Key Files

- `crates/patina-pipe-types/src/fact.rs` — Fact struct, FetchResult
- `crates/patina-pipe-types/src/error.rs` — PipeError enum (no codes)
- `crates/patina-pipe-types/src/canonical.rs` — deterministic hashing
- `crates/patina-pipe-types/src/config.rs` — FetchParams, AuthConfig
- `crates/patina-pipe-types/src/manifest.rs` — child.toml parser
- `plugins/sdk/src/mother_child/` — SDK rename target
- `plugins/forge/src/lib.rs` — forge import updates
- `plugins/forge/src/github.rs` — forge import updates

## Open Questions

1. **patina-sdk crate structure.** `Glob` found no files under
   `patina-sdk/src/`. The workspace Cargo.toml lists `plugins/sdk`
   as a member. Need to verify the actual module file layout inside
   `plugins/sdk/src/` before implementing the rename. May be that
   the SDK uses `wit-bindgen` macros instead of explicit module files.

2. **blake3 dependency.** blake3 is not currently in the dependency
   tree. It's small (no-std compatible, pure Rust fallback). Confirm
   acceptable before adding. Alternative: sha2 is already in tree,
   but blake3 is faster and the architecture DESIGN.md specifies it.
