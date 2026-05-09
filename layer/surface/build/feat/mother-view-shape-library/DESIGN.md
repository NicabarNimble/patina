# Design: Mother View Shape Library

## Why This Design

`mother-view-buffer-runtime` made buffers real. This spec makes shapes real.

The Allium target says agents create, adapt, and revise view shapes instead of generating arbitrary UI code. This slice implements the durable shape-library substrate needed before request matching, revisions, maturation, or SvelteKit frames can be honest.

## Core value anchors

- `layer/core/spec-driven-design.md` — this spec is the bounded contract for the next slice.
- `layer/core/dependable-rust.md` — keep shape storage and API semantics explicit and small.
- `layer/core/unix-philosophy.md` — separate shape library, buffer lifecycle, catalog validation, and rendering.
- `layer/core/adapter-pattern.md` — no speculative renderer/compiler traits before real boundaries exist.
- `layer/core/values/contract-first-execution.md` — shapes declare backing contracts; Mother validates before opening buffers.

## Build Target

Build a Mother-owned persistent view-shape library that stores structured `ViewShape` and `ViewRequirement` records, exposes narrow control-plane APIs, seeds `mother.status.default` as data, and lets `open_buffer(shape_id)` resolve active persisted shapes before built-in fallback.

The target is backend/runtime only. SvelteKit, TUI, and Emacs remain future frames/renderers.

## Relationship to completed kernel

Completed spec:

- `[[mother-view-buffer-runtime]]`

Kernel capabilities now available:

- persisted buffers/frames/windows/gaps;
- minimal data catalog over Mother status facts;
- fail-closed buffer opening;
- WIT-framed JSON payloads;
- buffer API routes.

This spec should reuse those seams rather than introduce a parallel display stack.

Guiding belief: [[allium-as-business-backlog]]. Passing a scoped implementation spec does not mean the full Allium target is done; this slice explicitly converts the next Allium obligation into durable Mother data.

## Resolved Decisions

- Mother owns view shapes; renderers do not own or mutate shape source-of-truth state.
- Shapes are structured metadata/guardrails, not executable UI code.
- Explicit shape-id lookup is in scope; natural-language request matching and adaptation are deferred.
- `required = true` requirements block opening; optional requirements can be stored but cannot invent data or block required-data views.
- Allium `vision` and `project` references are stored as `vision_id` and `project_uid` projections in this slice.
- The existing `mother.status.default` proof shape must become seeded library data so built-in-only lookup is no longer the only path.

## Allium slice map

### In this spec

| Allium construct | This spec responsibility |
|---|---|
| `ViewShape` | Persist and expose shape records |
| `ViewRequirement` | Persist and validate required catalog facts |
| `MotherDisplayContext.shapes` | Represent shapes as Mother-owned library data |
| `SelectExplicitUserRequestedShape` | Enable explicit shape-id selection as a stored-shape lookup |
| `OpenLiveBufferWhenRequiredFactsAreObserved` | Open buffers from library shapes after required requirements are observed and their sources are available |
| `RecordObservabilityGapWhenRequiredFactIsMissing` | Preserve missing-data gap behavior for library shapes when a required requirement is missing/unavailable |

### Deferred follow-on specs

| Allium construct/rule | Follow-on spec |
|---|---|
| `CaptureUserDisplayRequest` | `mother-view-request-composer` |
| `SelectExactShapeMatch` / `AdaptSimilarShapeWhenNoExactShapeExists` / `CreateInitialShapeWhenNoShapeMatches` | `mother-view-request-composer` |
| `ReplaceBufferWhenUserRevisesViewShape` / `ViewShapeRevision` | `mother-view-buffer-revision` |
| `LinkObservabilityGapToWorkItem` / `ResolveObservabilityGapWhenFactBecomesObserved` | `mother-view-observability-workflow` |
| `ViewMaturationEvent` / `ObservabilityImprovementArtifact` | `mother-view-maturation` |
| `FrameBufferSurface` renderer behavior | `mother-sveltekit-frame` first, TUI/Emacs later |

## Direct Code Targets

Primary files expected to change:

- `mother/src/view_buffer/model.rs`
- `mother/src/view_buffer/store.rs`
- `mother/src/view_buffer/service.rs`
- `mother/src/view_buffer/catalog.rs`
- `mother/src/view_buffer/mod.rs`
- `mother/src/http_api/view_buffer.rs` or a sibling `mother/src/http_api/view_shape.rs`
- `mother/src/http_api.rs`
- `mother/src/http_routes.rs`
- `src/commands/mother/daemon/dispatch.rs`
- `mother/src/state/mod.rs`
- `mother/src/http_api/tests/mod.rs`

## Proposed Rust layout

```text
mother/src/view_buffer/
  model.rs          # add shape maturity/source/project/vision/replaced_by fields
  shape_library.rs  # service-level shape library operations
  store.rs          # SQLite shape/requirement persistence
  service.rs        # open_buffer resolves library shapes
  payload.rs        # unchanged framed JSON envelope
```

HTTP/API additions may either live in the existing `mother/src/http_api/view_buffer.rs` or a sibling `view_shape.rs`; prefer the smallest coherent route module.

## Model additions

Existing `ViewShape` already has:

- `shape_id`
- `title`
- `scope`
- `version`
- `active`
- `major_mode`
- `minor_modes`
- `payload_contract`
- `payload_version`
- `requirements`

Needed for Allium alignment:

- `source_ref: String`
- `maturity: exploratory | candidate | stable | promoted`
- `vision_id: Option<String>`
- `project_uid: Option<String>`
- `replaced_by: Option<String>`

`vision_id` and `project_uid` are the Rust/SQLite storage projection of Allium's optional `vision: VisionContext?` and `project: ProjectContext?` relationships. They preserve identity without pulling full context records into this slice.

Derivations and display patterns can be represented as empty/preserved metadata in this slice only if useful, but their maturation behavior is deferred.

## Persistence sketch

```text
mother_view_shapes
  shape_id TEXT PRIMARY KEY
  title TEXT NOT NULL
  source_ref TEXT NOT NULL
  scope TEXT NOT NULL
  version INTEGER NOT NULL
  active INTEGER NOT NULL
  major_mode TEXT NOT NULL
  minor_modes_json TEXT NOT NULL
  maturity TEXT NOT NULL
  payload_contract TEXT NOT NULL
  payload_version INTEGER NOT NULL
  vision_id TEXT
  project_uid TEXT
  replaced_by TEXT
  created_at TEXT NOT NULL
  updated_at TEXT NOT NULL

mother_view_shape_requirements
  shape_id TEXT NOT NULL
  fact_path TEXT NOT NULL
  required INTEGER NOT NULL
  purpose TEXT NOT NULL
  PRIMARY KEY (shape_id, fact_path)
```

Store methods should stay narrow:

- `upsert_view_shape(shape)`
- `get_view_shape(shape_id)`
- `list_view_shapes()`
- `deactivate_view_shape(shape_id)`
- `seed_view_shape(shape)` or idempotent upsert for built-ins

Requirement semantics:

- `required = true` requirements block buffer opening unless the catalog fact is `observed` and the backing source is `available`.
- `required = false` requirements may be stored as optional enrichment metadata, but this slice must not invent optional values or let optional gaps block required-data views.
- The seeded `mother.status.default` shape should use required requirements only, matching the existing fail-closed proof behavior.

## API sketch

Candidate routes:

```text
GET  /api/view-shapes
GET  /api/view-shapes/<shape_id>
POST /api/view-shapes/upsert
POST /api/view-shapes/deactivate
```

Shape upsert accepts only structured JSON fields. It must not accept renderer-owned source code, HTML, Svelte components, shell commands, or arbitrary script fields.

## Open-buffer integration

`ViewBufferService::open_buffer(shape_id)` should resolve shape records in this order:

1. active persisted shape from Mother shape library;
2. seeded built-in/library default if no persisted record exists during bootstrap;
3. otherwise fail with unknown/inactive shape error.

If the shape exists but required facts are missing or source is unavailable, preserve current gap behavior. Optional requirements can be ignored for opening in this slice; renderer enrichment or optional-data display affordances belong to later shape-composition work.

## Read-before-write checklist

Before implementation, read and record findings for:

- [x] `mother/src/view_buffer/model.rs`
- [x] `mother/src/view_buffer/store.rs`
- [x] `mother/src/view_buffer/service.rs`
- [x] `mother/src/view_buffer/catalog.rs`
- [x] `mother/src/view_buffer/mod.rs`
- [x] `mother/src/view_buffer/payload.rs`
- [x] `mother/src/http_api/view_buffer.rs`
- [x] `mother/src/http_api.rs`
- [x] `mother/src/http_routes.rs`
- [x] `src/commands/mother/daemon/dispatch.rs`
- [x] `mother/src/state/mod.rs`
- [x] `mother/src/http_api/tests/mod.rs`

### Read-before-write findings

- `mother/src/view_buffer/model.rs` already contains most Allium-aligned enums and core records for buffers, frames, windows, gaps, catalog facts/sources, requirements, and proof `ViewShape`s. Missing shape-library fields are `source_ref`, `maturity`, `vision_id`, `project_uid`, and `replaced_by`; adding them to `ViewShape` is the first model change for `mvsl1-shape-model`.
- `ViewRequirement.required` already has the desired semantics in `ViewShape::requires_fact` and catalog validation: only required requirements are blockers. The shape-library implementation should preserve that behavior when persisting requirements.
- `mother/src/view_buffer/catalog.rs` cleanly separates `SourceAvailability` from `ObservationState`; `DataCatalog::observed_required_fact` already implements the Allium rule that a required fact must be observed and its source must be available. The shape library can reuse this without inventing data.
- `mother/src/view_buffer/service.rs` currently keeps `shapes` as an in-memory `BTreeMap` seeded with `mother_status_shape()`. That is the key seam to replace: construct the service with active persisted shapes, or inject a shape resolver/library, while keeping the same `OpenBufferOutcome::{Opened, ObservabilityGap}` contract.
- `mother_status_shape()` is still Rust-only built-in data. `mvsl5-proof-shapes-seeded` should move this representation into seedable library data while retaining the helper for bootstrap/test fixtures if useful.
- `mother/src/view_buffer/store.rs` initializes only buffers, frames, windows, and observability gaps. Shape persistence should extend this schema with `mother_view_shapes` and `mother_view_shape_requirements`, following the existing narrow free-function pattern and enum-as-kebab-string JSON helpers.
- `mother/src/state/mod.rs` calls `view_buffer::store::init_schema(conn)` from Mother runtime schema init and exposes thin `save_*/list_*` wrappers. Add shape wrappers here rather than introducing a second database owner.
- `mother/src/http_api/view_buffer.rs` uses small handler functions, generic API traits, `parse_json`, JSON wrappers, 400 for invalid JSON, 409 for observability gaps, and 500 for runtime errors. Shape APIs should follow this style; unknown/inactive shape behavior may need a typed error later if we want 404/409 instead of generic 500.
- `mother/src/http_api.rs` has one `ApiRuntime` plus narrower `ViewBufferApi` adapter trait and wires handlers through `build_route_table`. Shape API support should add a sibling narrow trait (`ViewShapeApi`) or extend view-buffer API only if it stays cohesive.
- `mother/src/http_routes.rs` centralizes auth-guarded route matching in `RouteTable`; add `/api/view-shapes` routes there and mirror route tests so auth behavior remains explicit.
- `src/commands/mother/daemon/dispatch.rs` currently reconstructs a fresh `ViewBufferService` per open request from live Mother health data, then persists only the resulting buffer or gap. Library shapes must be loaded from `runtime_store` before opening; otherwise persisted shapes will not affect daemon behavior.
- `mother/src/http_api/tests/mod.rs` uses a large `StubRuntime` implementing `ApiRuntime`. New shape API methods will require stub implementations and handler tests; keep tests deterministic by avoiding live daemon state.
- `mother/src/view_buffer/payload.rs` frames payloads from `Buffer` plus `ViewShape`. Adding shape metadata must not alter the WIT-style frame unless shape version/contract semantics change intentionally.

## Verification Plan

Run at minimum:

```bash
cargo check -q
cargo test -q -p mother view_buffer
cargo test -q -p mother view_shape
patina spec check mother-view-shape-library --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

Tests should cover:

- shape persistence list/get/upsert/deactivate;
- requirement persistence and optional-vs-required semantics;
- API shape list/read/upsert/deactivate;
- inactive/missing shape fail-closed behavior;
- opening a buffer from seeded and persisted library shapes;
- observability gaps for missing required facts or unavailable sources.

## Implementation Notes

### `mvsl1-shape-model`

Implemented the Allium-aligned shape model in `mother/src/view_buffer/model.rs`:

- added `ViewShapeMaturity::{Exploratory, Candidate, Stable, Promoted}` with kebab-case serde representation;
- added `ViewShape.source_ref`;
- added optional `vision_id`, `project_uid`, and `replaced_by` storage projections;
- updated `mother_status_shape()` to carry `source_ref = "local-allium-view-library"` and `maturity = Stable`;
- updated payload/model tests so the WIT-framed payload path still compiles against the enriched shape model.

This is model-only. Shape persistence, API exposure, seeded-library ownership, and library-based open lookup remain later exit criteria.

## Commits

No implementation commits yet. Promotion/polish changes prepared the spec for `mvsl0-read-before-write`. `mvsl1-shape-model` implementation should be committed as the next scalpel commit.

## Build Readiness

Ready to promote as the next implementation spec. Implementation must begin with the documented read-before-write pass, then proceed through the exit criteria in [[mother-view-shape-library]].

## Risks

### Risk: shape library becomes hidden UI code

Mitigation: accept structured shape metadata only. Renderer-specific code generation remains forbidden.

### Risk: scope creep into agent matching

Mitigation: support explicit shape-id selection only. Natural-language matching/adaptation gets its own spec.

### Risk: breaking existing proof buffer

Mitigation: seed `mother.status.default` and keep the same fact guardrails and framed payload behavior.

### Risk: storing shapes without validating requirements

Mitigation: upsert may store inactive draft shapes with unobserved requirements, but opening requires observed facts and available sources.
