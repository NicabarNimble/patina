# Design: Mother View Observability Workflow

## Why This Design

Observability gaps are Mother-owned consequences of fail-closed view opening. The workflow that links and resolves them must also be Mother-owned so renderers and agents cannot silently invent missing facts or bypass catalog checks.

This design keeps gap creation unchanged and adds only lifecycle transitions:

- open -> linked_to_work_item;
- open/linked_to_work_item -> resolved;
- resolved is terminal.

## Build Target

Add gap workflow state and APIs:

- optional `linked_work_item_id` on `ObservabilityGap`;
- `LinkObservabilityGapRequest`;
- `ResolveObservabilityGapRequest`;
- service validation for linking and resolving;
- store migration/persistence for linked work item ids;
- APIs:
  - `GET /api/view-buffers/gaps/<gap_id>`
  - `POST /api/view-buffers/gaps/link-work-item`
  - `POST /api/view-buffers/gaps/resolve`

## Resolved Decisions

- Work item ids are references, not created external tickets.
- Resolution validates the exact missing fact path against the current Mother data catalog.
- A fact must be observed and its source must be available before a gap resolves.
- Linking a resolved gap is refused.
- Resolving an already resolved gap is refused.

## Commits

1. `spec: promote mother-view-observability-workflow to ready` — define the bounded gap lifecycle slice.
2. `feat: manage mother view observability gaps` — implement gap linking/resolution, persistence, APIs, and tests.

## Direct Code Targets

- `mother/src/view_buffer/model.rs` — gap linked work item field and request types.
- `mother/src/view_buffer/service.rs` — link/resolve transition helpers.
- `mother/src/view_buffer/store.rs` — gap get/save migration and linked work item persistence.
- `mother/src/state/mod.rs` — runtime-store wrappers and persistence tests.
- `mother/src/http_api.rs` — API trait additions and route handlers.
- `mother/src/http_api/view_buffer.rs` — gap detail/link/resolve handlers.
- `mother/src/http_routes.rs` — route table wiring/auth tests.
- `src/commands/mother/daemon/dispatch.rs` — daemon runtime implementation and persistence effects.
- `mother/src/http_api/tests/mod.rs` — HTTP tests.

## Verification Plan

```bash
cargo check -q
cargo test -q -p mother view_observability_workflow
cargo test -q -p mother view_buffer
cargo test -q -p mother view_request
patina spec check mother-view-observability-workflow --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Build Readiness

Ready. Existing gap creation, store persistence, catalog checks, and buffer APIs provide the required substrate.

## Open Questions

- Should future work items become first-class Mother records? This slice stores only external/reference ids.
