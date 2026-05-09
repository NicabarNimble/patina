# Design: Mother View Buffer Runtime

## Design intent

This is the first implementation slice of the Allium target:

- `layer/allium/mother/mother-view-composer-target.allium`

The goal is to make the Mother view system real without recreating Atlas as another hardcoded dashboard. Mother owns live buffers; renderers connect later.

## Core value anchors

Read before implementation:

- `layer/core/spec-driven-design.md` — SPEC is the authority for non-trivial work.
- `layer/core/dependable-rust.md` — small public interface, private internals.
- `layer/core/unix-philosophy.md` — one module/job at a time, compose small tools.
- `layer/core/adapter-pattern.md` — no speculative trait boundaries.
- `layer/core/values/contract-first-execution.md` — Mother authority and contract-shaped execution boundaries.

## Operator rules

- Read current Mother code before writing new code.
- Start with existing route/runtime/store patterns, then add the smallest compatible surface.
- Make scalpel commits as each slice lands: spec/design, code reading notes if needed, model, service, API, tests/docs.
- Do not accumulate a broad shotgun diff before committing.

## Vocabulary

Use Emacs vocabulary directly:

| Term | Meaning in Patina Mother |
|---|---|
| Buffer | Mother-owned live display object over observed data |
| Frame | UI/client host, e.g. future SvelteKit, TUI, Emacs client |
| Window | A visible slot in a frame connected to one buffer |
| Major mode | Primary interpretation/render mode, e.g. table/log/markdown |
| Minor mode | Optional overlay behavior, e.g. pinned/filtered/grouped/alerting |
| Kill buffer | Close the Mother-owned buffer |
| Switch/connect buffer | Attach a frame window to an existing buffer |

## Slice boundaries

### Build now

- Rust domain model for buffers/shapes/catalog/gaps.
- In-memory or SQLite-backed Mother state for v1 buffers.
- Minimal Mother API routes or child/control-plane operations for:
  - list buffers
  - open proof buffer
  - connect window
  - disconnect window
  - kill buffer
- Minimal data catalog backed by existing Mother health/status facts.
- Observability-gap artifact when required facts are missing.
- WIT-framed JSON payload structure as data, not generated TS/Svelte code.
- Tests that trace to Allium obligations.

### Do not build yet

- SvelteKit frame.
- Full Allium compiler.
- Dynamic LLM shape creation.
- Persistent user view library.
- Live push transport beyond truthful buffer state.
- Typed WIT promotion.

## Proposed Rust module layout

```text
mother/src/view_buffer/
  mod.rs
  model.rs          # Buffer, Frame, Window, ViewShape, requirements, gaps
  catalog.rs        # Minimal data catalog and fact lookup
  store.rs          # Persistence/in-memory seam
  service.rs        # Open/list/connect/disconnect/kill behavior
  payload.rs        # WIT-framed JSON envelope
  tests.rs
```

If crate boundaries make this easier in the CLI crate first, keep the public seam small and move Mother-native logic into `mother/src/` before completing the spec.

## Minimal model

### Buffer

Fields:

- `buffer_id`
- `context_id` or user/mother identifiers
- `shape_id`
- `name`
- `state: live | stale | blocked | replaced | killed`
- timestamps: created/stale/blocked/replaced/killed
- `major_mode`
- `minor_modes`
- `payload_contract: framed_json`
- `payload_version`

### ViewShape

For this slice, shapes may be built-in Rust records, but names should match the Allium concept.

Fields:

- `shape_id`
- `title`
- `scope: mother_user | vision | project | buffer_local`
- `major_mode`
- `minor_modes`
- `requirements[]`
- payload metadata

### ViewRequirement

Each requirement references a catalog fact:

```text
fact_path: mother.status.control_plane_ready
required: true
purpose: display/control-plane readiness
```

### ObservabilityGap

Fields:

- `gap_id`
- `shape_id`
- `missing_fact_path`
- `missing_source_id?`
- `reason`
- `status: open | linked_to_work_item | resolved`
- `created_at`

## Minimal data catalog

Start with known Mother status facts already available from health/status code.

Potential fact paths:

```text
mother.status.version
mother.status.uptime_secs
mother.status.control_plane_ready
mother.status.registered_projects
mother.status.children_ready_count
mother.status.children_total
mother.status.startup_profile
mother.status.memory.pressure
```

The catalog must distinguish:

- source exists and fact observed
- source exists but fact stale/unavailable
- fact not catalogued

## API shape

Exact route naming can be adjusted, but keep semantics small and explicit.

Candidate routes:

```text
GET  /api/view-buffers
POST /api/view-buffers/open
POST /api/view-buffers/connect
POST /api/view-buffers/disconnect
POST /api/view-buffers/kill
GET  /api/view-buffers/gaps
```

Opening by proof shape could be:

```json
{
  "shape_id": "mother.status.default",
  "frame_id": "cli-debug",
  "window_id": "window-1"
}
```

Successful response:

```json
{
  "buffer": {
    "buffer_id": "buf_...",
    "name": "*Mother Status*",
    "state": "live",
    "major_mode": "table",
    "minor_modes": ["pinned"]
  },
  "payload": {
    "frame": {
      "protocol": "patina:view-buffer",
      "version": 1,
      "payload_contract": "framed_json",
      "shape_id": "mother.status.default",
      "buffer_id": "buf_..."
    },
    "json": {
      "rows": []
    }
  }
}
```

Missing data response:

```json
{
  "error": "observability_gap",
  "missing_fact_path": "mother.status.children_total",
  "gap_id": "gap_..."
}
```

## WIT-framed JSON compromise

This slice does not need a full typed WIT display world. It should still encode the intended boundary:

- stable envelope fields
- flexible JSON payload
- shape id/version
- buffer id
- maturity/payload contract metadata

This keeps us aligned with the Allium rule that JSON is flexible early, while WIT frames the boundary and mature pieces may later become typed.

## Allium obligation trace

Tests should include comments or names that make traceability obvious, e.g.

```rust
// obligation: rule-success.OpenLiveBufferWhenRequiredFactsAreObserved
#[test]
fn opens_live_buffer_when_required_facts_are_observed() { ... }

// obligation: rule-success.RecordObservabilityGapWhenRequiredFactIsMissing
#[test]
fn records_gap_and_refuses_buffer_when_required_fact_missing() { ... }
```

## Read-before-write checklist

Before the first code edit, inspect:

- `mother/src/http_routes.rs` for route-table wiring and auth behavior.
- `mother/src/http_api.rs` and sibling modules for API handler structure.
- `src/commands/mother/daemon/dispatch.rs` for `ApiRuntime` implementation patterns.
- Mother state/runtime store modules for persistence conventions.
- Existing tests in `mother/src/http_api/tests/` and route tests for deterministic style.

Record any discovered boundary mismatch in this DESIGN or a linked fix spec before coding around it.

## Read-before-write findings

Read pass completed before implementation code edits:

- `mother/src/http_routes.rs`: routes are wired through `RouteTable` handler fields and guarded per route with bearer auth when `require_auth` is enabled. View-buffer routes should follow the same explicit match-arm pattern and have deterministic 404 behavior when absent.
- `mother/src/http_api.rs`: API modules use small per-domain traits over `ApiRuntime` (`HealthApi`, `BridgeApi`, etc.) and `build_route_table` clones the runtime per handler. View-buffer should add one focused API trait/module rather than expanding unrelated handlers.
- `src/commands/mother/daemon/dispatch.rs`: `ServerState` implements `ApiRuntime`; health details already expose suitable observed facts for the first proof shape. The first catalog should derive from existing health/readiness data rather than creating new observability.
- `mother/src/state/mod.rs`: Mother state uses SQLite schema initialization inside `MotherRuntimeStore::init_schema`, with additive `CREATE TABLE IF NOT EXISTS` migrations and narrow public store methods. Buffer persistence should follow that store style if persisted in state.db.
- `mother/src/http_api/health.rs`: health response construction is a good source for the minimal `mother.status.*` catalog facts.
- `mother/src/http_api/tests/mod.rs` and `mother/src/http_routes.rs` tests: deterministic stub-runtime and route-table tests are the preferred proof style for this slice.

No boundary mismatch found that requires a separate fix spec before implementation.

## First implementation order

1. Add model types and serialization.
2. Add minimal catalog with Mother status facts.
3. Add service behavior for open/list/connect/disconnect/kill.
4. Add observability gap recording.
5. Add HTTP/control-plane routes.
6. Add CLI/debug access only if useful for verification.
7. Add tests and Allium obligation references.

## Risks

### Risk: reintroducing hardcoded dashboards

Mitigation: built-in proof shape must behave like a shape record with explicit requirements, not like custom HTML/rendering code.

### Risk: premature frontend design

Mitigation: no SvelteKit in this slice. Return framed JSON only.

### Risk: pretending data exists

Mitigation: every required fact must resolve through the catalog. Missing facts produce gaps and no buffer.

### Risk: confusing buffer state with window state

Mitigation: disconnecting a window must not kill the buffer. Only kill-buffer changes buffer terminal state.
