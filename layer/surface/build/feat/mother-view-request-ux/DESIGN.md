# Design: Mother View Request UX

## Why This Design

Mother already owns view requests, shapes, buffers, windows, and observability gaps. The request UX slice should therefore expose Mother-owned workflow state instead of creating renderer-local state.

The design keeps the existing compose behavior intact:

- exact/explicit matches may open buffers immediately;
- similar matches create adapted exploratory shapes without opening buffers;
- no-match requests create initial exploratory shapes without opening buffers;
- fail-closed outcomes remain explicit;
- later UX actions do not rewrite historical request outcomes.

Because Allium currently treats `DisplayRequest.outcome` as terminal, a follow-up action returns its own result and persists only buffer/gap side effects.

## Build Target

Add persisted request detail and explicit open-linked-shape action:

- detail model: request + match + optional creation/adaptation + linked shape + available actions;
- persistence: store `ViewShapeCreation` and `ViewShapeAdaptation` keyed by `request_id`;
- APIs:
  - `GET /api/view-requests/details`
  - `GET /api/view-requests/<request_id>/detail`
  - `POST /api/view-requests/open-shape`
- action behavior: validate request/shape relation, then call existing catalog-backed open-buffer path.

## Resolved Decisions

- Use new detail endpoints so existing basic request list/get APIs remain stable.
- Persist artifact relations directly; do not infer from generated shape ids.
- Opening a linked shape is an explicit action, not an automatic continuation of composition.
- The action result does not mutate the original `DisplayRequest.outcome`.
- Renderer frames may call these APIs but do not own the workflow state.

## Commits

1. `spec: promote mother-view-request-ux to ready` — define the request UX slice and guardrails.
2. `feat: expose mother view request ux` — implement detail/action models, persistence, APIs, and tests.

## Direct Code Targets

- `mother/src/view_buffer/model.rs` — request detail/action model types.
- `mother/src/view_buffer/service.rs` — detail construction and linked-shape open action.
- `mother/src/view_buffer/store.rs` — artifact persistence for shape creation/adaptation.
- `mother/src/state/mod.rs` — runtime-store wrappers and persistence tests.
- `mother/src/http_api.rs` — API trait additions and route handlers.
- `mother/src/http_api/view_buffer.rs` — detail/open-shape handlers.
- `mother/src/http_routes.rs` — route table wiring and auth tests.
- `src/commands/mother/daemon/dispatch.rs` — daemon runtime implementation and persistence effects.
- `mother/src/http_api/tests/mod.rs` — HTTP behavior tests.
- `src/commands/mother/daemon/tests/mod.rs` — daemon persistence/action tests.

## Verification Plan

```bash
cargo check -q
cargo test -q -p mother view_request_ux
cargo test -q -p mother view_request
cargo test -q -p mother view_buffer
patina spec check mother-view-request-ux --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Build Readiness

Ready. Existing request composer, shape adaptation, initial shape creation, and open-buffer seams provide the required substrate.

## Open Questions

- Should a future Allium revision add non-terminal request states such as `shape_created` or `awaiting_user_confirmation`? This slice intentionally avoids that change.
