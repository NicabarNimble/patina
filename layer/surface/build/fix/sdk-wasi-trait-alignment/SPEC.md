---
type: fix
id: sdk-wasi-trait-alignment
status: draft
created: 2026-04-06
sessions:
  origin: 20260405-133644-511306000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
  - "[[pandos-are-products-children-are-compute]]"
related:
  - sdk/patina-sdk/src/toys.rs
  - sdk/patina-sdk/src/child.rs
  - wit/toys/deps/
  - wit/toys/toybox.wit
  - children/file-system-monitor/child.toml
  - children/content-extractor/child.toml
  - children/schema-enforcer/child.toml
  - children/dedup-filter/child.toml
  - children/record-writer/child.toml
  - children/lakehouse-catalog/child.toml
blocks:
  - pando-platform
exit_criteria:

  - id: swa1-log-matches-wasi
    text: "LogBackend trait matches `wasi:logging@0.1.0` shape: single `log(level, context, message)` function with level enum (trace, debug, info, warn, error, critical). Convenience methods (`.info()`, `.error()`) are sugar on top, not the trait contract."
    checked: true

  - id: swa2-keyvalue-matches-wasi
    text: "StateBackend trait matches `wasi:keyvalue/store@0.2.0` shape: bucket resource with `open(identifier)`, `get(key) -> list<u8>`, `set(key, list<u8>)`, `delete(key)`, `exists(key)`, `list-keys(cursor)`. Values are bytes not strings. Bucket identifier scopes access."
    checked: false

  - id: swa3-filesystem-matches-wasi
    text: "LayerFsBackend trait matches `wasi:filesystem@0.2.6` shape: descriptor-based access with preopened directories. Not simplified string path functions. Children that need filesystem access must declare `filesystem` in their `[needs].toys` list."
    checked: false

  - id: swa4-messaging-matches-wasi
    text: "Event publishing is a separate `messaging` toy matching `wasi:messaging/producer@0.2.0` shape: client resource with `connect(name)`, `send(client, message)`. Split from current `events` bundle. Children that only publish list `messaging`. Children that only consume list `events`."
    checked: false

  - id: swa5-http-matches-wasi
    text: "FetchBackend trait matches `wasi:http/outgoing-handler@0.2.6` shape. Not simplified `get(url)/post(url, body)` string functions."
    checked: false

  - id: swa6-sql-matches-wasi
    text: "Current `store` toy (LakeBackend) replaced with `sql` toy matching `wasi:sql/readwrite@0.1.0` shape: `open(name)`, `prepare(query, params)`, `query(connection, statement)`, `exec(connection, statement)`. Child says `sql`, Mother wires to DuckDB. DuckDB is an implementation detail, not a toy contract."
    checked: false

  - id: swa7-patina-delta-documented
    text: "Every `patina:*` toy (`git`, `events-stream`, `measure`, `connect`, `task`, `peer`) has a comment in its WIT file stating: (a) why WASI doesn't cover this, (b) whether a WASI proposal exists that overlaps, (c) if so, how our interface mirrors the proposal shape."
    checked: true

  - id: swa8-canon-children-updated
    text: "All 6 canon children (`file-system-monitor`, `content-extractor`, `schema-enforcer`, `dedup-filter`, `record-writer`, `lakehouse-catalog`) updated to use aligned toy names and traits. `store` → `sql`. `events` split into `messaging` + `events`. `filesystem` explicitly granted where needed. All compile against aligned SDK. Spec-manager stub is out of scope — it will be rebuilt as the slate pando child on the aligned SDK."
    checked: false

  - id: swa9-capability-enforcement
    text: "A child with `toys = [\"log\"]` cannot call keyvalue, filesystem, sql, or git host functions. A child without `filesystem` in its toy grants cannot access the filesystem. Test proves enforcement."
    checked: false

  - id: swa10-mother-toys-registry
    text: "Mother manages a toy registry at `wit/toys/deps/` with version pinning. `patina mother toys status` shows all toys: name, version, source (wasi upstream or patina delta), WASI proposal phase where applicable. `patina mother toys check` verifies local WIT files match pinned versions."
    checked: true

  - id: swa11-mother-toys-sync
    text: "`patina mother toys sync` fetches latest WIT from upstream WASI repos, compares to pinned versions, reports changes. User decides when to bump. Pinned versions tracked in a registry manifest."
    checked: true

  - id: swa12-compile-proof
    text: "SDK, 6 canon children (`patina-ai-child-*`), `patina-ai`, and `patina-mother` all pass `cargo check -q`. `cargo test -q --lib` passes. `patina mother toys status` shows clean alignment."
    checked: false
---
# fix: SDK WASI Trait Alignment

## Problem

The SDK's Rust traits in `toys.rs` don't match the WASI interface shapes
they claim to wrap. Children code against simplified Patina abstractions,
not actual WASI contracts. Additionally, toy grants in the 6 canon children
have structural problems: a custom `store` toy that should be `wasi:sql`,
an `events` toy that bundles publish and subscribe, and filesystem access
without explicit toy grants.

## Toy Priority Rule

Every toy decision follows three tiers in strict order:

1. **WASI standard** — if a stable WASI interface exists, use it exactly.
   No simplification, no Patina wrapper that changes the shape.
2. **WASI proposal** — if a proposal exists but isn't stable, mirror its
   shape as closely as possible. Document where we diverge and why. When
   the proposal stabilizes, we swap to the standard.
3. **Patina delta** — only when WASI has nothing. Document why the child
   can't do this from pure compute, and why no WASI interface covers it.

No exceptions. This rule governs all toy design going forward.

## Divergence Inventory

### Tier 1: WASI Standard — must match exactly

#### `wasi:logging@0.1.0` (phase 2)

WIT:
```
log: func(level: level, context: string, message: string)
level = { trace, debug, info, warn, error, critical }
```

SDK trait:
```rust
trait LogBackend {
    fn debug(message: &str);
    fn info(message: &str);
    fn warn(message: &str);
    fn error(message: &str);
}
```

Gap: no `context` parameter, no `trace`/`critical` levels, split into
separate functions instead of one `log()` with level enum.

#### `wasi:keyvalue/store@0.2.0` (phase 3)

WIT:
```
resource bucket {
    get: func(key: string) -> result<option<list<u8>>, error>
    set: func(key: string, value: list<u8>) -> result<_, error>
    delete: func(key: string) -> result<_, error>
    exists: func(key: string) -> result<bool, error>
    list-keys: func(cursor: option<string>) -> result<key-response, error>
}
open: func(identifier: string) -> result<bucket, error>
```

SDK trait:
```rust
trait StateBackend {
    fn get(key: &str) -> Option<String>;
    fn put(key: &str, value_json: &str) -> Result<(), String>;
    fn delete(key: &str) -> Result<(), String>;
    fn list_prefix(prefix: &str) -> Vec<String>;
}
```

Gaps: no bucket resource (no scoped access), values are strings not bytes,
no `exists()`, `list_prefix` vs cursor-based `list-keys`, no error type
on get.

#### `wasi:filesystem@0.2.6` (phase 3)

WIT: descriptor-based API with preopened directories, streams, directory
entries, file metadata, permissions.

SDK trait:
```rust
trait LayerFsBackend {
    fn read_file(path: &str) -> Result<String, String>;
    fn write_file(path: &str, contents: &str) -> Result<(), String>;
    fn list_dir(path: &str) -> Result<Vec<String>, String>;
    fn delete_file(path: &str) -> Result<(), String>;
    fn move_path(from: &str, to: &str) -> Result<(), String>;
    fn exists(path: &str) -> Result<bool, String>;
}
```

Gaps: no descriptors, no preopened directories, no streams, string content
not bytes, flat string paths not scoped to preopens. Additionally,
`file-system-monitor` uses filesystem via `[needs.scopes.filesystem]`
without declaring `filesystem` in its `[needs].toys` — violates hard rule 2
(toys are explicit grants).

#### `wasi:http/outgoing-handler@0.2.6` (phase 3)

WIT: full HTTP with method variants, headers, trailers, streams, status
codes, TLS config.

SDK trait:
```rust
trait FetchBackend {
    fn get(url: &str) -> Result<String, String>;
    fn post(url: &str, body: &str, content_type: &str) -> Result<String, String>;
}
```

Gaps: only GET/POST, no headers, no status codes, string body not streams,
no method variants.

### Tier 2: WASI Proposal — mirror shape, document divergence

#### `wasi:messaging/producer@0.2.0` (phase 1)

WIT:
```
resource client
connect: func(name: string) -> result<client, string>
send: func(client: borrow<client>, message: message) -> result<u64, string>
message = { topic, content-type, data: list<u8>, metadata }
```

SDK: no direct messaging trait. Event publishing is bundled into `events`
toy via `EmitBackend` and `EventBackend` traits. This bundles WASI-standard
publish with Patina-delta subscribe under one grant, violating least-privilege
(hard rule 3).

Fix: split into `messaging` (WASI proposal shape, publish only) and
`events` (Patina delta, subscribe/consume only).

#### `wasi:sql/readwrite@0.1.0` (phase 1)

WIT:
```
resource connection
resource statement { query, params }
open: func(name: string) -> result<connection, error>
prepare: func(query: string, params: list<string>) -> result<statement, error>
query: func(c, s) -> result<list<row>, error>
exec: func(c, s) -> result<u32, error>
```

SDK: no SQL trait. `lakehouse-catalog` uses custom `store` toy backed by
`LakeBackend` trait with DuckDB-specific operations (`ensure_table`,
`append_json_batch`, `query_json`). This leaks the DuckDB implementation
into the toy contract.

Fix: replace `store` with `sql` matching `wasi:sql` shape. Child says
`sql`, Mother wires to DuckDB. Child never knows it's DuckDB.

### Tier 3: Patina Delta — no WASI coverage

- `patina:git@0.1.0` — version control ops. No WASI proposal.
- `patina:events-stream@0.1.0` — event consumption (subscribe/pull/ack).
  WASI messaging covers producing only; consumption is our delta.
- `patina:measure@0.1.0` — structured metrics recording. No WASI proposal.
- `patina:task@0.1.0` — task queue. No WASI proposal.
- `patina:connect@0.2.0` — authenticated connectors. Extends `wasi:http`
  with credential injection where Mother holds secrets.
- `patina:peer@0.1.0` — P2P event exchange. No WASI proposal.

## Canon Children Audit

Current toy grants and required fixes:

### `file-system-monitor`
```
Current:  toys = ["log", "events", "measure"]
          scopes.filesystem.path = "/tmp"
Fixed:    toys = ["logging", "messaging", "measure", "filesystem"]
          scopes.filesystem.path = "/tmp"
```
- `log` → `logging` (WASI package name)
- `events` → `messaging` (only publishes, doesn't subscribe)
- `filesystem` added as explicit grant (was implicit scope only)

### `content-extractor`
```
Current:  toys = ["log", "events"]
Fixed:    toys = ["logging", "events", "messaging", "filesystem"]
```
- `log` → `logging`
- Already subscribes (events) AND publishes (needs messaging)
- Reads files (needs filesystem)

### `schema-enforcer`
```
Current:  toys = ["log", "events", "measure"]
Fixed:    toys = ["logging", "events", "messaging", "measure"]
```
- `log` → `logging`
- Subscribes (events) AND publishes (messaging)
- No filesystem needed (pure compute on event payloads)

### `dedup-filter`
```
Current:  toys = ["log", "events", "state", "measure"]
Fixed:    toys = ["logging", "events", "messaging", "keyvalue", "measure"]
```
- `log` → `logging`, `state` → `keyvalue` (WASI package names)
- Subscribes (events) AND publishes (messaging)

### `record-writer`
```
Current:  toys = ["log", "state", "events", "measure"]
Fixed:    toys = ["logging", "keyvalue", "events", "messaging", "measure", "filesystem"]
```
- `log` → `logging`, `state` → `keyvalue`
- Subscribes (events) AND publishes (messaging)
- Writes parquet files (needs filesystem)

### `lakehouse-catalog`
```
Current:  toys = ["log", "state", "events", "store"]
Fixed:    toys = ["logging", "keyvalue", "events", "sql"]
```
- `log` → `logging`, `state` → `keyvalue`
- `store` → `sql` (WASI proposal shape, Mother wires to DuckDB)
- Only subscribes (events, no messaging needed — terminal node)

### SQL scope: what changes now vs future

Only `lakehouse-catalog` uses `store` today. Its current operations
(`ensure_table`, `append_json_batch`, `query_json`) map to `wasi:sql`
`exec` and `query` calls. `record-writer` currently uses `keyvalue` for
batch buffering and `filesystem` for parquet writes — it does not use
`store`/`sql`. No other canon child touches SQL. The `store` → `sql`
change affects one child.

## Capability Model Rule

One strict rule for all toys:

**Toy grant enables. Scope constrains.**

- A toy in `[needs].toys` authorizes the child to use that interface.
  Without the grant, calls to that interface fail at the host boundary.
- A scope in `[needs.scopes]` configures a granted toy (paths, streams,
  buckets). Scopes without a corresponding toy grant are rejected by Mother
  at child load time.
- Mother enforces: no grant = no access. No exceptions.

### Messaging vs events authorization

- `messaging` in `[needs].toys` → child can call `wasi:messaging/producer`
  (connect, send). Child publishes events.
- `events` in `[needs].toys` → child can call `patina:events-stream`
  (pull, ack, list-streams). Child consumes events.
- `[needs.scopes.events].subscribe = ["stream.name"]` constrains which
  streams the child can pull from.
- A child with only `messaging` cannot subscribe. A child with only
  `events` cannot publish. Mother checks at the host boundary per call.

### Filesystem authorization

- `filesystem` in `[needs].toys` → child can use `wasi:filesystem`.
- `[needs.scopes.filesystem].path` constrains the preopened directory.
- Without `filesystem` in toys, scope is rejected at load. Without scope,
  filesystem toy has no preopen and all path operations fail.

### DuckDB boundary

The external toy contract is `wasi:sql`. DuckDB is Mother's internal
implementation detail. No child manifest, SDK trait, or WIT interface
references DuckDB. Mother wires `sql.open("catalog")` to her DuckDB
instance. If Mother switches backends, zero children change.

## Root Cause

SDK traits were designed for developer ergonomics, not WASI conformance.
The WIT files reference WASI packages, `wit_bindgen` generates bindings
from them, but the hand-written trait layer in `toys.rs` simplifies the
shapes. Toy grants in canon children were assigned ad-hoc without
auditing against the three-tier priority rule.

## Mother Toy Registry

Mother manages toys as platform infrastructure. WIT files in `wit/toys/deps/`
are the canonical toy contracts. Mother tracks their provenance and versions.

### Registry manifest

`wit/toys/deps/toys-registry.toml`:

```toml
# Tier 1: WASI standard toys
[wasi-logging]
source = "https://github.com/WebAssembly/wasi-logging"
version = "0.1.0"
phase = 2
file = "logging.wit"

[wasi-keyvalue]
source = "https://github.com/WebAssembly/wasi-keyvalue"
version = "0.2.0"
phase = 3
file = "keyvalue.wit"

[wasi-filesystem]
source = "https://github.com/WebAssembly/wasi-filesystem"
version = "0.2.6"
phase = 3
file = "filesystem.wit"

[wasi-http]
source = "https://github.com/WebAssembly/wasi-http"
version = "0.2.6"
phase = 3
file = "http.wit"

# Tier 2: WASI proposal toys — mirror shape, swap when stable
[wasi-messaging]
source = "https://github.com/WebAssembly/wasi-messaging"
version = "0.2.0"
phase = 1
file = "messaging.wit"

[wasi-sql]
source = "https://github.com/WebAssembly/wasi-sql"
version = "0.1.0"
phase = 1
file = "sql.wit"

# Tier 3: Patina delta toys
[patina-git]
source = "patina"
version = "0.1.0"
wasi_overlap = "none"
file = "patina-git.wit"

[patina-events-stream]
source = "patina"
version = "0.1.0"
wasi_overlap = "wasi-messaging covers producing; consumption is our delta"
file = "patina-events-stream.wit"

[patina-measure]
source = "patina"
version = "0.1.0"
wasi_overlap = "none"
file = "patina-measure.wit"

[patina-connect]
source = "patina"
version = "0.2.0"
wasi_overlap = "extends wasi-http with credential injection"
file = "patina-connect.wit"

[patina-task]
source = "patina"
version = "0.1.0"
wasi_overlap = "none"
file = "patina-task.wit"

[patina-peer]
source = "patina"
version = "0.1.0"
wasi_overlap = "none"
file = "patina-peer.wit"
```

### Mother commands (deliverables — these do not exist yet)

These are built as part of this spec:

- `patina mother toys status` — show all toys with version, source, WASI
  phase, tier. Shows divergence between local WIT and pinned version.
- `patina mother toys check` — verify local WIT files match pinned versions.
- `patina mother toys sync` — fetch latest WIT from upstream WASI repos,
  compare to pinned, report what changed. User decides when to bump.

## Implementation Order

### Done
1. **swa10-swa11** — Toy registry manifest and Mother commands. ✓
2. **swa1** — Logging trait aligned to WASI shape. ✓
3. **swa7** — Patina delta toys documented in WIT files. ✓
4. **swa8 partial** — Canon child manifests updated to aligned grant names. ✓

### Next: runtime wiring (blocks all remaining work)
5. **Capability wiring fix** — update `src/child/internal/mod.rs` string
   comparisons from legacy names (`log`, `state`, `events`, `fetch`) to
   aligned names (`logging`, `keyvalue`, `events` + `messaging`, `http`).
   Add `filesystem` and `sql` grant checks. Without this, children with
   aligned manifests don't get their toys at runtime.
6. **Toys check fix** — change `mother toys check` from upstream hash
   comparison to local pinned-hash comparison. Add `hash` field to
   `toys-registry.toml`. `toys check` = offline local verification.
   `toys sync` = upstream comparison (unchanged).

### Then: trait alignments
7. **swa2** — Keyvalue: bucket resource, bytes values, cursor-based list.
8. **swa3** — Filesystem: descriptor model with preopened dirs.
9. **swa4** — Messaging: split from events, match wasi:messaging shape.
10. **swa5** — HTTP: align with outgoing-handler shape.
11. **swa6** — SQL: replace store/LakeBackend with wasi:sql shape.

### Then: children and proof
12. **swa8 remainder** — Canon child `src/lib.rs` code updated to use
    aligned traits.
13. **swa9** — Capability enforcement tests.
14. **swa12** — Compile/test proof.

## Verification

Scoped to SDK + 6 canon children. Workspace-wide checks may pull legacy
children — verify the 7 targets explicitly. Spec-manager stub is out of
scope (rebuilt as slate pando child after this spec completes).

```bash
# SDK compiles
cargo check -q -p patina-sdk --features child

# 6 canon children compile against aligned traits
for child in file-system-monitor content-extractor schema-enforcer \
             dedup-filter record-writer lakehouse-catalog; do
  cargo check -q -p "patina-ai-child-$child" 2>&1 || echo "FAIL: $child"
done

# Host compiles
cargo check -q -p patina-ai
cargo check -q -p patina-mother

# Tests pass
cargo test -q --lib

# Mother toy registry works (deliverable, not assumed)
patina mother toys status
patina mother toys check
```
