# Design: Pipe Contract Safety — Shared Types for Cross-Crate Wire Formats

## Current State — Two Drift Points

The child side already uses `patina_pipe_types` exclusively. The child
harness in `crates/patina-pipe/src/lib.rs:173` deserializes
`InitializeParams` and `FetchParams` from the JSON-RPC params. The
contract types are defined once in `crates/patina-pipe-types/src/config.rs`.

The broker side ignores these types:

### Drift 1: `build_init_params()` — ad-hoc JSON

```rust
// src/broker/spawn.rs:67-94
pub fn build_init_params(...) -> serde_json::Value {
    let mut params = serde_json::json!({ "protocol_version": "1.0" });
    if requires_token {
        params["auth"] = serde_json::json!({ "token": token, "provider": provider });
    }
    params
}
```

Returns raw `serde_json::Value`. If `InitializeParams` gains a field,
this function silently produces incomplete JSON. The child fails at
deserialization time with a confusing error.

### Drift 2: Duplicate `FetchParams` with manual `to_json()`

```rust
// src/broker/lifecycle.rs:11-58
pub struct FetchParams {
    pub types: Vec<String>,
    pub since: Option<String>,
    pub params: HashMap<String, toml::Value>,  // ← toml::Value, not serde_json::Value
    pub limit: Option<u64>,                    // ← Option, not u64
}

impl FetchParams {
    pub fn to_json(&self) -> serde_json::Value { /* 35 lines of manual mapping */ }
}
```

The shared type in `pipe_types::FetchParams` has `limit: u64` (not
`Option`) and `params: serde_json::Value` (not `HashMap<String,
toml::Value>`). These are semantically compatible but structurally
different — adding a required field to the shared type does not cause
a compile error in the broker.

## Approach

Replace both ad-hoc constructions with the shared types. The broker
becomes a consumer of `patina_pipe_types`, same as the child. Wire
format agreement is enforced at compile time.

### Boundary conversions

Two type mismatches need resolution at the broker boundary (where
source config meets wire protocol):

1. **`params`**: `sources.toml` stores `HashMap<String, toml::Value>`.
   The wire type uses `serde_json::Value`. Convert at construction:
   `serde_json::to_value(&source.params)?` — toml::Value serializes
   to JSON cleanly via serde.

2. **`limit`**: Broker uses `Option<u64>` (None = use default).
   Wire type uses `u64`. Resolve at construction:
   `.unwrap_or(DEFAULT_MAX_BATCH_SIZE as u64)`.

Both conversions happen exactly once, in `write_to_project()` where
the broker builds the fetch params from source config.

## Commits

### 1. `broker: use pipe_types::InitializeParams for pipe/initialize`

**spawn.rs changes:**
- `build_init_params()` returns `serde_json::Value` (keep return type
  for `conn.request()` compatibility) but constructs via
  `InitializeParams` + `serde_json::to_value()`.
- Construct `auth: Option<AuthConfig>` instead of `json!({ "token", "provider" })`.
- Update tests to still assert on JSON values (the wire format
  doesn't change, just how it's built).

**Why not change the return type?** `ChildConnection::request()` takes
`serde_json::Value`. Changing that is `pipe-native-transport` scope.
We serialize the shared type to Value — still gets compile-time field
checking.

### 2. `broker: replace lifecycle::FetchParams with pipe_types::FetchParams`

**lifecycle.rs changes:**
- Delete `pub struct FetchParams` and `impl FetchParams` (35-line `to_json()`).
- Delete `pub struct FetchResult` — identical to `pipe_types::FetchResult`
  (`emitted: u64`, `cursor: Option<String>`). Re-export from pipe_types.
- Re-export both at module level or import at call sites.
- Keep `BrokerFact` — structurally similar to `pipe_types::Fact` but
  `content_hash` is `Option<String>` (broker validates presence in
  routing.rs Step 1, so optionality is intentional at this layer).
- Keep `DEFAULT_MAX_BATCH_SIZE`, `BrokerChild` trait, `NativeChild` —
  those are broker-side concerns, not wire types.

**mod.rs changes:**
- `write_to_project()` constructs `pipe_types::FetchParams` directly:
  ```rust
  let fetch_params = patina_pipe_types::FetchParams {
      types: source.types.clone(),
      since: stored_cursor,
      limit: DEFAULT_MAX_BATCH_SIZE as u64,
      params: serde_json::to_value(&source.params)?,
  };
  ```
- `child.fetch()` call site: `NativeChild::fetch()` currently takes
  `&lifecycle::FetchParams` and calls `.to_json()`. Change to take
  `&pipe_types::FetchParams` and call `serde_json::to_value()`.

**Test updates:**
- Delete `fetch_params_to_json_full` and `fetch_params_to_json_minimal`
  from lifecycle.rs (they test the deleted manual mapping).
- The round-trip tests in commit 3 replace them with stronger guarantees.

### 3. `test: round-trip serde tests for pipe protocol messages`

**patina-pipe-types changes — tests colocated with their types:**
- `config.rs`: add round-trip serde tests for `InitializeParams`
  (with and without auth) and `FetchParams` (full and minimal fields).
- `fact.rs`: add round-trip serde test for `FetchResult`.
- Each test: construct → `serde_json::to_value()` → `serde_json::from_value()` →
  assert fields match. Proves the JSON the broker produces is what the
  child will successfully deserialize.

## Key Files

- `crates/patina-pipe-types/src/config.rs` — shared types (already exists, add tests)
- `src/broker/spawn.rs` — `build_init_params()` (ad-hoc → shared type)
- `src/broker/lifecycle.rs` — duplicate FetchParams (delete, re-export)
- `src/broker/mod.rs` — `write_to_project()` (construct shared FetchParams)

## Dependency Check

`patina-pipe-types` is already a dependency of the main crate (used by
`broker/routing.rs` for `ChildManifest`). No new dependency needed.

## Open Questions

None — all design decisions resolved by the SPEC exit criteria.
