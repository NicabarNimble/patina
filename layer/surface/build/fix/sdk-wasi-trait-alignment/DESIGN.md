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
      → children code against StateBackend (most paths)
      → some children also touch generated bindings directly
```

The trait layer diverges from the WASI shape. Children mostly use the
simplified traits, but some code paths reach through to generated
bindings. The inconsistency means two different APIs for the same toy.

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

`record-writer` does not use SQL today — it uses `keyvalue` for batch
buffering and `filesystem` for parquet writes. If it needs table operations
in the future, it would use `sql`, but that is not part of this spec.

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

### Filesystem: what "aligned" means

Two separate concerns:

**Contract alignment** (SDK trait shape): the SDK trait must match
`wasi:filesystem@0.2.6` — descriptor-based access, preopened directories,
byte streams. Not the current simplified `read_file(path) -> String`.

**Runtime policy** (Mother enforcement): Mother uses wasmtime's preopen
configuration to scope filesystem access. The child's WASM component sees
a preopened directory and uses standard WASI filesystem calls within it.
The grant (`filesystem` in `[needs].toys`) authorizes access. The scope
(`[needs.scopes.filesystem].path`) configures the preopen. This is how
wasmtime and Fastly Compute handle filesystem — preopens are the sandboxing
mechanism.

The child code uses normal `wasi:filesystem` calls (open, read, write).
Mother's wasmtime configuration limits what paths are visible. The SDK
trait matches the WASI interface. No Patina-specific filesystem abstraction.

**Fix for `file-system-monitor`**: add `filesystem` to `[needs].toys`.
Mother rejects any child that has `[needs.scopes.filesystem]` without the
corresponding toy grant.

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
- Keyvalue (3 canon children use state — dedup-filter, record-writer, lakehouse-catalog)
- Filesystem (file-system-monitor, content-extractor, record-writer)
- Messaging/events split (all 6 children affected)
- HTTP (no canon children use it directly)
- SQL (lakehouse-catalog only)

## Resolved Questions

- **`wasi:sql` shape stability:** Take the phase 1 shape as-is. Pin the
  version in the toy registry. If the proposal evolves, `mother toys sync`
  will flag the divergence and we update then. Don't wait for stability —
  mirror the proposal now, adjust later.
- **Grammar plugins:** Audit during swa8. They likely only use `logging`.
  If so, only the grant name changes (`log` → `logging`). If any use other
  toys, update accordingly.
- **`store` toy removal:** Hard cutover. No deprecated alias. `store` is
  removed from the SDK, replaced by `sql`. `lakehouse-catalog` is the only
  consumer — one child to update.

## Verification Plan

Each aligned toy is verified by:
1. SDK compiles with the new trait
2. Affected children compile against it
3. `patina mother toys status` shows aligned versions
4. Capability enforcement test passes for the new toy

Final verification (scoped to SDK + 6 canon children, not workspace):
```bash
cargo check -q -p patina-sdk --features child
for child in file-system-monitor content-extractor schema-enforcer \
             dedup-filter record-writer lakehouse-catalog; do
  cargo check -q -p "patina-ai-child-$child"
done
cargo check -q -p patina-ai
cargo check -q -p patina-mother
cargo test -q --lib
patina mother toys status
patina mother toys check
```

## Commits

### Phase 1: Toy registry (swa10, swa11)
1. [[commit-b3d0f9af]] — `patina mother toys status` command + registry manifest
2. [[commit-babb8044]] — `patina mother toys check` command (hash comparison)
3. [[commit-c8b8fcb2]] — `patina mother toys sync` command (upstream fetch)

### Phase 2: SDK trait alignment (swa1 only — swa2-6 pending)
4. [[commit-e77250ec]] — logging trait aligned to `wasi:logging@0.1.0` shape

### Phase 3: Patina delta documentation (swa7)
5. [[commit-e9414e23]] — all 6 patina delta WIT files documented

### Phase 4: Canon child manifests (swa8 partial — code updates pending)
6. [[commit-9faed641]] — file-system-monitor toy grants aligned
7. [[commit-392b3b91]] — content-extractor toy grants aligned
8. [[commit-a1629be2]] — schema-enforcer toy grants aligned
9. [[commit-5f97eff1]] — dedup-filter toy grants aligned
10. [[commit-1442a443]] — record-writer toy grants aligned
11. [[commit-5c619efa]] — lakehouse-catalog toy grants aligned (store → sql)

### Still pending
- swa2-6: keyvalue, filesystem, messaging split, http, sql trait alignments
- swa8: child `src/lib.rs` code updates to use aligned trait names
- swa9: capability enforcement tests
- swa12: final compile proof

### Known issues from build
- `mother toys check` reports hash mismatches — upstream WASI repos have
  inconsistent tagging/file layouts, complicating strict pinned-hash checks
- Mother's capability mapping in `src/child/internal/mod.rs` uses legacy
  toy aliases — new grant names need deeper wiring for enforcement
