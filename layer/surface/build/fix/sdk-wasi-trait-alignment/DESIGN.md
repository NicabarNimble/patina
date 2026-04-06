# Design: SDK WASI Trait Alignment

## Why This Design

The SDK has a trait wrapper layer (`toys.rs`) between children and the
WIT-generated WASI bindings. This layer was built for developer ergonomics
— simpler functions, string values, no resources. But it diverged from
WASI shapes, creating a Patina-only API that prevents ecosystem portability
and misrepresents our WASI alignment.

The fix is not to remove the wrapper layer — it's to make it match. The
trait contracts must be the WASI shapes. Convenience sugar (`.info()` instead
of `.log(Level::Info, "", msg)`) can exist as extension methods, but the
trait itself is the WASI contract.

This matters because the pando platform builds on the SDK. Every new child
and pando will inherit whatever shapes we have. If those shapes are wrong,
every child built from here forward is wrong.

## Build Target

1. SDK traits match their WIT interface shapes exactly for WASI standard
   and WASI proposal toys.
2. All 6 canon children compile against aligned traits with correct toy
   grants.
3. Capability enforcement is proven — ungrant​ed toys are inaccessible.
4. Mother manages toy provenance and upstream synchronization.

## Trait Wrapper Architecture

### Current (broken)

```
WIT (wasi:keyvalue@0.2.0)
  → wit_bindgen generates Rust bindings in child.rs
    → toys.rs defines StateBackend trait (DIFFERENT shape)
      → children code against StateBackend
```

Children never touch the generated bindings. The trait is a wall between
the child and the real WASI interface.

### Target (aligned)

```
WIT (wasi:keyvalue@0.2.0)
  → wit_bindgen generates Rust bindings in child.rs
    → toys.rs defines KeyvalueToy trait (SAME shape as WIT)
      → children code against KeyvalueToy
        → convenience extension: .get_string() wraps .get() + decode
```

The trait IS the WASI contract. Convenience methods are opt-in sugar, not
the contract surface.

### Naming convention

SDK traits follow WASI interface names, not Patina aliases:

| Current name | Aligned name | WASI interface |
|---|---|---|
| `LogBackend` | `LogToy` | `wasi:logging` |
| `StateBackend` | `KeyvalueToy` | `wasi:keyvalue/store` |
| `LayerFsBackend` | `FilesystemToy` | `wasi:filesystem` |
| `FetchBackend` | `HttpToy` | `wasi:http/outgoing-handler` |
| `EmitBackend` | `MessagingToy` | `wasi:messaging/producer` |
| `LakeBackend` | `SqlToy` | `wasi:sql/readwrite` |
| `EventBackend` | `EventsToy` | `patina:events-stream` (delta) |
| `MeasureBackend` | `MeasureToy` | `patina:measure` (delta) |
| `GitBackend` | `GitToy` | `patina:git` (delta) |

### Toy grant naming

child.toml toy names align with the WASI package name or Patina package:

| Current grant | Aligned grant | Source |
|---|---|---|
| `log` | `logging` | `wasi:logging` |
| `state` | `keyvalue` | `wasi:keyvalue` |
| `fs` / implicit | `filesystem` | `wasi:filesystem` |
| `events` (bundled) | `messaging` | `wasi:messaging` (publish) |
| `events` (bundled) | `events` | `patina:events-stream` (consume) |
| `http` | `http` | `wasi:http` |
| `store` | `sql` | `wasi:sql` |
| `measure` | `measure` | `patina:measure` |
| `git` | `git` | `patina:git` |

## Key Design Decisions

### `store` → `wasi:sql`

The `store` toy gave `lakehouse-catalog` direct access to DuckDB-shaped
operations: `ensure_table`, `append_json_batch`, `query_json`. This leaked
DuckDB internals into the toy contract.

`wasi:sql` is a phase 1 proposal with a clean interface: `open(name)`,
`prepare(query, params)`, `query(connection, statement)`, `exec(connection,
statement)`. The child works with SQL. Mother wires `open("catalog")` to
DuckDB. If Mother switches to PostgreSQL tomorrow, the child doesn't change.

`record-writer` also needs SQL for batch inserts. Currently it uses
`keyvalue` for buffering — that stays, but any table operations move to
`sql`.

### Events/messaging split

Currently one `events` grant bundles:
- Publishing events (`wasi:messaging/producer` — WASI proposal)
- Subscribing/pulling events (`patina:events-stream` — our delta)

This violates least-privilege. After split:
- `messaging` — publish only. `wasi:messaging/producer` shape.
- `events` — subscribe/pull/ack only. `patina:events-stream` shape.

Children declare which they need. `schema-enforcer` needs both (subscribes
to `record.extracted`, publishes `record.validated`). `lakehouse-catalog`
only subscribes (terminal node, no outbound events).

### Filesystem grant enforcement

`file-system-monitor` currently has `[needs.scopes.filesystem]` without
`filesystem` in `[needs].toys`. The scope configures the preopen path,
but the toy grant is what authorizes filesystem access.

Fix: `filesystem` must be in `[needs].toys`. Scope configures it. No
grant = no access, regardless of scopes. Mother rejects a child that has
`[needs.scopes.filesystem]` without `filesystem` in its toy list.

### Host implementation changes

Each WASI trait alignment requires corresponding host changes in Mother:

| Toy | Host location | Change needed |
|---|---|---|
| logging | inline in `child/internal/mod.rs` | Add context + level enum routing |
| keyvalue | inline | Add bucket open/scoping, bytes I/O |
| filesystem | `mother/src/toys/layer_fs.rs` | Descriptor model, preopened dirs |
| messaging | inline (part of events) | Separate publish host from subscribe host |
| http | `src/child/toy_host/http.rs` | Full request/response model |
| sql | `src/child/toy_host/lake.rs` | Replace lake-specific ops with generic SQL |

### Convenience sugar pattern

Children shouldn't have to write `bucket.get(key).map(|bytes| String::from_utf8(bytes))` every time. The SDK provides extension methods:

```rust
// The trait contract (matches WASI exactly)
pub trait KeyvalueStore {
    fn open(identifier: &str) -> Result<Bucket, String>;
}

pub trait Bucket {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String>;
    fn set(&self, key: &str, value: &[u8]) -> Result<(), String>;
    // ... matches wasi:keyvalue exactly
}

// Convenience sugar (extension, not contract)
pub trait BucketExt: Bucket {
    fn get_string(&self, key: &str) -> Result<Option<String>, String> {
        self.get(key).map(|opt| opt.map(|b| String::from_utf8_lossy(&b).into()))
    }
    fn set_string(&self, key: &str, value: &str) -> Result<(), String> {
        self.set(key, value.as_bytes())
    }
}
impl<T: Bucket> BucketExt for T {}
```

Children can use `.get_string()` for convenience but the trait they
implement and the interface they export is pure WASI.

## Migration Approach

Incremental, one toy at a time. Each trait alignment is a self-contained
change:

1. Update the SDK trait to match WASI shape
2. Update the host implementation in Mother
3. Update affected children
4. Compile check — all children must compile
5. Commit

The 6 canon children are the only consumers. Each trait change touches
a known set of children. No surprises.

Order by dependency risk (lowest risk first):
- Logging (no children depend on log shape for data flow)
- Keyvalue (4 children use state — dedup, record-writer, lakehouse, spec-manager)
- Filesystem (file-system-monitor, content-extractor, record-writer)
- Messaging/events split (all 6 children affected)
- HTTP (no canon children use it directly)
- SQL (lakehouse-catalog only)

## Open Questions

- Should `wasi:sql` phase 1 proposal shape be taken as-is, or should we
  track the proposal repo for changes before committing? The proposal is
  early — shapes may evolve.
- Do grammar plugins (9 children) need updating, or do they only use `log`?
- Should the `store` toy be removed entirely, or kept as a deprecated alias
  during migration?

## Verification Plan

Each aligned toy is verified by:
1. SDK compiles with the new trait
2. Affected children compile against it
3. `patina mother toys status` shows aligned versions
4. Capability enforcement test passes for the new toy

Final verification:
```bash
cargo check --workspace -q
cargo test -q --lib
patina mother toys status
patina mother toys check
```
