# Design: Pipe Protocol Types — The Shared Vocabulary

## Why This Work Exists

Patina has two runtimes for children — WASM (via patina-sdk) and native
(via patina-pipe). Today they don't share a type system. The forge
plugin defines its own fact shape, its own error handling, its own
config format. When we build the github-connector as a native child,
it would invent its own types. When Mother routes facts from either
runtime, she'd need to understand two vocabularies.

[[pipe-protocol-is-transport-agnostic]] says the protocol doesn't care
how facts travel. But for that to be true, the *types* must exist
independently of any transport. A Fact is a Fact whether it arrives
over WASM host calls or stdio JSON-RPC. A PipeError means the same
thing whether it's a WASM trap or a JSON-RPC error code.

This crate is the protocol definition in code. It answers: what IS a
fact? What IS an error? What IS a child? Everything above (transport
bindings, routing, children) depends on these answers.

**Origin:** [[session-20260306-061745]] (architecture reframe: pipe is
protocol, not process), [[session-20260306-174214]] (five-lens audit:
PipeError codes are transport detail, zeroize boundary is Mother's code,
opaque cursors, canonical serialization trade-off documented).

## The Type Unification Problem

Today, facts are ad-hoc. The forge plugin emits `(schema, fact_type,
data)` via `host_emit::emit_fact()`. The host side in
`host_support.rs:emit_fact` validates and writes to events.db. But
there's no shared Fact struct — the schema is implicit, the error
handling is `Result<_, String>`, and there's no content addressing.

The pipe architecture needs all of these to be explicit:

| Concept | Today (ad-hoc) | After (shared types) |
|---------|---------------|---------------------|
| Fact shape | Implicit in host_emit args | `Fact` struct with schema, fact_type, data, content_hash, signature |
| Errors | `Result<_, String>` | `PipeError` enum (Transient, Fatal, RateLimited, Partial) |
| Capabilities | Not declared | `Capabilities` struct (provider, data_types, supports_incremental) |
| Config delivery | Plugin-specific | `InitializeParams` + `AuthConfig` |
| Child manifest | `plugin.toml` (WASM only) | `child.toml` for native + `plugin.toml` for WASM |
| Content addressing | None | blake3 over canonical JSON |

## Design Decisions

### 1. PipeError Codes Are Transport Detail

The five-lens audit ([[session-20260306-174214]]) established this
clearly: callers match on the *variant*, not the code. JSON-RPC error
codes (-32001 through -32004) exist only in the transport serialization
layer.

```rust
pub enum PipeError {
    Transient { message: String, retry_after_ms: Option<u64> },
    Fatal { message: String },
    RateLimited { message: String, retry_after_ms: u64 },
    Partial { message: String, emitted: u64 },
}
```

The transport crate (patina-pipe) maps these to JSON-RPC codes via
`jsonrpc_code()` and `from_jsonrpc()`. The SDK (patina-sdk) maps them
to WASM host error codes. The types crate knows nothing about either.

**Why not codes in the enum?** The original parent DESIGN.md had `code:
i32` in every variant. Session 12's Gjengset-lens review caught this:
if callers match on code numbers, you can't add new variants without
breaking everyone. If they match on variants, adding new codes is a
transport-layer detail. Variants are the API. Codes are the wire format.

### 2. Credentials Use Plain String in Types

The audit established that the security boundary is Mother's code, not
the types crate. The credential lifetime:

1. Mother decrypts from vault (using `Zeroizing<String>` from the
   `zeroize` crate already in the dep tree via `age`)
2. Mother serializes to child's stdin (InitializeParams with plain String)
3. Mother drops the `Zeroizing<String>` (zeroed)
4. Child process holds the token for its lifetime
5. OS sandbox prevents exfiltration

The types crate uses `String` because `Zeroizing<String>` can't derive
`Serialize`/`Deserialize` and the child *must* read the token to use it.
Wrapping it in the types crate would add complexity with no security
benefit — the child is already trusted with the credential.

### 3. Opaque Cursors

Children own cursor interpretation. The `since` field in FetchParams
is `Option<String>` — Mother stores it and passes it back. Could be an
RFC 3339 timestamp, a page token, an etag, a sequence number. Mother
doesn't parse it. This keeps the protocol generic and lets each
connector evolve its cursor format without protocol changes.

### 4. Canonical JSON for Content Addressing

blake3 hashing requires deterministic serialization. `serde_json` does
NOT guarantee key ordering. The same data can produce different hashes
on different runs.

**The trade-off (documented per audit requirement):** Data flows through
double serialization — child serializes struct to JSON string, parsed to
Value, re-serialized with sorted keys for hashing. This costs ~ms per
fact for large payloads. Acceptable for correctness of content
addressing. Profile if it becomes a bottleneck.

**Edge case decisions:**
- **Floats:** Rust's default f64 formatting. No normalization.
- **Unicode:** No normalization. NFC/NFD differences produce different
  hashes. The data IS different if the bytes differ.
- **Duplicate keys:** `serde_json::Value` uses Map (last-wins). Resolved
  before hashing.

### 5. child.toml Manifest for Native Children

WASM children use `plugin.toml`. Native children use `child.toml`.
Mother's manifest loader reads the top-level section name to determine
the format: `[child]` = native, `[plugin]` = WASM.

The manifest declares what the child needs and what it can do. Mother
validates at load time AND at call time — a child can't emit facts for
undeclared schemas, can't contact undeclared domains.

```toml
[child]
name = "github-connector"
version = "0.1.0"
type = "connector"
runtime = "native"
lifecycle = "poll"

[capabilities]
data_types = ["issues", "prs"]
supports_incremental = true

[domains]
allowed = ["api.github.com"]

[auth]
required = true
provider = "github"

[schemas.github]
package = "patina:schema/github@1.0.0"
```

### 6. SDK Rename (host_* to Semantic Names)

The `host_*` prefix is WASM jargon. `host_emit::emit_fact` doesn't
communicate intent to humans or LLMs. The rename:

| Old | New | WIT interface |
|-----|-----|---------------|
| `host_emit::emit_fact` | `emit::emit_fact` | Already `emit` |
| `host_http::get` | `fetch::get` | Already `http` |
| `host_log::log` | `log::log` | Already `log` |
| `host_query::query` | `query::query` | Already `query` |

WIT definitions don't change — they already use the short names. Only
the Rust SDK module names change. This is a breaking change for existing
WASM plugins (forge), but the fix is mechanical: update `use` paths.

## Crate Structure

```
crates/patina-pipe-types/
  Cargo.toml              # serde, serde_json, blake3, toml
  src/
    lib.rs                # re-exports all public types
    fact.rs               # Fact, FetchResult
    error.rs              # PipeError enum
    capabilities.rs       # Capabilities, Status, HealthStatus
    config.rs             # FetchParams, AuthConfig, InitializeParams
    canonical.rs          # canonical_json(), content_hash()
    manifest.rs           # ChildManifest (child.toml parser)
```

## Dependency Diagram

```
patina-pipe-types (zero deps beyond serde/blake3)
  +-- patina-sdk (existing WASM SDK, adds dep on pipe-types)
  |     +-- plugins/forge/ (existing WASM child, updated imports)
  +-- patina-pipe (new, native transport -- spec: pipe-native-transport)
  |     +-- children/github-connector/ (new -- spec: github-connector)
  +-- patina (main binary, uses pipe-types for Mother-side validation)
        +-- src/broker/ (new -- spec: mother-broker)
```

## What's NOT In Scope

- **WIT definitions** (`wit/deps/patina-host/host.wit`) — unchanged.
  WIT already uses short names. Only Rust SDK module names change.
- **Host trait impls** (`src/plugin/internal/mother_child.rs`) —
  unchanged. WASM host functions still call `host_support::emit_fact()`.
- **host_support.rs** — unchanged. Mother-broker will later extract
  validation logic, but that's a separate spec.
- **Transport protocol** — this crate defines WHAT messages contain.
  HOW they travel is patina-pipe (native) or patina-sdk (WASM).

## Belief Anchors

- [[pipe-protocol-is-transport-agnostic]] — types must exist
  independently of any transport. This crate IS that independence.
- [[wit-defines-pipe-contract-not-runtime]] — WIT defines the contract.
  This crate is the Rust-side reification of that contract.
- [[host-proxied-io-is-the-security-model]] — the manifest declares
  what a child may do. Mother enforces it. Types crate provides the
  declaration format.

## Open Questions

1. **blake3 dependency.** blake3 is not currently in the dependency
   tree. It's small (no-std compatible, pure Rust fallback). Confirm
   acceptable before adding. Alternative: sha2 is already in tree,
   but blake3 is faster and the architecture DESIGN.md specifies it.

2. **patina-sdk crate structure.** The workspace Cargo.toml lists
   `plugins/sdk` as a member. Need to verify the actual module file
   layout inside `plugins/sdk/src/` before implementing the rename —
   SDK may use `wit-bindgen` macros instead of explicit module files.

## Commits

1. `pipe-types: add patina-pipe-types crate with Fact, PipeError,
   Capabilities, FetchParams` — Create crate, workspace member, all
   type definitions.

2. `pipe-types: implement canonical_json() and content_hash()` —
   Recursive sorted-key serializer, blake3 hashing, cross-runtime
   test fixtures.

3. `pipe-types: add ChildManifest parser for child.toml` — child.toml
   format parser with tests.

4. `sdk: rename host_* modules to semantic names` — patina-sdk module
   renames. Add patina-pipe-types dep. Re-export pipe types.

5. `forge: update imports to use renamed SDK modules` — Import changes
   only. Verify `cargo build --release` passes for entire workspace.

**Commits 4 and 5 are atomic.** Commit 4 renames SDK modules; commit 5
updates forge to use the new names. If 4 ships without 5, forge won't
compile. Both must land in the same build verification cycle. Run
`cargo build --release` after commit 5 to verify the entire workspace.

## Key Files

- `crates/patina-pipe-types/src/fact.rs` — Fact struct, FetchResult
- `crates/patina-pipe-types/src/error.rs` — PipeError enum
- `crates/patina-pipe-types/src/canonical.rs` — deterministic hashing
- `crates/patina-pipe-types/src/config.rs` — FetchParams, AuthConfig
- `crates/patina-pipe-types/src/manifest.rs` — child.toml parser
- `plugins/sdk/src/mother_child/` — SDK rename target
- `plugins/forge/src/lib.rs` — forge import updates

## Reference Implementation

### Fact and FetchResult

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub schema: String,
    pub fact_type: String,
    pub data: serde_json::Value,
    pub content_hash: String,       // "blake3:<hex>"
    #[serde(default)]
    pub signature: String,          // stub until persona-federation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    pub emitted: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,     // opaque, child-owned
}
```

### PipeError

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PipeError {
    Transient { message: String, retry_after_ms: Option<u64> },
    Fatal { message: String },
    RateLimited { message: String, retry_after_ms: u64 },
    Partial { message: String, emitted: u64 },
}

// Transport-only methods (not part of child author API):
impl PipeError {
    pub fn jsonrpc_code(&self) -> i32 { /* -32001..-32004 */ }
    pub fn from_jsonrpc(code: i32, message: String, data: Option<Value>) -> Self { /* ... */ }
}
```

### Canonical JSON

```rust
/// Deterministic JSON serialization for content addressing.
/// Sorted keys, no whitespace, minimal escaping.
pub fn canonical_json(value: &Value) -> Vec<u8> { /* recursive sort */ }

/// blake3 over canonical bytes. Returns "blake3:<hex>".
pub fn content_hash(value: &Value) -> String { /* canonical_json + blake3 */ }
```

Cross-runtime invariant: a WASM child and a native child emitting
identical data MUST produce identical content_hash values. Test
fixtures verify this with pinned inputs and expected hashes.

### Config Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,   // "1.0"
    pub auth: Option<AuthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub token: String,              // plain String (see decision #2)
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchParams {
    pub types: Vec<String>,
    pub since: Option<String>,      // opaque cursor (see decision #3)
    pub limit: u64,
    pub params: serde_json::Value,  // provider-specific
}
```

### ChildManifest

Parsed from `child.toml`. Enums for type safety over stringly-typed
config: `ChildRuntime` (Native/Wasm), `ChildLifecycle` (Poll/Stream/
Manual), `ChildType` (Connector/Transport/Lakehouse/Transform).
