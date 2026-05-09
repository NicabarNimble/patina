---
type: feat
id: mother-view-composer
status: draft
created: 2026-05-09
sessions:
  origin: 20260508-144836-859149000
related:
- layer/allium/mother/mother-view-composer-target.allium
- layer/allium/mother/mother-view-composer-target.plan.json
- mother-view-buffer-runtime
- mother-view-shape-library
- mother-view-request-composer
- mother-view-shape-adaptation
- mother-view-initial-shape-creation
- mother-view-request-ux
- mother-view-buffer-revision
- mother-view-observability-workflow
- mother-view-maturation
- mother-sveltekit-frame
beliefs:
- '[[allium-as-agent-display-lisp]]'
- '[[allium-as-business-backlog]]'
exit_criteria:
- id: mvc1-backend-substrate
  text: Mother owns persistent buffers, shapes, requests, matches, windows, frames, and observability gaps behind JSON/control-plane APIs.
  checked: true
- id: mvc2-request-to-buffer-path
  text: A structured user display request can be matched to an active persisted shape and safely open a Mother-owned buffer or record a fail-closed outcome.
  checked: true
- id: mvc3-shape-adaptation
  text: Similar-shape matches can create/adapt exploratory shapes without arbitrary generated UI code.
  checked: true
- id: mvc4-initial-shape-creation
  text: No-match requests can create initial structured shapes with explicit backing requirements and no invented data.
  checked: true
- id: mvc5-revision-and-replacement
  text: User corrections create shape revisions and replace live/stale/blocked buffers while preserving history.
  checked: true
- id: mvc6-observability-workflow
  text: Observability gaps can be linked to work items and resolved when catalog facts become observed.
  checked: false
- id: mvc7-maturation
  text: Shapes, derivations, display patterns, and observability-improvement artifacts can mature through Allium states.
  checked: false
- id: mvc8-renderer-frame
  text: At least one renderer frame, starting with SvelteKit, connects to Mother-owned buffers without owning buffer or shape state.
  checked: false
---
# feat: Mother View Composer

> Umbrella roadmap for the Allium-backed Mother view system. This is the product/business feature; implementation specs are slices under it.

## Problem

The recent `v0.68.0`, `v0.69.0`, and `v0.70.0` releases are valuable checkpoints, but they are not separate product features in the conceptual model. They are additive slices of one larger Allium target: the Mother View Composer.

Without an umbrella artifact, release names and spec boundaries can imply that each slice is an independent feature. That makes it harder to answer “is the view system done?” honestly.

## Goal

Track the full Mother view system as one Allium-backed feature while preserving the useful implementation-slice history.

This spec exists to make the hierarchy explicit:

```text
Mother View Composer / Mother View System
├── v0.68.0: [[mother-view-buffer-runtime]]
├── v0.69.0: [[mother-view-shape-library]]
├── v0.70.0: [[mother-view-request-composer]]
├── v0.70.1: [[mother-view-shape-adaptation]]
├── v0.70.2: [[mother-view-initial-shape-creation]]
├── v0.70.3: [[mother-view-request-ux]]
├── v0.70.4: [[mother-view-buffer-revision]]
├── future: [[mother-view-observability-workflow]]
├── future: [[mother-view-maturation]]
└── future: [[mother-sveltekit-frame]]
```

## Status

Draft roadmap/umbrella spec. Not an implementation slice by itself.

Completed backend slices:

- [[mother-view-buffer-runtime]] — `v0.68.0`
- [[mother-view-shape-library]] — `v0.69.0`
- [[mother-view-request-composer]] — `v0.70.0`
- [[mother-view-shape-adaptation]] — `v0.70.1`
- [[mother-view-initial-shape-creation]] — `v0.70.2`
- [[mother-view-request-ux]] — `v0.70.3`
- [[mother-view-buffer-revision]] — `v0.70.4`

Current honest system label:

> Mother View Composer backend substrate and structured request path are implemented; the full visible view system is not product-complete until a renderer frame exists.

## Allium authority

Behavioral authority remains:

- `layer/allium/mother/mother-view-composer-target.allium`
- `layer/allium/mother/mother-view-composer-target.plan.json`

This umbrella follows [[allium-as-business-backlog]]: completing an implementation slice does not imply the whole Allium behavior is complete.

## Non-Goals

- Do not rewrite completed tags/releases.
- Do not retroactively rename existing commits.
- Do not collapse child implementation specs into this artifact.
- Do not mark the whole view system done until request → shape → buffer → renderer frame works.

## Target Shape

A product-complete Mother View Composer means:

1. A user can ask for a Mother display.
2. An agent can select, adapt, or create a structured view shape.
3. Mother validates required catalog facts and source availability.
4. Mother opens or refuses a live buffer without inventing data.
5. One or more renderer frames can connect windows to that Mother-owned buffer.
6. User corrections revise/replace shapes and buffers.
7. Missing data becomes observable work, not fake UI state.
8. Useful shapes/derivations/patterns can mature into promoted artifacts.

## Release Naming Guidance

Going forward, release titles for this effort should use umbrella + slice naming and child/slice patch versions while the parent feature remains open:

- `Mother View Composer: Buffer Runtime`
- `Mother View Composer: Shape Library`
- `Mother View Composer: Request Composer`
- `Mother View Composer: Shape Adaptation`
- `Mother View Composer: SvelteKit Frame`

Implementation slice specs under this umbrella should carry:

```yaml
target: mother-view-composer
release_bump: patch
```

The parent/umbrella completion may use a minor or major bump when the whole Mother View Composer product feature is complete.

## Implementation Slices

### Completed

- [[mother-view-buffer-runtime]]: persistent Mother-owned buffer/window/frame/gap kernel.
- [[mother-view-shape-library]]: persistent Mother-owned `ViewShape` and `ViewRequirement` records.
- [[mother-view-request-composer]]: structured request capture, shape-match persistence, explicit/exact safe opening.
- [[mother-view-shape-adaptation]]: similar-shape adaptation into exploratory shapes.
- [[mother-view-initial-shape-creation]]: initial structured shapes for no-match requests.
- [[mother-view-request-ux]]: persisted request details and explicit linked-shape open actions.
- [[mother-view-buffer-revision]]: structured corrections, shape history, and buffer replacement.

### Planned

- [[mother-view-observability-workflow]]: link/resolve observability gaps.
- [[mother-view-maturation]]: mature shapes, derivations, patterns, and observability-improvement artifacts.
- [[mother-sveltekit-frame]]: first visible renderer frame.

## Verification

Umbrella verification is not a code test. It is a roadmap consistency check:

```bash
patina spec check mother-view-composer --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

Implementation slices carry their own tests and releases.

## Exit Criteria

- [x] `mvc1-backend-substrate`
- [x] `mvc2-request-to-buffer-path`
- [x] `mvc3-shape-adaptation`
- [x] `mvc4-initial-shape-creation`
- [x] `mvc5-revision-and-replacement`
- [ ] `mvc6-observability-workflow`
- [ ] `mvc7-maturation`
- [ ] `mvc8-renderer-frame`

## Build Readiness

This umbrella is ready as a tracking artifact. Do not promote it as a normal implementation spec unless the intent is to close the entire Mother View Composer product feature.
