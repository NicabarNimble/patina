---
type: feat
id: mother-view-maturation
status: draft
created: 2026-05-09
target: mother-view-composer
release_bump: patch
sessions:
  origin: 20260508-144836-859149000
related:
- mother-view-composer
- layer/allium/mother/mother-view-composer-target.allium
- mother-view-shape-library
- mother-view-observability-workflow
beliefs:
- '[[allium-as-business-backlog]]'
- '[[umbrella-specs-release-child-patches]]'
exit_criteria:
- id: mvmat1-maturation-model
  text: Mother exposes structured model types for view maturation events, target kinds, origins, derivations, display patterns, and observability-improvement artifacts using Allium maturity vocabulary.
  checked: false
- id: mvmat2-artifact-library
  text: Mother can persist and list derivation and display-pattern artifacts linked to view shapes so non-shape view artifacts have durable maturity state.
  checked: false
- id: mvmat3-shape-maturation
  text: A structured maturation request can promote an active view shape forward through maturity states and record a maturation event without rewriting shape history.
  checked: false
- id: mvmat4-derivation-pattern-maturation
  text: A structured maturation request can promote existing derivation and display-pattern artifacts forward through maturity states while failing closed for unknown targets or invalid transitions.
  checked: false
- id: mvmat5-observability-improvement-artifact
  text: Promoting a derivation to stable or promoted can create a persisted observability-improvement artifact with desired fact path, reason, source maturation, and no work item side effect.
  checked: false
- id: mvmat6-api
  text: HTTP and daemon APIs list/upsert derivations and display patterns, list maturation events and observability improvements, and record maturation requests.
  checked: false
- id: mvmat7-tests-and-trace
  text: Model, service, persistence, daemon, and HTTP tests cover successful shape/derivation/pattern maturation, observability improvement creation, and fail-closed guardrails.
  checked: false
- id: mvmat8-validation
  text: "`cargo check -q`, targeted Mother tests, `cargo test -q -p mother`, `patina spec check mother-view-maturation --json`, and `allium check layer/allium/mother/mother-view-composer-target.allium` pass."
  checked: false
---
# feat: Mother View Maturation

> Promote view shapes, derivations, display patterns, and observability-improvement artifacts through Allium maturity states.

## Problem

Mother can now create, adapt, revise, open, and observe view shapes, but every newly created/adapted/revised shape remains exploratory unless a user or agent edits the whole shape record directly. Allium describes a separate maturation pathway: useful shapes, derivations, and display patterns should move from exploratory to candidate, stable, and promoted states with explicit event history.

Without a maturation workflow:

- exploratory artifacts cannot become trusted library artifacts through a recorded action;
- derivations and display patterns do not have durable artifact records at all;
- observability improvements from mature derivations have no structured place to land;
- renderers would eventually need to infer maturity from shape metadata instead of Mother-owned history.

## Goal

Add Mother-owned maturation workflow state for view artifacts while preserving the existing rule that views never invent data.

A structured request should be able to:

1. mature an active `ViewShape` forward through Allium maturity states;
2. persist `ViewDerivation` and `DisplayPattern` records linked to a shape;
3. mature derivation and display-pattern records forward through maturity states;
4. record each maturation as a `ViewMaturationEvent`;
5. optionally create an `ObservabilityImprovementArtifact` when a derivation reaches stable/promoted maturity.

## Status

Draft implementation slice under [[mother-view-composer]]. Expected release: `v0.70.6 — Mother View Composer: Maturation`.

## Non-Goals

- Do not implement a full Allium expression compiler for derivations.
- Do not generate renderer code or Svelte components.
- Do not open buffers as a side effect of maturation.
- Do not create external tickets/work items. Observability-improvement artifacts are internal Mother records with `work_item_created = false`.
- Do not demote maturity or rewrite historical maturation events.

## Target Shape

Mother owns these records:

- `ViewDerivation`: `derivation_id`, `shape_id`, `label`, `expression_ref`, input fact paths, maturity.
- `DisplayPattern`: `pattern_id`, `shape_id`, `pattern_kind`, maturity.
- `ViewMaturationEvent`: target kind, target ids, origin, from/to maturity, timestamp.
- `ObservabilityImprovementArtifact`: desired fact path, reason, optional source gap, optional source maturation, creation time, `work_item_created`.

Maturation is explicit and forward-only:

```text
exploratory -> candidate -> stable -> promoted
```

## Solution

- Extend `mother/src/view_buffer/model.rs` with maturation/derivation/pattern/artifact types.
- Extend `mother/src/view_buffer/store.rs` with durable tables and save/get/list helpers.
- Extend `ViewBufferService` with `mature_view_artifact` validation and transition logic.
- Extend `MotherRuntimeStore`, daemon dispatch, HTTP traits, route table, and handlers.
- Add focused tests before release.

## Implementation Order

1. Add model types and model tests.
2. Add persistence tables and store/state tests.
3. Add service maturation transition logic and guardrail tests.
4. Add daemon dispatch persistence integration.
5. Add HTTP handlers/routes/tests.
6. Validate, promote active, complete, and release as a patch child of [[mother-view-composer]].

## Resolved Decisions

- Maturation is a new event stream, not a revision. It changes maturity only and records an immutable event.
- Shape maturation mutates `ViewShape.maturity` in place because it does not replace shape definition or buffer semantics.
- Derivation and display-pattern records are minimal library records; expression execution and renderer behavior stay out of scope.
- Observability-improvement artifact creation is allowed only from derivation maturation to `stable` or `promoted`.

## Verification

```bash
cargo check -q
cargo test -q -p mother view_maturation
cargo test -q -p mother view_buffer
cargo test -q -p mother
patina spec check mother-view-maturation --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Exit Criteria

- [ ] `mvmat1-maturation-model`
- [ ] `mvmat2-artifact-library`
- [ ] `mvmat3-shape-maturation`
- [ ] `mvmat4-derivation-pattern-maturation`
- [ ] `mvmat5-observability-improvement-artifact`
- [ ] `mvmat6-api`
- [ ] `mvmat7-tests-and-trace`
- [ ] `mvmat8-validation`

## Build Readiness

Ready after direct-code targets are filled in the design read pass.
