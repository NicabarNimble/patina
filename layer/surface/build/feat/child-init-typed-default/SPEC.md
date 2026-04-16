---
type: feat
id: child-init-typed-default
status: draft
created: 2026-04-11
split_from: sdk-vision-lock
sessions:
  origin: 20260410-220235-028265000
related:
- src/main.rs
- src/child/scaffold.rs
- resources/templates/child/
- sdk/template/
- sdk/template-legacy/
- sdk/patina-sdk/
- sdk/patina-sdk-legacy/
- tests/template_end_to_end.rs
- layer/surface/build/feat/sdk-vision-lock/SPEC.md
- layer/surface/build/fix/sdk-public-surface-alignment/SPEC.md
beliefs:
- '[[sdk-is-mct-entry-point]]'
- '[[children-have-agency-toys-are-capabilities]]'
- '[[wasi-is-foundation-not-option]]'
exit_criteria:
- id: citd1-init-default-typed
  text: "`patina child init` defaults to typed SDK-first scaffolding for new first-party child authoring (no legacy handle template by default)."
  checked: true
- id: citd2-legacy-explicit
  text: "Legacy scaffolding path is explicit and non-default (flag/path/docs), with clear maintenance-only wording."
  checked: true
- id: citd3-template-alignment
  text: "Embedded scaffold templates, sdk/template, and docs are aligned (no contradictory imports, worlds, or manifest schema)."
  checked: true
- id: citd4-manifest-policy
  text: "Generated child manifests use `[needs].toys` + optional scopes and conform to current vocabulary (`child`/`kind`)."
  checked: true
- id: citd5-generated-builds
  text: "Generated typed child builds for `wasm32-wasip2` in e2e template test and compiles cleanly in CI."
  checked: true
- id: citd6-drift-guard
  text: "A regression test or check fails when scaffolded output drifts from locked SDK policy (typed-default + explicit legacy lane)."
  checked: true
---
# feat: Child init typed-default

> Make `patina child init` and scaffold templates typed-SDK-first, with legacy scaffolding explicit and non-default.

## Problem

Today there is a tooling split:

- `sdk/template/` is typed-first and aligned with new typed children.
- `patina child init` currently scaffolds embedded legacy-style templates from
  `resources/templates/child/*`.

This creates drift between SDK policy and CLI behavior. Developers can follow
CLI defaults and accidentally start on the wrong lane.

## Goal

Make CLI authoring behavior match SDK vision lock:

1. New child scaffolding defaults to typed SDK lane.
2. Legacy lane remains available only through explicit opt-in.
3. Templates, tests, and docs stay aligned so drift is blocked early.

## Non-Goals

- Migrating all existing legacy children in this spec.
- Resolving grammar lane strategy (covered by legacy/grammar disposition spec).
- Redesigning runtime composition.

## Current State

- `src/main.rs` routes `patina child init` to `patina::child::scaffold::scaffold`.
- `src/child/scaffold.rs` uses embedded templates under `resources/templates/child/`.
- Current embedded templates target legacy SDK feature lanes (`child` / `pipeline`).
- `sdk/template/` provides typed-first generate flow but is not the CLI default.

## Target State

- `patina child init` creates typed-first child scaffolds by default.
- Legacy scaffolding has explicit opt-in path and warning language.
- Docs/README/examples match CLI behavior.
- CI test catches template drift.

## Solution

1. Introduce typed-first scaffold selection in `patina child init` path.
2. Keep legacy scaffold only behind explicit opt-in UX.
3. Update embedded templates and/or wiring to reuse typed template source.
4. Update docs and tests to enforce behavior.

## Implementation Order

1. Add typed-default scaffold flow in `src/child/scaffold.rs` + CLI wiring.
2. Define explicit legacy opt-in behavior and user messaging.
3. Align template files across `resources/templates/child/` and `sdk/template/`.
4. Update template e2e tests and add drift guard.
5. Update docs references (`README.md`, SDK docs, command help text if needed).

## Resolved Decisions

- Default developer path must match vision lock, not historical behavior.
- Legacy path is allowed for maintenance only, never silent default.

## Verification

```bash
patina spec check child-init-typed-default --json
cargo test --test template_end_to_end
cargo check --workspace -q
```

## Exit Criteria

Frontmatter `citd1..citd6` are source of truth.

## Build Readiness

High: code path and templates are local and testable with existing harness.
