# Design: Mother View Shape Library

## Design intent

`mother-view-buffer-runtime` made buffers real. This spec makes shapes real.

The Allium target says agents create, adapt, and revise view shapes instead of generating arbitrary UI code. This slice implements the durable shape-library substrate needed before request matching, revisions, maturation, or SvelteKit frames can be honest.

## Core value anchors

- `layer/core/spec-driven-design.md` — this spec is the bounded contract for the next slice.
- `layer/core/dependable-rust.md` — keep shape storage and API semantics explicit and small.
- `layer/core/unix-philosophy.md` — separate shape library, buffer lifecycle, catalog validation, and rendering.
- `layer/core/adapter-pattern.md` — no speculative renderer/compiler traits before real boundaries exist.
- `layer/core/values/contract-first-execution.md` — shapes declare backing contracts; Mother validates before opening buffers.

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

## Allium slice map

### In this spec

| Allium construct | This spec responsibility |
|---|---|
| `ViewShape` | Persist and expose shape records |
| `ViewRequirement` | Persist and validate required catalog facts |
| `MotherDisplayContext.shapes` | Represent shapes as Mother-owned library data |
| `SelectExplicitUserRequestedShape` | Enable explicit shape-id selection as a stored-shape lookup |
| `OpenLiveBufferWhenRequiredFactsAreObserved` | Open buffers from library shapes |
| `RecordObservabilityGapWhenRequiredFactIsMissing` | Preserve missing-data gap behavior for library shapes |

### Deferred follow-on specs

| Allium construct/rule | Follow-on spec |
|---|---|
| `CaptureUserDisplayRequest` | `mother-view-request-composer` |
| `SelectExactShapeMatch` / `AdaptSimilarShapeWhenNoExactShapeExists` / `CreateInitialShapeWhenNoShapeMatches` | `mother-view-request-composer` |
| `ReplaceBufferWhenUserRevisesViewShape` / `ViewShapeRevision` | `mother-view-buffer-revision` |
| `LinkObservabilityGapToWorkItem` / `ResolveObservabilityGapWhenFactBecomesObserved` | `mother-view-observability-workflow` |
| `ViewMaturationEvent` / `ObservabilityImprovementArtifact` | `mother-view-maturation` |
| `FrameBufferSurface` renderer behavior | `mother-sveltekit-frame` first, TUI/Emacs later |

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

If the shape exists but required facts are missing or source is unavailable, preserve current gap behavior.

## Read-before-write checklist

Before implementation, read and record findings for:

- `mother/src/view_buffer/model.rs`
- `mother/src/view_buffer/store.rs`
- `mother/src/view_buffer/service.rs`
- `mother/src/http_api/view_buffer.rs`
- `mother/src/http_routes.rs`
- `src/commands/mother/daemon/dispatch.rs`
- `mother/src/state/mod.rs`

## Risks

### Risk: shape library becomes hidden UI code

Mitigation: accept structured shape metadata only. Renderer-specific code generation remains forbidden.

### Risk: scope creep into agent matching

Mitigation: support explicit shape-id selection only. Natural-language matching/adaptation gets its own spec.

### Risk: breaking existing proof buffer

Mitigation: seed `mother.status.default` and keep the same fact guardrails and framed payload behavior.

### Risk: storing shapes without validating requirements

Mitigation: upsert may store inactive draft shapes with unobserved requirements, but opening requires observed facts and available sources.
