---
type: feat
id: mother-view-request-ux
status: ready
created: 2026-05-09
target: mother-view-composer
release_bump: patch
sessions:
  origin: 20260508-144836-859149000
related:
- mother-view-composer
- layer/allium/mother/mother-view-composer-target.allium
- mother-view-request-composer
- mother-view-shape-adaptation
- mother-view-initial-shape-creation
beliefs:
- '[[allium-as-agent-display-lisp]]'
- '[[allium-as-business-backlog]]'
exit_criteria:
- id: mvru0-allium-code-alignment
  text: The slice starts from an Allium/code alignment pass over DisplayRequest, ShapeMatch, created/adapted shapes, non-opening semantics, existing open-buffer behavior, persistence, daemon, and HTTP seams.
  checked: true
- id: mvru1-detail-model
  text: Mother exposes a request-detail model that combines the persisted DisplayRequest with its ShapeMatch, optional shape creation/adaptation record, linked shape, and explicit next actions.
  checked: false
- id: mvru2-persist-request-artifacts
  text: Shape creation/adaptation artifacts are persisted with request ids so request inspection does not infer history from naming conventions or invent data.
  checked: false
- id: mvru3-detail-api
  text: HTTP callers can list and fetch request details without losing the existing basic request list/get APIs.
  checked: false
- id: mvru4-open-linked-shape-action
  text: HTTP callers can explicitly open only a shape linked to the request detail; missing, unlinked, inactive, or unobservable shapes fail closed.
  checked: false
- id: mvru5-non-mutating-history
  text: Opening a linked shape through request UX does not rewrite historical composed request outcomes; it returns an action result and persists only buffer/gap effects.
  checked: false
- id: mvru6-no-fake-data-guardrails
  text: Request UX displays only persisted Mother request/match/artifact/shape data and delegates buffer opening to existing catalog-backed open-buffer checks.
  checked: false
- id: mvru7-tests-and-trace
  text: Deterministic tests cover detail construction, persisted creation/adaptation artifacts, HTTP detail/open endpoints, fail-closed unlinked shapes, and Allium/spec obligation comments.
  checked: false
---
# feat: Mother View Request UX

> Expose user-facing request-composition flows and statuses over agents/renderers without moving buffer or shape ownership out of Mother.

## Problem

[[mother-view-request-composer]], [[mother-view-shape-adaptation]], and [[mother-view-initial-shape-creation]] now give Mother safe composition outcomes, but the user-facing workflow is still fragmented:

- exact/explicit matches may open immediately;
- similar matches create adapted exploratory shapes without opening buffers;
- no-match requests create initial exploratory shapes without opening buffers;
- unable/fail-closed outcomes have reasons only in the immediate compose response;
- `GET /api/view-requests/<id>` returns only the persisted `DisplayRequest`, not the match, created/adapted artifact, linked shape, or available next action.

This makes the backend correct but hard for agents/renderers to present. Callers need an inspectable workflow object that says, "what happened, what shape is involved, and what safe action is available next?"

## Goal

Implement a bounded request UX backend slice:

1. Persist request-linked shape creation/adaptation artifacts.
2. Build a Mother-owned request detail from persisted request, match, artifact, linked shape, and action data.
3. Expose request details over HTTP while preserving existing basic request APIs.
4. Provide an explicit open-linked-shape action for created/adapted/matched shapes.
5. Keep historical request outcomes immutable after composition.
6. Reuse existing open-buffer catalog checks so required data is never invented.
7. Fail closed for unknown requests, missing links, inactive shapes, unlinked shape ids, and observability gaps.

## Status

Ready for implementation after Allium/code alignment.

## Allium authority

This spec implements the user-facing workflow slice around:

- `layer/allium/mother/mother-view-composer-target.allium`
- `layer/allium/mother/mother-view-composer-target.plan.json`

Primary Allium entities used:

- `DisplayRequest`
- `ShapeMatch`
- `ViewShape`
- `ViewShapeAdaptationRequested`
- `ViewShapeCreationRequested`
- `Buffer`
- `ObservabilityGap`

Primary Allium rules respected:

- `CaptureUserDisplayRequest`
- `SelectExplicitUserRequestedShape`
- `SelectExactShapeMatch`
- `AdaptSimilarShapeWhenNoExactShapeExists`
- `CreateInitialShapeWhenNoShapeMatches`
- `OpenLiveBufferWhenRequiredFactsAreObserved`
- `RecordObservabilityGapWhenRequiredFactIsMissing`

## Non-Goals

- Do not parse natural language inside Mother.
- Do not let renderers own request, buffer, shape, or action state.
- Do not add arbitrary UI code, Svelte components, HTML, or executable renderer payloads.
- Do not auto-open adapted or created shapes.
- Do not mutate the original composition outcome when a later explicit action opens a linked shape.
- Do not implement shape revisions/replacements; that belongs to [[mother-view-buffer-revision]].
- Do not implement observability-work-item workflow; that belongs to [[mother-view-observability-workflow]].
- Do not implement renderer frames; that belongs to [[mother-sveltekit-frame]].

## Target Shape

A request detail should let agents/renderers display a structured flow, for example:

```json
{
  "request": {"request_id": "req_123", "outcome": "unable"},
  "shape_match": {"match_kind": "none", "shape_id": null},
  "shape_creation": {"created_shape_id": "initial::req_123::...", "opens_buffer": false},
  "created_shape": {"shape_id": "initial::req_123::...", "title": "Mother Runtime Summary"},
  "available_actions": [
    {"kind": "open_created_shape", "shape_id": "initial::req_123::...", "label": "Open created shape"}
  ]
}
```

Opening is an explicit request UX action:

```json
{
  "request_id": "req_123",
  "shape_id": "initial::req_123::..."
}
```

The action may create a buffer or report an observability gap, using the same open-buffer guardrails already used by exact/explicit composition.

## Solution

Extend the current Mother view request backend:

- add `ViewRequestDetail`, `ViewRequestAction`, and action-kind model types;
- persist `ViewShapeCreation` and `ViewShapeAdaptation` rows keyed by `request_id`;
- save those artifacts when composition creates/adapts shapes;
- build request details from persisted rows, not naming conventions;
- add detail APIs:
  - `GET /api/view-requests/details`
  - `GET /api/view-requests/<request_id>/detail`
- add explicit action API:
  - `POST /api/view-requests/open-shape`
- validate that a requested shape id is linked to the request detail before opening;
- call the existing `open_buffer` service path to enforce catalog-backed required facts;
- persist opened buffers or observability gaps as action effects;
- leave the original `DisplayRequest.outcome` unchanged.

## Implementation Order

1. Add request detail/action model types.
2. Add store tables and functions for shape creation/adaptation artifacts.
3. Save artifacts in daemon composition persistence.
4. Add request-detail construction and linked-shape validation helpers.
5. Wire HTTP routes for details and open-shape action.
6. Add deterministic service/store/daemon/HTTP tests.
7. Update umbrella tracking after release.

## Resolved Decisions

- Request UX is backend workflow state, not renderer ownership.
- Created/adapted shapes remain non-opening composition results until a later explicit action.
- The later explicit action does not rewrite the historical composition outcome, because current Allium outcomes are terminal.
- Request detail must use persisted relations, not parse natural language or infer history from shape ids.
- Open action delegates to existing `OpenBufferRequest` behavior so observability gaps stay consistent.

## Verification

```bash
cargo check -q
cargo test -q -p mother view_request_ux
cargo test -q -p mother view_request
cargo test -q -p mother view_buffer
patina spec check mother-view-request-ux --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Exit Criteria

- [x] `mvru0-allium-code-alignment`
- [ ] `mvru1-detail-model`
- [ ] `mvru2-persist-request-artifacts`
- [ ] `mvru3-detail-api`
- [ ] `mvru4-open-linked-shape-action`
- [ ] `mvru5-non-mutating-history`
- [ ] `mvru6-no-fake-data-guardrails`
- [ ] `mvru7-tests-and-trace`

## Build Readiness

Ready to promote after the matching design document records code seams and implementation boundaries.
