# Design: Mother View Initial Shape Creation

## Why This Design

The request composer currently fails closed for `ShapeMatchKind::None`. That protects users from fake displays, but it also stops the Allium flow at the point where an initial shape should be created or requested.

This design keeps creation structured and Mother-owned. Agents can propose a shape, but Mother validates the proposal against its data catalog and persists only metadata it can justify. Mother does not parse natural language and does not accept generated UI code.

Guiding beliefs:

- [[allium-as-agent-display-lisp]] — shape creation is structured local display metadata, not generated application code.
- [[allium-as-business-backlog]] — this slice implements the next explicit Allium obligation under [[mother-view-composer]].

## Build Target

Build no-match initial-shape creation as a backend/runtime slice:

- accept structured `proposed_initial_shape` input on display composition;
- only activate it when `proposed_match.match_kind = none`;
- validate title, modes, and explicit requirements;
- require proposed required fact paths to exist in `DataCatalog`;
- create an exploratory active `ViewShape`;
- persist through the existing shape-library store;
- return structured creation data to the caller;
- do not open a buffer automatically.

## Allium/code alignment pass

### Allium slice

| Allium construct/rule | This spec responsibility |
|---|---|
| `CreateInitialShapeWhenNoShapeMatches` | Convert no-match composition plus structured proposal into a shape-creation artifact/shape |
| `ShapeMatch(match_kind = none)` | Persist the no-match request and confidence 0.0 |
| `ViewShapeCreationRequested` | Represent through structured response data and persisted exploratory shape |
| `ViewRequirement` | Require explicit catalog-backed requirements; do not invent facts |

### Current code seams read

- `mother/src/view_buffer/model.rs` already has `DisplayRequest`, `ShapeMatch`, `ShapeMatchKind::None`, `ViewShape`, `ViewRequirement`, and exploratory maturity.
- `mother/src/view_buffer/service.rs` currently returns unable for `ShapeMatchKind::None` with reason `no usable shape matched request`. This is the exact seam to extend.
- `mother/src/view_buffer/catalog.rs` exposes `DataCatalog::fact`, `value`, and `observed_required_fact`, enough to validate proposed required facts against catalogued Mother data.
- `mother/src/view_buffer/store.rs` already persists `ViewShape` and `ViewRequirement` records. No new table is required for the first creation slice.
- `mother/src/state/mod.rs` exposes `upsert_view_shape`, request, and match wrappers.
- `src/commands/mother/daemon/dispatch.rs` already persists composed requests/matches/open outcomes and adapted shapes. It should also persist created initial shapes.
- `mother/src/http_api/view_buffer.rs` returns `ComposedViewRequest` from `POST /api/view-requests/compose`; optional JSON fields keep this backward-compatible.

### Deferred Allium work

| Deferred behavior | Follow-on spec |
|---|---|
| User-facing confirmation/editing of initial shapes | [[mother-view-request-ux]] |
| Creating shapes that require missing facts plus observability work | [[mother-view-observability-workflow]] |
| User corrections and replacement | [[mother-view-buffer-revision]] |
| Maturation of created shapes | [[mother-view-maturation]] |
| Renderer display | [[mother-sveltekit-frame]] |

## Resolved Decisions

- Add optional `proposed_initial_shape` to `ComposeViewRequest` rather than overloading `ProposedShapeMatch`.
- Add a `ViewShapeCreation` result type parallel to `ViewShapeAdaptation`.
- Add optional `shape_creation` and `created_shape` fields to `ComposedViewRequest`.
- Do not add a new `DisplayRequestOutcome`; persist `unable` and represent creation with structured response data and persisted shape/match rows.
- Require at least one required requirement. Optional-only shapes are too ambiguous for this first slice.
- Validate only required requirements against `DataCatalog`; optional requirements may be allowed later, but this slice should keep the proposal minimal.

## Direct Code Targets

- `mother/src/view_buffer/model.rs`
- `mother/src/view_buffer/service.rs`
- `mother/src/view_buffer/mod.rs`
- `mother/src/view_buffer/catalog.rs`
- `mother/src/state/mod.rs`
- `mother/src/http_api/tests/mod.rs`
- `src/commands/mother/daemon/dispatch.rs`
- `src/commands/mother/daemon/tests/mod.rs`

Likely no schema change is required because existing shape/request/match persistence is sufficient.

## Proposed Runtime Behavior

For `match_kind = none` composition:

1. Reject if `proposed_initial_shape` is absent.
2. Reject if title is blank.
3. Reject if no required requirements are provided.
4. Reject if any required requirement has blank fact path or purpose.
5. Reject if any required fact path is absent from `DataCatalog`.
6. Create shape:
   - `shape_id = initial::<request_id>::<uuid>`
   - `title = proposed title`
   - `source_ref = local-allium-view-library`
   - `scope = mother_user` unless a later API explicitly supports a narrower scope
   - `version = 1`
   - `active = true`
   - `major_mode = proposed major mode`
   - `minor_modes = proposed minor modes`
   - `maturity = exploratory`
   - `payload_contract = framed_json`
   - `payload_version = 1`
   - `requirements = proposed requirements`
7. Return composed result with request outcome `unable`, persisted no-match `ShapeMatch`, `shape_creation`, and `created_shape`.
8. Daemon persists the created shape.

## Verification Plan

Run at minimum:

```bash
cargo check -q
cargo test -q -p mother view_initial_shape_creation
cargo test -q -p mother view_request
cargo test -q -p mother view_buffer
patina spec check mother-view-initial-shape-creation --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

Tests should cover:

- successful no-match initial shape creation;
- created shape metadata and requirements;
- daemon/store persistence of created shape;
- HTTP response shape;
- absent proposal refusal;
- blank title refusal;
- empty required requirement refusal;
- missing catalog fact refusal;
- no buffer opened during creation.

## Commits

No implementation commits yet. This design prepares the spec for implementation slices.

## Build Readiness

Ready to promote as the next [[mother-view-composer]] implementation slice. The Allium/code alignment pass is recorded and `mvisc0-allium-code-alignment` is checked.

## Open Questions

None blocking this slice. Richer user confirmation and observability work-item creation are intentionally deferred.
