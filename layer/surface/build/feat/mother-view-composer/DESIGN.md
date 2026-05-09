# Design: Mother View Composer

## Why This Design

The Mother View Composer is one Allium business feature implemented through multiple Patina specs. The umbrella spec prevents implementation slices from being mistaken for independent product features or full completion of the view system.

The important distinction:

- Allium target = product/business behavior.
- Patina specs = implementation slices.
- Releases = delivery checkpoints for slices.

This follows [[allium-as-business-backlog]] and keeps future work aligned with `layer/allium/mother/mother-view-composer-target.allium`.

## Build Target

This is a roadmap artifact, not a runtime build target.

It tracks the full Mother View Composer path:

```text
request → shape selection/adaptation/creation → fact validation → Mother buffer → renderer frame → revision/observability/maturation loops
```

## Resolved Decisions

- Keep existing tags/releases unchanged.
- Treat `v0.68.0`, `v0.69.0`, and `v0.70.0` as slices under the Mother View Composer feature.
- Use future release titles in the form `Mother View Composer: <Slice>`.
- Do not call the full view system complete until at least one renderer frame connects to Mother-owned buffers.
- Continue using child specs for implementation because the Allium target is intentionally larger than a single delivery slice.

## Current Slice Map

| Slice | Status | Release | Responsibility |
|---|---:|---:|---|
| [[mother-view-buffer-runtime]] | complete | `v0.68.0` | Mother-owned buffers/windows/frames/gaps |
| [[mother-view-shape-library]] | complete | `v0.69.0` | Persistent shapes and requirements |
| [[mother-view-request-composer]] | complete | `v0.70.0` | Structured request capture and explicit/exact safe opening |
| [[mother-view-shape-adaptation]] | draft | future | Similar-shape adaptation |
| [[mother-view-initial-shape-creation]] | draft | future | First shape when no match exists |
| [[mother-view-request-ux]] | draft | future | User/agent-facing request flow |
| [[mother-view-buffer-revision]] | draft | future | Corrections, revisions, buffer replacement |
| [[mother-view-observability-workflow]] | draft | future | Gap work items and resolution |
| [[mother-view-maturation]] | draft | future | Shape/derivation/pattern maturation |
| [[mother-sveltekit-frame]] | draft | future | First visible renderer frame |

## Direct Code Targets

None for this umbrella. Implementation remains in child specs.

Primary child-spec code areas so far:

- `mother/src/view_buffer/`
- `mother/src/http_api/view_buffer.rs`
- `mother/src/http_api.rs`
- `mother/src/http_routes.rs`
- `mother/src/state/mod.rs`
- `src/commands/mother/daemon/dispatch.rs`

## Verification Plan

Roadmap consistency:

```bash
patina spec check mother-view-composer --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

Child implementation verification remains owned by each child spec.

## Commits

No runtime commits expected for this umbrella.

## Build Readiness

Ready as a tracking/roadmap artifact. It should remain draft/ready until the full Mother View Composer product feature is actually complete.

## Open Questions

- Which next slice should be prioritized: visible renderer frame ([[mother-sveltekit-frame]]) or deeper Allium behavior ([[mother-view-shape-adaptation]] / [[mother-view-initial-shape-creation]])?
