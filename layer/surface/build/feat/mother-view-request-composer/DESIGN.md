# Design: Mother View Request Composer

## Why This Design

[[mother-view-shape-library]] made Mother-owned shapes selectable and durable. The next Allium obligation is not a renderer; it is the request-composition layer that captures what a user asked for, records how an agent matched that request to a shape, and opens a buffer only when Mother can validate the backing facts.

This design keeps the contract narrow: agents propose structured matches; Mother validates active shape state, confidence thresholds, and backing observability. Mother does not parse natural language or execute agent-generated UI code in this slice.

Guiding beliefs:

- [[allium-as-agent-display-lisp]] — request composition selects/adapts structured local display metadata, not arbitrary UI source code.
- [[allium-as-business-backlog]] — this spec implements the next bounded Allium obligation and splits the rest explicitly.

## Build Target

Build durable Mother request-composition support:

- persist `DisplayRequest` records;
- persist `ShapeMatch` records;
- expose a structured compose API;
- accept explicit/exact shape proposals from agents/callers;
- open buffers through the existing shape-library path when safe;
- record unable/follow-on outcomes when a match cannot safely open a buffer.

The target is backend/runtime only. Renderer UX remains future work.

## Allium/code alignment pass

### Allium slice

| Allium construct/rule | This spec responsibility |
|---|---|
| `DisplayRequest` | Persist request id, raw request, user id, agent id, timestamps, and outcome |
| `ShapeMatch` | Persist request id, optional shape id, match kind, and confidence |
| `CaptureUserDisplayRequest` | Structured compose API creates a pending request for non-empty raw text |
| `SelectExplicitUserRequestedShape` | Accept an active explicit shape proposal and request/open requirements checking |
| `SelectExactShapeMatch` | Accept an active exact shape proposal when confidence >= `0.60` |
| `OpenLiveBufferWhenRequiredFactsAreObserved` | Reuse existing shape-library open path |
| `RecordObservabilityGapWhenRequiredFactIsMissing` | Reuse existing open-buffer gap behavior |

### Deferred Allium work

| Allium construct/rule | Follow-on spec |
|---|---|
| `AdaptSimilarShapeWhenNoExactShapeExists` | [[mother-view-shape-adaptation]] or later extension of this composer |
| `CreateInitialShapeWhenNoShapeMatches` | [[mother-view-initial-shape-creation]] or later extension of this composer |
| `ReplaceBufferWhenUserRevisesViewShape` / `ViewShapeRevision` | [[mother-view-buffer-revision]] |
| `LinkObservabilityGapToWorkItem` / `ResolveObservabilityGapWhenFactBecomesObserved` | [[mother-view-observability-workflow]] |
| `ViewMaturationEvent` / `ObservabilityImprovementArtifact` | [[mother-view-maturation]] |
| `FrameBufferSurface` renderer behavior | [[mother-sveltekit-frame]] first, TUI/Emacs later |

### Current code seams read

- `mother/src/view_buffer/model.rs` already has shape, requirement, buffer, window, frame, and gap models. Add request/match models here to keep the display domain cohesive.
- `mother/src/view_buffer/store.rs` already persists shapes, requirements, buffers, windows, frames, and gaps through narrow free functions. Add request/match tables and functions here.
- `mother/src/view_buffer/service.rs` already opens buffers from a provided active shape library with fail-closed catalog validation. Request composition should call this path instead of duplicating requirement checks.
- `mother/src/http_api/view_buffer.rs` already owns view-buffer and view-shape handlers. Request-composition handlers can live here initially to avoid a parallel display route module.
- `mother/src/http_api.rs` has `ApiRuntime` plus a narrow `ViewBufferApi` adapter trait. Add request-composition methods to the same display API surface unless it becomes too large.
- `mother/src/http_routes.rs` centralizes auth-guarded route matching. Add `/api/view-requests/compose` and list/read routes there.
- `src/commands/mother/daemon/dispatch.rs` now seeds built-in shapes, loads persisted shapes, and opens buffers through `ViewBufferService::with_catalog_and_shapes`. Composition should reuse the same daemon helper path.
- `mother/src/state/mod.rs` exposes thin wrappers around view-buffer store functions. Add request/match wrappers here.

## Resolved Decisions

- This slice is not a natural-language matcher. Agents/callers provide proposed matches as structured data.
- Explicit user choices require `shape.active` and do not require a confidence threshold beyond being valid structured input.
- Exact matches require `shape.active` and `confidence >= 0.60`, matching Allium config.
- Similar and none matches are persisted but do not open buffers in this slice.
- Low-confidence exact matches return an unable/fail-closed outcome and do not open buffers.
- Missing facts or unavailable sources are delegated to existing open-buffer gap behavior, preserving no-fake-data semantics.

## Direct Code Targets

- `mother/src/view_buffer/model.rs`
- `mother/src/view_buffer/store.rs`
- `mother/src/view_buffer/service.rs`
- `mother/src/view_buffer/mod.rs`
- `mother/src/http_api/view_buffer.rs`
- `mother/src/http_api.rs`
- `mother/src/http_routes.rs`
- `mother/src/http_api/tests/mod.rs`
- `mother/src/state/mod.rs`
- `src/commands/mother/daemon/dispatch.rs`

## Proposed Rust model

```rust
DisplayRequest {
    request_id: String,
    user_id: String,
    agent_id: String,
    raw_request: String,
    requested_at: DateTime<Utc>,
    outcome: DisplayRequestOutcome,
}

ShapeMatch {
    request_id: String,
    shape_id: Option<String>,
    match_kind: ShapeMatchKind,
    confidence: f64,
}
```

`DisplayRequestOutcome` should serialize in Allium-aligned snake case:

- `pending`
- `buffer_opened`
- `observability_gap_reported`
- `unable`

`ShapeMatchKind` should serialize in Allium-aligned snake case:

- `exact`
- `explicit_user_choice`
- `similar`
- `none`

## API sketch

```text
GET  /api/view-requests
GET  /api/view-requests/<request_id>
POST /api/view-requests/compose
```

Composition request:

```json
{
  "user_id": "local-user",
  "agent_id": "pi",
  "raw_request": "show mother status",
  "proposed_match": {
    "shape_id": "mother.status.default",
    "match_kind": "explicit-user-choice",
    "confidence": 1.0
  }
}
```

Composition response should include the persisted request, optional shape match, and either opened-buffer payload, observability gap, or an unable/follow-on reason.

## Verification Plan

Run at minimum:

```bash
cargo check -q
cargo test -q -p mother view_request
cargo test -q -p mother view_buffer
patina spec check mother-view-request-composer --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

Tests should cover:

- non-empty request capture;
- blank raw request rejection;
- request/match persistence;
- explicit active shape opening;
- exact active shape opening above confidence threshold;
- exact low confidence not opening;
- inactive/missing shape not opening;
- similar/none outcomes recorded without invented shape data;
- missing required fact still records an observability gap.

## Implementation Notes

### `mvrc1-request-model`

Implemented the Allium request-composer vocabulary in `mother/src/view_buffer/model.rs`:

- `DisplayRequestOutcome::{Pending, BufferOpened, ObservabilityGapReported, Unable}`;
- `ShapeMatchKind::{Exact, ExplicitUserChoice, Similar, None}`;
- `DisplayRequest` with request id, user id, agent id, raw request, request timestamp, and outcome;
- `ShapeMatch` with request id, optional shape id, match kind, and confidence;
- snake_case serde representation for request outcomes and match kinds, matching the Allium enum spellings.

This is model-only; persistence, APIs, and composition behavior remain later exit criteria.

### `mvrc2-request-persistence`

Persisted request-composer state through the existing Mother runtime store seam:

- added `mother_view_display_requests` and `mother_view_shape_matches` tables;
- added deterministic save/get/list/update functions for display requests;
- added deterministic save/get/list functions for one shape match per request;
- added `MotherRuntimeStore` wrappers for request/match operations;
- added persistence tests covering request capture, match round-trip, outcome updates, and missing update fail-closed behavior.

This still does not expose composition APIs or open buffers from requests; those remain later exit criteria.

### `mvrc3-compose-api` / `mvrc4-explicit-exact-open` / `mvrc5-fail-closed-outcomes`

Added structured request composition across service, HTTP, route, runtime, and daemon seams:

- added `ComposeViewRequest`, `ProposedShapeMatch`, and `ComposedViewRequest` DTOs with `deny_unknown_fields` on inbound request payloads;
- added `ViewBufferService::compose_request` to capture a request, persist a proposed match in the result, validate match kind/confidence/active shape state, and delegate safe opens to `open_buffer`;
- added `POST /api/view-requests/compose`, `GET /api/view-requests`, and `GET /api/view-requests/<request_id>`;
- wired `ApiRuntime`, `ViewBufferApi`, `RouteTable`, daemon dispatch, and bootstrap/test route tables;
- daemon composition persists the display request, shape match, opened buffer, or observability gap through `MotherRuntimeStore`;
- explicit matches open active shapes without a confidence threshold;
- exact matches require `confidence >= 0.60`;
- blank requests, low-confidence exact matches, similar matches, none matches, missing shape ids, unknown shapes, and inactive shapes do not open buffers or invent data.

### `mvrc6-tests-and-trace`

Completed deterministic test and trace coverage for the implemented request-composer slice:

- model vocabulary and serde tests in `mother/src/view_buffer/model.rs`;
- request/match persistence tests in `mother/src/state/mod.rs`;
- compose handler success and blank-request rejection in `mother/src/http_api/tests/mod.rs`;
- route/auth coverage in `mother/src/http_routes.rs`;
- service tests for explicit open, exact open at threshold, missing required fact observability gap, missing/inactive shape refusal, low-confidence exact refusal, similar/no-match refusal, and existing view-buffer guardrails in `mother/src/view_buffer/service.rs`.

All relevant tests carry Allium/spec obligation comments.

## Commits

No implementation commits yet. This design prepared the spec for implementation slices.

## Build Readiness

Ready to promote as the next implementation spec. The Allium/code alignment pass is recorded here and `mvrc0-allium-code-alignment` is checked.

## Open Questions

None blocking this slice. The semantics of similar-shape adaptation and initial shape creation are intentionally deferred because they need shape-edit rules beyond safe selection.
