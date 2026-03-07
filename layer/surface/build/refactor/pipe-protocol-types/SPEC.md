---
type: refactor
id: pipe-protocol-types
status: draft
created: 2026-03-06
sessions:
  origin: 20260306-171859
related:
- pipe-architecture
beliefs:
- wit-defines-pipe-contract-not-runtime
- pipe-protocol-is-transport-agnostic
exit_criteria:
- id: pipe-types-crate-compiles
  text: '`patina-pipe-types` crate compiles and is usable by both patina-sdk and future patina-pipe — contains Fact, PipeError, Capabilities, FetchParams, canonical_json(), content_hash()'
  verify: '`cargo build -p patina-pipe-types && cargo test -p patina-pipe-types` — both succeed with no warnings. Cross-runtime test fixtures produce identical content_hash values for pinned inputs.'
  checked: false
- id: child-manifest-defined
  text: 'child.toml manifest format defined with: name, version, type (connector/transport/lakehouse/transform), runtime (native/wasm), lifecycle (poll/stream/manual), capabilities, domains, auth, schemas'
  verify: 'Parse test fixtures (valid child.toml, invalid child.toml with missing required fields). Round-trip: parse → serialize → parse produces identical ChildManifest.'
  checked: false
- id: sdk-renamed
  text: 'patina-sdk renamed: host_emit → emit, host_http → fetch, host_log → log — forge plugin updated to use new names'
  verify: '`cargo build --release` succeeds for entire workspace. `rg "host_emit|host_http|host_log|host_query" plugins/forge/src/` returns zero matches.'
  checked: false
---
# refactor: Pipe Protocol Types — Shared Crate + Manifest Format

> Define the shared type foundation for pipe protocol. Zero-dep crate
> that both WASM children (patina-sdk) and native children (patina-pipe)
> depend on. The protocol defined in code.

## Context

[[spec-pipe-architecture]] defines pipe protocol as JSON-RPC 2.0 +
WIT type contracts. This spec builds the type foundation that all
other pipe architecture child specs depend on.

**What exists today:**
- `host_emit::emit_fact` in patina-sdk emits typed facts via WASM
  host calls — this IS pipe protocol over WASM transport, it just
  doesn't know it yet
- `host_http::get/post` provides domain-allowlisted HTTP — the
  `host_*` prefix is WASM jargon that doesn't communicate intent
- `plugins/forge/plugin.toml` is the prototype child manifest
- `src/plugin/internal/host_support.rs` contains emit validation,
  canonical types, and error handling that should be shared

**What this spec delivers:**
- `crates/patina-pipe-types/` — shared types crate (Fact, PipeError,
  Capabilities, FetchParams, canonical_json(), content_hash())
- child.toml manifest format specification
- patina-sdk rename (host_* → semantic names)

## Current State

Types are scattered and implicit:
- Fact structure is implicit in `emit_fact()` parameters (schema,
  fact_type, data as JSON string) — no Rust type
- Errors are `Result<_, String>` — no typed error categories
- Capabilities exist only in plugin.toml as ad-hoc fields
- Content hashing doesn't exist (data-architecture-v3 added provenance
  but not content addressing)
- Canonical serialization doesn't exist (serde_json key ordering is
  not deterministic)

## Target State

`patina-pipe-types` crate with zero runtime deps beyond serde +
serde_json + blake3:

```
crates/patina-pipe-types/
  src/
    lib.rs          # re-exports
    fact.rs         # Fact, FetchResult
    error.rs        # PipeError (Transient, Fatal, RateLimited, Partial)
    capabilities.rs # Capabilities, Status, HealthStatus
    config.rs       # FetchParams, AuthConfig
    canonical.rs    # canonical_json(), content_hash()
```

patina-sdk updated:
- `host_emit` → `emit` (emit::emit_fact)
- `host_http` → `fetch` (fetch::get, fetch::post)
- `host_log` → `log` (log::log)
- `host_query` → `query` (query::query)
- Re-exports patina-pipe-types for shared type access

child.toml manifest format for native children:
- Uses `[child]` section (not `[plugin]`) with `type`, `runtime`,
  `lifecycle` fields
- Adds `[domains]` section (from current capabilities.host_http)
- Adds `[auth]` section (from current capabilities.host_secrets)
- WASM children keep plugin.toml (existing loader unchanged). Native
  children use child.toml. Mother's manifest loader reads either
  format — `[plugin]` → WASM path, `[child]` → native path.

**Canonical serialization scope:** canonical_json() handles objects
(sorted keys), arrays (preserve order), strings, numbers, booleans,
null. Floats serialize without trailing zeros. Nested objects are
recursively sorted. Binary/blob data is not supported — facts contain
JSON-serializable data only. The implementation session DESIGN.md
will include a test fixture set for cross-runtime verification.

**Design reference:** All type definitions (Fact struct fields,
PipeError variants with JSON-RPC codes, Capabilities fields,
FetchParams) are specified in [[spec-pipe-architecture]] DESIGN.md
§1.3-1.5, §9.1. The implementation session writes the DESIGN.md for
this spec with concrete Rust types derived from those sections.

## Steps

1. Create `crates/patina-pipe-types/` with Fact, PipeError,
   Capabilities, FetchParams types from DESIGN.md sections 1.3-1.5
2. Implement `canonical_json()` and `content_hash()` per DESIGN.md
   section 1.4 (sorted keys, blake3)
3. Add patina-pipe-types as dependency of patina-sdk
4. Rename patina-sdk modules: host_emit → emit, host_http → fetch,
   host_log → log, host_query → query
5. Update forge plugin imports to use new SDK names
6. Define child.toml manifest format spec (extending plugin.toml)
7. Verify: `cargo build --release` succeeds, forge plugin compiles

## Key Files

**Read before implementing:**
- `src/plugin/internal/host_support.rs` — emit_fact, validate_emit,
  canonical types to extract
- `patina-sdk/src/mother_child/` — current host_* modules to rename
- `plugins/forge/plugin.toml` — manifest prototype
- `plugins/forge/src/lib.rs` — consumer of SDK, update imports
- [[spec-pipe-architecture]] DESIGN.md §1, §7.1, §9.1-9.2

## Design Constraints (from architecture review, session 20260306-174214)

- **PipeError: JSON-RPC codes are transport detail, not public API.**
  The Rust enum variants (Transient, Fatal, RateLimited, Partial) are
  the public interface. JSON-RPC error codes (-32001, etc.) belong in
  the transport serialization layer (patina-pipe, patina-sdk), not as
  fields on the PipeError enum itself. Callers match on the variant,
  not the code. (Gjengset lens: the abstraction should match how it's
  consumed.)

- **Credential fields use `SecretString` or zeroize-on-drop.** Any
  credential data in FetchParams/AuthConfig must not linger in memory
  as plain strings. Use `secrecy::SecretString` or equivalent.
  Especially important for stream-mode children where the process is
  long-lived. (Kelley lens: the simple-looking API shouldn't leak
  secrets through error messages or memory.)

- **Cursor is opaque to Mother.** The `since` field in FetchParams
  should be an opaque string that the child owns and interprets — could
  be a timestamp, page token, etag, or sequence number. Mother stores
  it and passes it back on next fetch, but never parses it. (Kelley
  lens: don't assume the cursor is a timestamp when the source API
  might use pagination tokens.)

- **Document the double-serialization in canonical_json.** Data flows:
  child serializes struct → JSON string → parsed to serde_json::Value
  → re-serialized with sorted keys for hashing. This is a conscious
  trade-off for a simpler emit API (child passes `&str`, not generics).
  Name the cost in the DESIGN.md so it's not a mystery when profiling
  100K facts. (Kelley lens: the abstraction should be honest about
  what it does.)

## Non-Goals

- Building the native transport (that's [[spec-pipe-native-transport]])
- Building the routing engine (that's [[spec-mother-broker]])
- Implementing fact signing (depends on persona keypair from
  [[spec-persona-federation]] — stub the signature field)
