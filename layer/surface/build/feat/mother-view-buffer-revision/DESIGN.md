# Design: Mother View Buffer Revision

## Why This Design

Revision belongs in Mother because Mother owns shapes, buffers, requests, and observability gaps. Renderers may surface correction actions, but they should not mutate shape history or decide whether a buffer can be replaced.

The design follows the existing view system guardrails:

- structured inputs only;
- catalog-backed required facts only;
- no invented payload data;
- no executable UI code;
- replacement buffers open through the same `open_buffer` checks as normal view opens.

## Build Target

Add structured shape revision and optional buffer replacement:

- `ViewShapeRevision` plus origin/state enums;
- `ReviseViewShapeRequest` / `RevisedViewShapeOutcome`;
- persistent revision records;
- buffer replacement linkage via `replacement_buffer_id`;
- APIs:
  - `GET /api/view-shape-revisions`
  - `GET /api/view-shape-revisions/<revision_id>`
  - `POST /api/view-shapes/revise`

## Resolved Decisions

- Previous shapes are preserved as inactive records with `replaced_by` set.
- Revised shapes are new active versions, not in-place edits.
- Previous buffers are marked `replaced` only after a replacement buffer opens.
- Observability gaps from attempted replacement are persisted, but they do not mark the previous buffer replaced.
- Requirement changes use the existing Mother data catalog checks.

## Commits

1. `spec: promote mother-view-buffer-revision to ready` — define the bounded revision/replacement slice.
2. `feat: revise mother view buffers` — implement revision model, persistence, API, daemon wiring, and tests.

## Direct Code Targets

- `mother/src/view_buffer/model.rs` — revision model, enums, buffer replacement linkage.
- `mother/src/view_buffer/service.rs` — revision validation, shape history, optional buffer replacement.
- `mother/src/view_buffer/store.rs` — revision table and buffer replacement column persistence.
- `mother/src/state/mod.rs` — runtime-store wrappers and persistence tests.
- `mother/src/http_api.rs` — API trait additions and route handlers.
- `mother/src/http_api/view_buffer.rs` — revision list/get/apply handlers.
- `mother/src/http_routes.rs` — route table wiring/auth tests.
- `src/commands/mother/daemon/dispatch.rs` — daemon runtime implementation and persistence effects.
- `mother/src/http_api/tests/mod.rs` — HTTP tests.

## Verification Plan

```bash
cargo check -q
cargo test -q -p mother view_buffer_revision
cargo test -q -p mother view_buffer
cargo test -q -p mother view_request
patina spec check mother-view-buffer-revision --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Build Readiness

Ready. The shape library, buffer runtime, request UX, and open-buffer paths provide the required substrate.

## Open Questions

- Should future revisions support multi-step proposed/rejected/reverted workflows? This slice implements applied structured corrections only.
