---
type: feat
id: mother-view-observability-workflow
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
- id: mvow0-allium-code-alignment
  text: The slice starts from an Allium/code alignment pass over ObservabilityGap, LinkObservabilityGapToWorkItem, ResolveObservabilityGapWhenFactBecomesObserved, catalog checks, persistence, daemon, and HTTP seams.
  checked: true
- id: mvow1-gap-detail-model
  text: Mother exposes observability gap detail with gap status, missing fact/source, optional linked work item, and terminal resolved timestamp.
  checked: true
- id: mvow2-link-work-item
  text: Open observability gaps can be linked to a non-empty work item id and transition to linked_to_work_item without inventing data.
  checked: true
- id: mvow3-resolve-from-catalog
  text: Open or linked gaps resolve only when the missing fact is catalogued, observed, and sourced from an available source.
  checked: true
- id: mvow4-persistence
  text: Gap status, linked work item ids, and resolved timestamps persist through the Mother store/daemon path.
  checked: true
- id: mvow5-api
  text: HTTP callers can list/get gaps, link gaps to work items, and resolve gaps without owning observability state.
  checked: true
- id: mvow6-fail-closed-guardrails
  text: Unknown gaps, terminal gaps, blank work item ids, mismatched fact paths, missing catalog facts, and unavailable sources fail closed without status mutation.
  checked: true
- id: mvow7-tests-and-trace
  text: Deterministic tests cover linking, resolving, persistence, API response shape, fail-closed behavior, and Allium/spec obligation comments.
  checked: true
validated_against_commit: 5878e24223bdcd2580d9057d9db7af465b8c6420
last_freshness_check: 2026-05-10T00:42:21Z
freshness_scope:
- Mother view observability gap detail, link/resolve workflow, catalog resolution checks, store/daemon persistence, HTTP API, fail-closed tests
---
# feat: Mother View Observability Workflow

> Link, track, and resolve observability gaps produced by view requirements when catalog facts or sources are missing.

## Problem

Mother already records fail-closed observability gaps when required facts are missing or their sources are unavailable. Those gaps are currently listable, but they are not actionable: users/agents cannot link them to work items or close them when catalog data becomes available.

The Allium target includes explicit lifecycle rules for this: open gaps may be linked to work items, and open/linked gaps may resolve only when the missing catalog fact becomes observed.

## Goal

Implement the bounded observability workflow slice:

1. Preserve existing gap creation behavior.
2. Add a gap detail model with optional linked work item.
3. Allow open gaps to link to non-empty work item ids.
4. Resolve open or linked gaps only when the missing fact is observed from an available catalog source.
5. Persist status transitions and timestamps.
6. Expose gap workflow through Mother APIs.
7. Fail closed for invalid transitions and unavailable facts.

## Status

Ready for implementation after Allium/code alignment.

## Allium authority

This spec implements the bounded observability lifecycle slice of:

- `layer/allium/mother/mother-view-composer-target.allium`
- `layer/allium/mother/mother-view-composer-target.plan.json`

Primary Allium rules targeted:

- `LinkObservabilityGapToWorkItem`
- `ResolveObservabilityGapWhenFactBecomesObserved`

Primary Allium entities used:

- `ObservabilityGap`
- `MotherDataCatalog`
- `CataloguedFact`
- `CataloguedSource`

## Non-Goals

- Do not create external issue tracker tickets in this slice.
- Do not scrape or synthesize missing facts.
- Do not mark a gap resolved unless the exact missing fact is observed and its source is available.
- Do not reopen resolved gaps.
- Do not implement observability improvement artifact maturation; that belongs to [[mother-view-maturation]].
- Do not implement renderer UI; that belongs to [[mother-sveltekit-frame]].

## Target Shape

A gap can be linked:

```json
{
  "gap_id": "gap_mother_status_default_children_total",
  "work_item_id": "work/MOTHER-123"
}
```

Then later resolved when the missing fact is observed:

```json
{
  "gap_id": "gap_mother_status_default_children_total"
}
```

Mother validates the missing fact path against the current data catalog before setting:

```text
status = resolved
resolved_at = now
```

## Solution

Extend the view-buffer backend with:

- optional `linked_work_item_id` on `ObservabilityGap`;
- `LinkObservabilityGapRequest` and `ResolveObservabilityGapRequest`;
- service methods for link/resolve transitions;
- store get/save support for gap details and additive schema migration;
- daemon methods that load current gaps and catalog state;
- APIs:
  - `GET /api/view-buffers/gaps/<gap_id>`
  - `POST /api/view-buffers/gaps/link-work-item`
  - `POST /api/view-buffers/gaps/resolve`

## Implementation Order

1. Add gap detail fields and link/resolve request types.
2. Add service transition helpers with catalog validation.
3. Add store get/save support and migration for linked work item ids.
4. Wire daemon runtime methods and persistence effects.
5. Wire HTTP routes/handlers.
6. Add model/service/store/HTTP tests.
7. Complete/release as patch under [[mother-view-composer]].

## Resolved Decisions

- Work item ids are references only; Mother does not create external work items here.
- Resolution is tied to the exact `missing_fact_path` already recorded on the gap.
- Resolved is terminal.
- Linking is allowed only from open gaps.
- Renderers can call the APIs but do not own gap state.

## Verification

```bash
cargo check -q
cargo test -q -p mother view_observability_workflow
cargo test -q -p mother view_buffer
cargo test -q -p mother view_request
patina spec check mother-view-observability-workflow --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Exit Criteria

- [x] `mvow0-allium-code-alignment`
- [x] `mvow1-gap-detail-model`
- [x] `mvow2-link-work-item`
- [x] `mvow3-resolve-from-catalog`
- [x] `mvow4-persistence`
- [x] `mvow5-api`
- [x] `mvow6-fail-closed-guardrails`
- [x] `mvow7-tests-and-trace`

## Build Readiness

Ready to promote after the matching design document records code seams and implementation boundaries.
