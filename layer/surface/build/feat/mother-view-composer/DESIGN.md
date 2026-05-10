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
| [[mother-view-shape-adaptation]] | complete | `v0.70.1` | Similar-shape adaptation |
| [[mother-view-initial-shape-creation]] | complete | `v0.70.2` | First shape when no match exists |
| [[mother-view-request-ux]] | complete | `v0.70.3` | User/agent-facing request flow |
| [[mother-view-buffer-revision]] | complete | `v0.70.4` | Corrections, revisions, buffer replacement |
| [[mother-view-observability-workflow]] | complete | `v0.70.5` | Gap work items and resolution |
| [[mother-view-maturation]] | complete | `v0.70.6` | Shape/derivation/pattern maturation |
| [[mother-sveltekit-frame]] | complete | `v0.70.7` | First visible renderer frame |

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

- [[commit-75011474]] — added the first SvelteKit frame and Mother payload endpoint.
- [[commit-533918ff]] — released the final renderer slice as `v0.70.7`.

## Build Readiness

The tracked v1 product feature is now complete at `8/8`: Mother owns the view substrate and the first renderer frame exists.

## Open Questions

- Should the completed umbrella be closed as a parent release, or remain as a roadmap artifact for post-v1 frame work?
- Which next frame should follow the SvelteKit proof: TUI or Emacs?
