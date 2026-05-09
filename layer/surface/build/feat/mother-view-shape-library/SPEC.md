---
type: feat
id: mother-view-shape-library
status: active
created: 2026-05-09
sessions:
  origin: 20260508-144836-859149000
related:
- layer/allium/mother/mother-view-composer-target.allium
- layer/allium/mother/mother-view-composer-target.plan.json
- layer/surface/done/feat/mother-view-buffer-runtime/SPEC.md
- mother/src/view_buffer
beliefs:
- '[[spec-driven-design]]'
- '[[contracts-before-consumers]]'
- '[[allium-as-agent-display-lisp]]'
- '[[allium-as-business-backlog]]'
exit_criteria:
- id: mvsl0-read-before-write
  text: Implementation begins from documented reads of the completed view-buffer runtime, Mother store, and API route patterns before changing shape-library code.
  checked: true
- id: mvsl1-shape-model
  text: 'Mother has a first-class ViewShape library model aligned with Allium v1 fields: shape id, title, source_ref, scope, version, active, major/minor modes, maturity, payload contract/version, optional vision/project/replaced_by, and requirements.'
  checked: true
- id: mvsl2-shape-persistence
  text: View shapes and requirements persist in Mother state with deterministic list/get/upsert/deactivate behavior and no dependency on renderer state.
  checked: true
- id: mvsl3-shape-api
  text: Mother exposes control-plane APIs to list, read, create/update, and deactivate view shapes without accepting arbitrary executable UI code.
  checked: true
- id: mvsl4-open-from-library
  text: Opening a buffer can use an active persisted shape from the shape library, validates declared requirements through the data catalog, and still fails closed with observability gaps.
  checked: true
- id: mvsl5-proof-shapes-seeded
  text: The existing Mother status proof shape is seeded or represented as a library shape so built-in-only shape lookup is no longer the only path.
  checked: true
- id: mvsl6-tests-and-trace
  text: Deterministic tests cover shape persistence, shape APIs, inactive/missing shape fail-closed behavior, opening from library shape, and Allium obligation trace comments.
  checked: true
- id: mvsl7-follow-on-backlog
  text: Remaining Allium business goals outside this slice are explicitly split into follow-on specs for request composition, shape revision/replacement, observability workflow, maturation, and renderer frames.
  checked: true
validated_against_commit: 5c53a451879e9271c3c9ede3de73bad2c8f65154
last_freshness_check: 2026-05-09T10:26:14-04:00
freshness_scope:
- mother/src/view_buffer
- mother/src/http_api.rs
- mother/src/http_api/view_buffer.rs
- mother/src/http_routes.rs
- mother/src/state/mod.rs
- src/commands/mother/daemon/dispatch.rs
---
# feat: Mother View Shape Library

> Persist and expose local/user editable Mother view shapes so the view buffer runtime is no longer limited to built-in proof shapes.

## Problem

`mother-view-buffer-runtime` completed the kernel: Mother can persist buffers/frames/windows/gaps, open a proof buffer over observed Mother status facts, and expose buffer APIs. But the Allium target is broader: agents and users should work with editable view shapes, not a hardcoded Rust-only proof shape.

Right now the runtime still treats `mother.status.default` as built-in service data. That proves the buffer contract, but it does not yet implement the local Allium view-shape library described by `mother-view-composer-target.allium`.

## Goal

Build the next business slice from the Allium target:

1. Make `ViewShape` records first-class Mother-owned data.
2. Persist each shape's declared backing requirements.
3. Expose small control-plane APIs for listing, reading, upserting, and deactivating shapes.
4. Let `open buffer` resolve active shapes from the library, not only from built-in Rust records.
5. Preserve the core rule: shapes are structured metadata/guardrails, not arbitrary executable UI code.
6. Keep SvelteKit/TUI/Emacs as future frames/renderers; they do not own shapes.

## Status

Draft follow-on spec created after `[[mother-view-buffer-runtime]]` passed 8/8 and was completed/released.

## Allium authority

This spec implements the ViewShape library portion of:

- `layer/allium/mother/mother-view-composer-target.allium`
- `layer/allium/mother/mother-view-composer-target.plan.json`

Primary Allium entities targeted here:

- `ViewShape`
- `ViewRequirement`
- `MotherDisplayContext.shapes`
- `MotherDataCatalog` only as a backing validation dependency

Primary Allium rules partially enabled here:

- `SelectExplicitUserRequestedShape` — by allowing explicit shape ids to be stored and selected.
- `OpenLiveBufferWhenRequiredFactsAreObserved` — now from library shapes, with required requirements validated against observed facts and available sources.
- `RecordObservabilityGapWhenRequiredFactIsMissing` — preserved for library shapes when a required requirement cannot be satisfied.

## Non-Goals

- Natural-language request parsing or agent shape matching.
- Similar-shape adaptation.
- User correction/revision flow that replaces an existing buffer.
- Maturation of derivations/patterns into promoted artifacts.
- SvelteKit/TUI/Emacs renderer implementation.
- Arbitrary generated Svelte/TypeScript or executable scripts inside a shape.

These are follow-on specs, not hidden work in this slice:

- [[mother-view-request-composer]]
- [[mother-view-buffer-revision]]
- [[mother-view-observability-workflow]]
- [[mother-view-maturation]]
- [[mother-sveltekit-frame]]

## Target Shape

A v1 library shape should be able to represent the Mother status shape as data:

```text
shape_id: mother.status.default
title: Mother Status
source_ref: local-allium-view-library
scope: mother_user
version: 1
active: true
major_mode: table
minor_modes: [pinned]
maturity: stable
payload_contract: framed_json
payload_version: 1
requirements:
  - mother.status.version
  - mother.status.control_plane_ready
  - mother.status.registered_projects
  - mother.status.children_ready_count
  - mother.status.children_total
```

The representation may stay Rust/SQLite JSON-friendly in this slice, but it must be data that can later be generated from or round-tripped with Allium-local view metadata.

Allium `vision: VisionContext?` and `project: ProjectContext?` may be stored as the stable projections `vision_id: Option<String>` and `project_uid: Option<String>` in Rust/SQLite. This is a storage projection, not a semantic change.

## Solution

Add a shape-library layer to `mother/src/view_buffer/`:

- extend the model with Allium-aligned shape fields where missing;
- persist shapes and requirements in Mother state;
- treat `ViewRequirement.required == true` as opening blockers; optional requirements may be stored for future display enrichment but must not cause fake data to be invented;
- seed/register the Mother status proof shape;
- update service lookup so `open_buffer(shape_id)` resolves an active library shape first;
- expose HTTP/control-plane handlers for shape list/read/upsert/deactivate;
- reject shape payloads that contain executable renderer code or undeclared requirement data.

## Implementation Order

1. Read current `view_buffer` model/store/service/API code and record boundary findings in `DESIGN.md`.
2. Extend `ViewShape` model minimally for Allium alignment.
3. Add shape and requirement persistence.
4. Seed/register `mother.status.default` as library data.
5. Wire service lookup to use active persisted shapes.
6. Add shape API handlers/routes.
7. Add deterministic tests and Allium obligation comments.
8. Update follow-on backlog for the remaining Allium target.

## Resolved Decisions

- Mother owns shapes; renderer clients only render buffers opened from shapes.
- View shapes are structured data, not arbitrary executable UI code.
- This slice keeps shape matching/revision/maturation out of scope so the shape library can land independently.

## Verification

```bash
cargo check -q
cargo test -q -p mother view_buffer
cargo test -q -p mother view_shape
patina spec check mother-view-shape-library --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Exit Criteria

- [x] `mvsl0-read-before-write`
- [x] `mvsl1-shape-model`
- [x] `mvsl2-shape-persistence`
- [x] `mvsl3-shape-api`
- [x] `mvsl4-open-from-library`
- [x] `mvsl5-proof-shapes-seeded`
- [x] `mvsl6-tests-and-trace`
- [x] `mvsl7-follow-on-backlog`

## Build Readiness

Ready to promote as the next implementation spec after this polish pass. The first implementation task remains `mvsl0-read-before-write`: record the read-before-write findings in `DESIGN.md` before changing runtime code.
