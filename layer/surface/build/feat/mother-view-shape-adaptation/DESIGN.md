# Design: Mother View Shape Adaptation

## Why This Design

[[mother-view-request-composer]] intentionally made `similar` matches safe but non-operative: they persist as unable/follow-on states and do not create data. The Allium target now points to the next step: turn a sufficiently confident similar-shape match into a new exploratory `ViewShape`.

This design keeps adaptation inside Mother-owned structured metadata. Agents can propose the precedent and confidence; Mother creates a controlled shape record. No executable UI code is accepted.

Guiding beliefs:

- [[allium-as-agent-display-lisp]] — adaptation edits structured local display metadata, not generated UI source.
- [[allium-as-business-backlog]] — this slice implements the next explicit Allium obligation after request composition.

## Build Target

Build similar-shape adaptation as a backend/runtime slice:

- detect `ShapeMatchKind::Similar` above threshold;
- resolve the active precedent shape;
- create an exploratory adapted `ViewShape`;
- persist the adapted shape through the existing shape-library store;
- return structured adaptation data to the caller;
- do not open a buffer automatically.

## Allium/code alignment pass

### Allium slice

| Allium construct/rule | This spec responsibility |
|---|---|
| `AdaptSimilarShapeWhenNoExactShapeExists` | Create an exploratory adapted shape from a similar precedent |
| `ShapeMatch(match_kind = similar)` | Persist the proposed precedent and confidence |
| `ViewShape.created` | Create a new adapted shape with copied display contract/modes and exploratory maturity |
| `ViewShapeAdaptationRequested` | Represent through structured response data and persisted adapted shape |

### Current code seams read

- `mother/src/view_buffer/model.rs` already has `DisplayRequest`, `ShapeMatch`, `ShapeMatchKind::Similar`, and `ViewShapeMaturity::Exploratory`.
- `mother/src/view_buffer/service.rs` currently returns unable for `ShapeMatchKind::Similar` with reason `similar shape adaptation is deferred`. This is the exact seam to replace.
- `mother/src/view_buffer/store.rs` already persists `ViewShape` and `ShapeMatch` records. No new tables should be needed for the first adaptation slice unless we choose to preserve richer adaptation provenance.
- `mother/src/state/mod.rs` already exposes `upsert_view_shape`, request, and match wrappers.
- `src/commands/mother/daemon/dispatch.rs` already persists composed requests/matches/open outcomes. It should additionally persist any adapted shape returned by composition.
- `mother/src/http_api/view_buffer.rs` already returns `ComposedViewRequest` from `POST /api/view-requests/compose`; adding an optional adapted-shape field can remain backward-compatible JSON.

### Deferred Allium work

| Deferred behavior | Follow-on spec |
|---|---|
| No-match initial shape creation | [[mother-view-initial-shape-creation]] |
| User-facing adaptation confirmation/editing | [[mother-view-request-ux]] |
| Opening adapted shapes after user/agent confirmation | Later request-composer UX/API slice |
| User corrections and replacement | [[mother-view-buffer-revision]] |
| Maturation of adapted shapes | [[mother-view-maturation]] |
| Renderer display | [[mother-sveltekit-frame]] |

## Resolved Decisions

- Do not add a new `DisplayRequestOutcome` for adaptation because Allium v1 does not define one.
- Represent adaptation with `ComposedViewRequest.adapted_shape` or equivalent response data, plus persisted `ShapeMatch` and `ViewShape` rows.
- Adapted shape id may use a sanitized precedent id plus generated suffix; deterministic exact id is not required because adaptation is a new artifact.
- Copy precedent requirements in this first slice. Requirement editing/adaptation belongs to later UX or shape-creation work.
- Adapted shapes are active and exploratory by default, matching Allium's `ViewShape.created` rule.

## Direct Code Targets

- `mother/src/view_buffer/model.rs`
- `mother/src/view_buffer/service.rs`
- `mother/src/view_buffer/mod.rs`
- `src/commands/mother/daemon/dispatch.rs`
- `mother/src/http_api/tests/mod.rs`
- `mother/src/state/mod.rs`

Likely no schema change is required for the first slice because existing shape/request/match persistence is sufficient.

## Proposed Runtime Behavior

For `similar` composition:

1. Reject if confidence is below `SHAPE_MATCH_CONFIDENCE_THRESHOLD`.
2. Reject if `shape_id` is missing.
3. Reject if precedent shape is unknown or inactive.
4. Clone precedent modes, requirements, payload contract, and optional context projections.
5. Set adapted fields:
   - `shape_id = <precedent>::adapted::<uuid>`
   - `title = Adapted <precedent.title>`
   - `source_ref = local-allium-view-library`
   - `version = 1`
   - `active = true`
   - `maturity = exploratory`
   - `replaced_by = null`
6. Return composed result with request outcome `unable`, persisted shape match, and `adapted_shape` present.
7. Daemon persists the adapted shape.

## Verification Plan

Run at minimum:

```bash
cargo check -q
cargo test -q -p mother view_shape_adaptation
cargo test -q -p mother view_request
cargo test -q -p mother view_buffer
patina spec check mother-view-shape-adaptation --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

Tests should cover:

- successful similar adaptation above threshold;
- adapted shape metadata and copied requirements;
- daemon/store persistence of adapted shape;
- low-confidence similar match refusal;
- missing precedent shape refusal;
- inactive precedent shape refusal;
- no buffer opened during adaptation.

## Commits

1. `feat: model mother view shape adaptation` — adds structured `ViewShapeAdaptation` result data and wires `ComposedViewRequest` to carry a non-opening adaptation result for `mvsa1-adaptation-model`.
2. `feat: adapt similar mother view shapes` — creates exploratory adapted shapes from confident similar matches, returns the adaptation response, persists adapted shapes through daemon/store seams, and verifies fail-closed guardrails.

## Build Readiness

Implemented and ready to complete. All exit criteria are checked, including follow-on backlog boundaries.

## Open Questions

None blocking this slice. Richer requirement editing and user confirmation UX are intentionally deferred.
