---
type: refactor
id: wasi-toy-alignment
status: draft
created: 2026-03-27
sessions:
  origin: 20260327-104954-066673000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[observation-at-the-boundary]]"
sequencing:
  before:
    - folder-text-to-parquet
  note: "Children built after this spec import WASI standards directly. Building on a misaligned toy surface bakes debt into SDK reference examples."
related:
  - wit/toys/deps/
  - wit/knowledge-child/knowledge-child.wit
  - sdk/patina-sdk-core/
  - sdk/patina-sdk-data/
  - sdk/patina-sdk/
exit_criteria:
  - id: wta1-log-aligned
    text: "`patina:log` evaluated against `wasi:logging`. Decision: migrate, extend, or justify keeping custom."
    checked: true
  - id: wta2-state-aligned
    text: "`patina:state` evaluated against `wasi:keyvalue`. Decision: migrate, extend, or justify keeping custom."
    checked: false
  - id: wta3-events-aligned
    text: "`patina:events` evaluated against `wasi:messaging`. Decision: migrate, extend, or justify keeping custom."
    checked: false
  - id: wta4-store-aligned
    text: "`patina:store` evaluated against `wasi:sql`. Decision: migrate, extend, or justify keeping custom."
    checked: false
  - id: wta5-connect-aligned
    text: "`patina:connect` evaluated against already-imported `wasi:http/outgoing-handler`. Decision: merge, layer, or justify separate."
    checked: false
  - id: wta6-measure-aligned
    text: "`patina:measure` (not yet in WIT) evaluated against `wasi:observe` proposals. Decision: adopt standard, define delta, or defer."
    checked: false
  - id: wta7-wit-updated
    text: "All migration decisions implemented in `wit/toys/deps/` and `knowledge-child.wit`."
    checked: false
  - id: wta8-sdk-updated
    text: "SDK crates updated — trait backends, build.rs WIT copies, and feature flags reflect new imports."
    checked: false
  - id: wta9-children-compile
    text: "All existing children compile and pass tests against the updated toy surface."
    checked: false
  - id: wta10-shims-removed
    text: "All compatibility shims in `sdk/patina-sdk-*/src/compat/` are removed. No child imports from compat modules."
    checked: false
---
# refactor: wasi-toy-alignment

## Problem

Patina has 12 custom `patina:*` toy interfaces. Six overlap with existing or emerging WASI proposals. Per the `wasi-is-foundation-not-option` belief, we build with WASI, not parallel to it — standard interfaces are used directly, custom toys cover only the delta.

Building `folder-text-to-parquet` (and all future children) on a misaligned toy surface bakes debt into what becomes SDK reference code.

## Goal

Evaluate each overlapping toy against its WASI counterpart. For each: migrate to standard, extend the standard with a Patina delta, or document why the custom interface is justified. Update WIT, SDK, and existing children.

## Execution Contract (anti-drift)

Core-values anchor is mandatory before each phase starts:

- `layer/core/values/spec-driven-design.md`
- `layer/core/values/dependable-rust.md`
- `layer/core/values/safety-boundaries.md`
- `layer/core/values/unix-philosophy.md`

Execution rules:

1. Read code before write or remove code.
2. No silent scope expansion: update SPEC first if new prerequisite work is discovered.
3. Migration shims are mandatory but temporary: place in `compat/`, mark deprecated, remove before `folder-text-to-parquet` ships.
4. Every completion claim must be backed by reproducible proof command output.

## Non-Goals

- Touching the 6 non-overlapping toys (`task`, `peer`, `git`, `lake`, `checkpoint`, `connector`). Those have no WASI analog.
- Redesigning Mother's toy host implementation. This is a WIT/SDK contract change; host internals adapt to match.
- Waiting for WASI proposals to reach final status. Evaluate against current proposal state and note maturity.

## Migration Shim Policy

Each migrated toy gets a temporary SDK compatibility shim so existing children don't break on day one. Shims are not optional — they're required for every breaking change. But they must die.

Rules:
- Every shim gets a `deprecated` attribute with a message pointing to the WASI replacement.
- Every shim gets a **sunset criterion** in this spec's exit criteria: removed when all children in `children/` compile without it.
- Shims live in a `sdk/patina-sdk-*/src/compat/` module — isolated, not mixed into the main API.
- No new child code may import from `compat/`. Only existing children get the grace period.
- If a shim survives past the next child system MVP (`folder-text-to-parquet`), it's a bug.

## Audit

Source repos added to `patina repo` for ongoing reference: `WebAssembly/wasi-logging`, `WebAssembly/wasi-keyvalue`, `WebAssembly/wasi-messaging`, `WebAssembly/wasi-sql`, `dylibso/observe-sdk`.

### 1. `patina:log` → **MIGRATE** to `wasi:logging`

Current Patina:
```wit
enum level { debug, info, warn, error }
log: func(level: level, message: string);
```

WASI standard:
```wit
enum level { trace, debug, info, warn, error, critical }
log: func(level: level, context: string, message: string);
```

**Diff:** WASI adds `trace` and `critical` levels, plus a `context` string for grouping. Same function shape, same name. Drop-in migration — add the two levels, add context parameter, delete `patina-log.wit`.

**Action:** Replace `import patina:log/log` with `import wasi:logging/logging` in knowledge-child world. Update SDK `LogBackend` trait to pass context. Host adapter maps context to child identity for telemetry correlation.

### 2. `patina:state` → **MIGRATE** to `wasi:keyvalue`

Current Patina:
```wit
get: func(key: string) -> option<string>;
set: func(key: string, value: string) -> result<_, string>;
delete: func(key: string) -> result<_, string>;
list-prefix: func(prefix: string) -> list<string>;
```

WASI standard:
```wit
open: func(identifier: string) -> result<bucket, error>;
resource bucket {
    get: func(key: string) -> result<option<list<u8>>, error>;
    set: func(key: string, value: list<u8>) -> result<_, error>;
    delete: func(key: string) -> result<_, error>;
    exists: func(key: string) -> result<bool, error>;
    list-keys: func(cursor: option<string>) -> result<key-response, error>;
}
```
Plus `batch` (get-many, set-many, delete-many) and `atomics` (CAS, increment) extensions.

**Diff:** Values change from `string` to `list<u8>` (children handle serialization). `bucket` resource replaces flat namespace — maps naturally to Mother's per-child grant isolation. `list-keys` with cursor pagination replaces `list-prefix`. Gains: `exists`, batch ops, atomic CAS.

**Action:** Replace `import patina:state/state` with `import wasi:keyvalue/store`. SDK `StateBackend` trait changes to bucket-based API with `list<u8>` values. `list-prefix` callers migrate to `list-keys` — if prefix filtering is needed, it happens client-side or via a thin helper in SDK.

**Risk: silent behavior drift.** Existing children assume text semantics (string values). Moving to `list<u8>` means children must handle serialization explicitly. SDK should provide ergonomic helpers (`get_string`, `set_string`) that wrap the bytes API to prevent every child from hand-rolling `String::from_utf8`. Migration must audit every `state.get`/`state.set` call site in existing children.

### 3. `patina:events` → **EXTEND** `wasi:messaging` with delta

Current Patina:
```wit
record event { stream-name, offset: u64, event-type, payload, occurred-at }
publish: func(stream-name, event-type, payload) -> result<u64, string>;
subscribe: func(stream-name, after: option<u64>, limit: u32) -> result<list<event>, string>;
ack: func(stream-name, offset: u64) -> result<_, string>;
```

WASI standard (`wasi:messaging@0.2.0-draft`):
```wit
resource client { connect: static func(name: string) -> result<client, error>; }
resource message { constructor(data: list<u8>); topic, content-type, data, metadata... }
// producer interface:
send: func(c: borrow<client>, topic: topic, message: message) -> result<_, error>;
// guest export:
incoming-handler.handle: func(message: message) -> result<_, error>;
```

**Diff:** Fundamentally different model. WASI messaging is fire-and-forget publish + push-based handler. Patina events is pull-based subscribe with offset cursoring and explicit ack — this is checkpoint-recovery infrastructure (hard rule 5: idempotent reruns).

The publish side overlaps: `patina:events.publish` ≈ `wasi:messaging/producer.send`. The subscribe/ack side has no WASI equivalent.

**Action:** Import `wasi:messaging/producer` for the publish path. Define `patina:events-stream` as a delta covering the Patina-specific subscribe/ack/offset semantics. Reuse `wasi:messaging/types.message` as the message envelope where possible. This gives us: standard publish, custom stream-cursor subscribe. The delta is the genuine Patina value (checkpoint recovery), built on the standard foundation.

**Risk: cognitive overhead from split imports.** A child that publishes and subscribes would import from two packages (`wasi:messaging/producer` + `patina:events-stream`). SDK must unify this behind a single `EventsToy` facade so child authors don't think about the split. The naming must be crisp — `patina:events-stream` (not `patina:events-ext` or `patina:events-v2`) signals that this is the stream-cursor layer on top of standard messaging.

**Contract details to lock:**

- **Envelope:** Publish via `wasi:messaging/producer.send` wraps a `wasi:messaging/types.message` — binary data, topic, metadata. `patina:events-stream` subscribe returns a `stream-event` record that wraps the same `message` plus Patina-specific fields: `offset: u64`, `occurred-at: string`. The WASI message is the envelope; Patina adds the cursor.
- **Offset ownership:** Offsets are assigned by the host (Mother), not by the child or the WASI layer. When a child publishes via `wasi:messaging/producer.send`, Mother assigns the offset internally. When a child subscribes via `patina:events-stream`, Mother returns events with offsets. The offset is Mother's truth — children consume it, never generate it.
- **Ack semantics:** `ack(stream-name, offset)` tells Mother that processing through that offset is complete and checkpointable. This is the bridge between events and checkpoint recovery (hard rule 5). If a child crashes and restarts, it subscribes with `after: last_acked_offset` and replays from there.

### 4. `patina:store` → **MIGRATE** to `wasi:sql`

Current Patina:
```wit
use patina:connect/connect@0.1.0.{connection};
query: func(conn: borrow<connection>, query: string) -> result<string, string>;
mutate: func(conn: borrow<connection>, action: string, payload: string) -> result<string, string>;
```

WASI standard:
```wit
resource connection { open: static func(name: string) -> result<connection, error>; }
resource statement { prepare: static func(query: string, params: list<string>) -> result<statement, error>; }
query: func(c: borrow<connection>, q: borrow<statement>) -> result<list<row>, error>;
exec: func(c: borrow<connection>, q: borrow<statement>) -> result<u32, error>;
```
With typed `row` records containing `data-type` variant (int32, int64, float, string, bool, date, timestamp, binary, null).

**Diff:** WASI provides parameterized queries (SQL injection safe), typed results, and its own `connection` resource. Patina's string-in/string-out is simpler but less safe and less useful. `wasi:sql.connection.open(name)` is similar to `patina:connect.resolve(name)` — host-managed named connections.

**Action:** Replace `import patina:store/store` with `import wasi:sql/readwrite`. This also decouples from `patina:connect` for database connections — `wasi:sql` has its own connection resource. `patina:connect` remains for HTTP-level named-service auth (different concern). SDK `StoreBackend` trait changes to parameterized queries with typed results.

**Risk: larger SDK surface change than it looks.** Moving from string-in/string-out to prepared statements + typed `row`/`data-type` results means the `StoreBackend` trait grows significantly. Children that currently do `query(conn, "SELECT ...")` and parse JSON responses now work with `statement::prepare` + typed rows. SDK should provide a convenience layer (e.g., `query_json` wrapper) for children that don't need typed access, so the migration path isn't all-or-nothing.

### 5. `patina:connect` → **KEEP** as Patina delta, reuse WASI HTTP types

Current Patina:
```wit
resource connection;
resolve: func(name: string) -> result<connection, string>;
base-url: func(conn: borrow<connection>) -> string;
request: func(conn: borrow<connection>, method: string, path: string,
    headers: list<header>, body: option<list<u8>>) -> result<response, string>;
```

WASI standard: `wasi:http/outgoing-handler@0.2.6` already imported.

**Assessment:** These serve different purposes. `wasi:http` is raw HTTP. `patina:connect` is named-service resolution where Mother injects auth, base-url, and routing. `resolve("github")` returns a handle where credentials never cross the WASM wall. This IS the Patina authority model.

With `wasi:sql` taking over database connections, `patina:connect` narrows to HTTP-only named-service authority.

**Action:** Keep `patina:connect` but refactor to import `wasi:http/types` for `header`/`response` records instead of defining custom ones. The `resolve` + host-managed-auth pattern is the genuine delta. Everything else uses standard HTTP types.

**Anti-creep rule:** With `wasi:sql` owning database connections, `patina:connect` is HTTP-only named-service authority. No database, no storage, no messaging concerns. If a future toy needs named-service resolution for non-HTTP protocols, it gets its own delta interface — `patina:connect` does not grow to absorb it.

### 6. `patina:measure` → **DESIGN NEW** following WASI conventions

No WIT definition exists yet in Patina. No WASI standard exists — `dylibso/observe-sdk` is host-side instrumentation (instruments WASM at runtime level, no guest WIT interface).

**Assessment:** observe-sdk's approach is philosophically aligned with `observation-at-the-boundary` — the host (Mother) instruments automatically. That covers Mother-tier metrics. But the `measure` toy is for child-declared domain signals (duplicate rate, parse accuracy, etc.) — the optional second tier.

No WASI standard covers "guest emits a named metric." This is a genuine Patina delta.

**Action:** Design `patina:measure` following WASI interface conventions:
```wit
package patina:measure@0.1.0;
interface measure {
    record metric {
        name: string,
        value: float64,
        labels: list<tuple<string, string>>,
    }
    emit: func(metric: metric);
    gauge: func(name: string, value: float64);
    counter: func(name: string, delta: float64);
}
```
Shape follows OpenTelemetry metric primitives (gauge, counter) so it could inform a future `wasi:observe` guest-side proposal. Mother collects, tags with child/pipeline context, and exports to otel backend.

**Cardinality constraint and manifest schema:**

Manifest declaration:
```toml
[needs]
toys = ["measure"]

[needs.metrics]
parse_accuracy = { type = "gauge", labels = ["file_type"] }
records_ingested = { type = "counter", labels = ["source"] }
duplicate_rate = { type = "gauge", labels = [] }
```

Rules:
- Children must declare every metric name, type (gauge/counter), and allowed label keys in `[needs.metrics]`.
- Mother rejects `emit` calls with undeclared metric names — **hard fail**, not warn-and-drop. Silent drops create invisible data loss; hard fail forces the child author to update the manifest. Mother logs the rejection as a Mother-tier error metric (observable via rule 7).
- Maximum label cardinality: 10 distinct label keys per metric. Mother rejects metrics that exceed this at emit time.
- Mother tags every emitted metric with child identity, pipeline context, and objective ID automatically — children never need to add these as labels.

## Approach

Execution order based on dependency and risk:

1. **`patina:log` → `wasi:logging`** — simplest migration, proves the pattern.
2. **`patina:state` → `wasi:keyvalue`** — straightforward, validates bucket/grant mapping.
3. **`patina:store` → `wasi:sql`** — decouples from `patina:connect`, validates typed results.
4. **`patina:connect` refactor** — reuse `wasi:http` types, narrow scope to HTTP authority.
5. **`patina:events` → `wasi:messaging` + delta** — most complex, touches checkpoint semantics.
6. **`patina:measure` design** — new interface, no migration, can parallel with others.

## Verification

### Structural checks

```bash
wasm-tools component wit wit/toys
wasm-tools component wit wit/knowledge-child
cargo check --workspace -q
cargo test -q --workspace
```

### Proof checks

**1. Shim extinction check** — Fails if any `compat/` shim remains once `folder-text-to-parquet` spec is active.
```bash
# Must return 0 files. Any result is a spec violation.
find sdk/patina-sdk-*/src/compat/ -name '*.rs' -not -name 'mod.rs' | grep -c .
```

**2. Events replay determinism check** — Fixture proves crash/replay + ack yields identical output set hash.
```bash
# Test harness: publish N events, ack through offset K, kill child,
# restart child with subscribe(after: K), process remaining,
# compare output set hash to clean run. Must match.
cargo test --test events_replay_determinism -q
```

**3. Measure enforcement check** — Test child emits undeclared metric and gets deterministic rejection.
```bash
# Test child declares [needs.metrics] with metric "foo",
# emits metric "bar" (undeclared). Mother must return a
# deterministic error code, not silently drop.
cargo test --test measure_undeclared_rejection -q
```

## Build Readiness

Ready to start. Two independent lanes:

- **Lane 1 (sequential):** log → state → store → connect → events. These share SDK trait surfaces and child manifest changes. State's type change ripples into store's decoupling from connect, and events depends on the messaging model being settled. Execute in order.
- **Lane 2 (parallel):** measure design. No existing code to migrate, no SDK coupling to the other 5. Can run alongside lane 1.

Log is the first move — simplest migration, proves the WASI import pattern works end-to-end through WIT → SDK → children.
