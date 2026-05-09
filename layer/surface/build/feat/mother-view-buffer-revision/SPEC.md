---
type: feat
id: mother-view-buffer-revision
status: ready
created: 2026-05-09
target: mother-view-composer
release_bump: patch
sessions:
  origin: 20260508-144836-859149000
related:
- mother-view-composer
- layer/allium/mother/mother-view-composer-target.allium
- mother-view-shape-library
- mother-view-buffer-runtime
- mother-view-request-ux
beliefs:
- '[[allium-as-agent-display-lisp]]'
- '[[allium-as-business-backlog]]'
exit_criteria:
- id: mvbr0-allium-code-alignment
  text: The slice starts from an Allium/code alignment pass over ViewShapeRevision, UserCorrectsBufferView, buffer replacement states, shape replaced_by, persistence, daemon, and HTTP seams.
  checked: true
- id: mvbr1-revision-model
  text: Mother has structured revision request/result data for user corrections, including previous shape, revised shape, optional previous/replacement buffer ids, scope, origin, state, reason, and timestamps.
  checked: false
- id: mvbr2-catalog-guardrails
  text: Revised shapes that change requirements are validated against the Mother data catalog and fail closed for blank, uncatalogued, or unavailable required facts.
  checked: false
- id: mvbr3-shape-history
  text: Applying a revision creates a new active ViewShape version and marks the previous shape inactive with replaced_by pointing at the revised shape.
  checked: false
- id: mvbr4-buffer-replacement
  text: When a live/stale/blocked previous buffer is provided, Mother opens the revised shape and marks the previous buffer replaced with replacement linkage only after the replacement opens.
  checked: false
- id: mvbr5-persistence
  text: Shape revisions, previous/revised shape records, replaced buffers, replacement buffers, and observability gaps persist through the Mother store/daemon path.
  checked: false
- id: mvbr6-api
  text: HTTP callers can apply a structured view-shape revision and inspect persisted revisions without owning buffer or shape state.
  checked: false
- id: mvbr7-fail-closed-guardrails
  text: Unknown shapes, inactive shapes, empty reasons, no-op changes, unlinked/non-connectable buffers, and missing required facts do not replace buffers or invent data.
  checked: false
- id: mvbr8-tests-and-trace
  text: Deterministic tests cover successful shape revision, buffer replacement, persistence, API response shape, fail-closed behavior, and Allium/spec obligation comments.
  checked: false
---
# feat: Mother View Buffer Revision

> Apply user corrections to view shapes and replace existing Mother buffers while preserving revision history.

## Problem

Mother can now create, adapt, inspect, and explicitly open view shapes. But once a buffer is visible, users and agents need a safe way to say "this view is close, but wrong" without mutating history or letting a renderer invent state.

The Allium target includes `ViewShapeRevision` and `ReplaceBufferWhenUserRevisesViewShape`: user corrections should become durable Mother-owned revision records, previous buffers should enter `replaced`, and previous shapes should point to their replacement.

## Goal

Implement a bounded structured revision slice:

1. Accept a structured view-shape revision request.
2. Require an existing active previous shape.
3. Require a non-empty correction reason and at least one explicit structured change.
4. Validate changed requirements against observed/available catalog facts.
5. Create a new active revised `ViewShape` version.
6. Mark the previous shape inactive with `replaced_by = revised_shape_id`.
7. Optionally replace a provided live/stale/blocked previous buffer by opening the revised shape.
8. Persist the revision record, shape history, and buffer/gap effects.
9. Expose the workflow through Mother APIs.

## Status

Ready for implementation after Allium/code alignment.

## Allium authority

This spec implements the bounded revision/replacement slice of:

- `layer/allium/mother/mother-view-composer-target.allium`
- `layer/allium/mother/mother-view-composer-target.plan.json`

Primary Allium rule targeted:

- `ReplaceBufferWhenUserRevisesViewShape`

Primary Allium entities used:

- `ViewShapeRevision`
- `ViewShape`
- `Buffer`
- `ViewRequirement`
- `MotherDataCatalog`

## Non-Goals

- Do not parse natural language corrections inside Mother.
- Do not generate renderer code or executable UI payloads.
- Do not mutate historical revision records.
- Do not replace killed/replaced buffers.
- Do not mark a previous buffer replaced unless a replacement buffer opens successfully.
- Do not implement maturation; that belongs to [[mother-view-maturation]].
- Do not implement renderer frames; that belongs to [[mother-sveltekit-frame]].

## Target Shape

A structured correction request looks like:

```json
{
  "user_id": "local-user",
  "agent_id": "pi",
  "shape_id": "mother.status.default",
  "previous_buffer_id": "buf_mother_status_default_1",
  "revision_scope": "mother-user",
  "reason": "show readiness first",
  "title": "Mother Readiness",
  "minor_modes": ["pinned", "sorted"],
  "requirements": [
    {
      "fact_path": "mother.status.control_plane_ready",
      "required": true,
      "purpose": "display readiness first"
    }
  ]
}
```

Mother creates a revised shape, marks the previous shape replaced, and if a previous buffer is supplied and connectable, opens a replacement buffer before marking the previous buffer `replaced`.

## Solution

Extend the view-buffer backend with:

- `ViewShapeRevision` model and revision origin/state enums;
- `ReviseViewShapeRequest` and `RevisedViewShapeOutcome`;
- revision persistence table keyed by `revision_id`;
- `replacement_buffer_id` on `Buffer` to preserve buffer replacement history;
- a service method that validates structured changes, creates the revised shape, and optionally replaces a buffer;
- daemon persistence that saves revised/previous shapes, revisions, replaced buffers, replacement buffers, and gaps;
- APIs:
  - `GET /api/view-shape-revisions`
  - `GET /api/view-shape-revisions/<revision_id>`
  - `POST /api/view-shapes/revise`

## Implementation Order

1. Add revision model, request, and outcome types.
2. Add store schema/persistence for revisions and buffer replacement linkage.
3. Implement structured revision validation and shape history updates.
4. Implement optional previous-buffer replacement through existing open-buffer guardrails.
5. Wire daemon and HTTP routes.
6. Add model/service/store/HTTP tests.
7. Complete/release as patch under [[mother-view-composer]].

## Resolved Decisions

- Revisions are structured data only; Mother does not parse correction prose into UI code.
- Previous shapes remain in the library as inactive historical records with `replaced_by` set.
- Previous buffers are marked `replaced` only after a replacement buffer opens.
- If replacement opening reports an observability gap, the revision may persist but the previous buffer is not marked replaced.
- The renderer is only a caller; Mother owns revision, shape, and buffer state.

## Verification

```bash
cargo check -q
cargo test -q -p mother view_buffer_revision
cargo test -q -p mother view_buffer
cargo test -q -p mother view_request
patina spec check mother-view-buffer-revision --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Exit Criteria

- [x] `mvbr0-allium-code-alignment`
- [ ] `mvbr1-revision-model`
- [ ] `mvbr2-catalog-guardrails`
- [ ] `mvbr3-shape-history`
- [ ] `mvbr4-buffer-replacement`
- [ ] `mvbr5-persistence`
- [ ] `mvbr6-api`
- [ ] `mvbr7-fail-closed-guardrails`
- [ ] `mvbr8-tests-and-trace`

## Build Readiness

Ready to promote after the matching design document records code seams and implementation boundaries.
