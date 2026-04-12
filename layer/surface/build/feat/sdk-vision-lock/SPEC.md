---
type: feat
id: sdk-vision-lock
status: active
created: 2026-04-11
sessions:
  origin: 20260410-220235-028265000
related:
- sdk/patina-sdk
- wit/
- src/child
- src/mother
- layer/surface/build/feat/sdk-developer-platform/SPEC.md
- layer/surface/build/refactor/child-typed-composition/SPEC.md
- layer/surface/build/feat/child-construction-canon/SPEC.md
- layer/surface/build/fix/sdk-public-surface-alignment/SPEC.md
- layer/surface/build/explore/spec-manager-wasm-child/SPEC.md
- layer/surface/build/feat/ba-truths/SPEC.md
- layer/surface/build/feat/child-init-typed-default/SPEC.md
- layer/surface/build/refactor/legacy-and-grammar-disposition/SPEC.md
- layer/surface/build/feat/mother-grant-audit-coverage/SPEC.md
- layer/surface/build/feat/job-aligned-mct-backoffice/SPEC.md
beliefs:
- '[[sdk-is-mct-entry-point]]'
- '[[wasi-is-foundation-not-option]]'
- '[[children-have-agency-toys-are-capabilities]]'
- '[[core-verbs-standalone-mother-additive]]'
exit_criteria:
- id: svl1-vision-ratified
  text: 'SDK vision lock statements are explicitly ratified and recorded in this spec: BA/WASI/component-model-first, SDK as canonical child authoring surface, Mother as authority boundary, typed toy contracts over string payloads for new data-plane children.'
  checked: true
- id: svl2-sdk-canonical-surface
  text: SDK is canonical for all new first-party child authoring. `patina child init` and templates default to the typed SDK lane; legacy lane is maintenance-only via explicit allowlist.
  checked: false
- id: svl3-typed-default
  text: Default lane for new data-plane children is typed WIT toy exports/imports. The six canon children run in typed lane; `handle(action,payload)` remains legacy/control-plane lane only with explicit justification and migration note.
  checked: false
- id: svl4-world-layout-locked
  text: 'WIT/package layout for child authoring is locked and documented: toy contracts in wit/toys/deps, child-facing world contracts in wit/child (+ per-child toyboxes/worlds as needed), pipeline composition contracts aligned with typed composition plan.'
  checked: false
- id: svl5-capability-governance
  text: Capability governance is locked to child.toml [needs].toys (+ optional [needs.scopes]) with fail-closed validation in Mother; no ambient capability escalation.
  checked: false
- id: svl6-runtime-composition-contract
  text: 'Mother runtime composition contract is explicit: typed wiring validation, grant audit logging, deterministic failure mode for invalid or unauthorized toy links.'
  checked: false
- id: svl7-sdk-ci-gates
  text: CI gates enforce vision constraints for first-party children/templates (SDK usage, manifest schema, world/package conventions, and typed-vs-legacy lane checks).
  checked: false
- id: svl8-spec-alignment
  text: sdk-developer-platform, child-typed-composition, sdk-public-surface-alignment, and child-construction-canon are aligned to this vision lock with no contradictory guidance.
  checked: false
- id: svl9-migration-playbook
  text: A migration playbook exists for existing handle-based service children and grammar children, with explicit keep/migrate/retire decisions per child (including spec-manager and grammar pipeline lane).
  checked: false
- id: svl10-proof-build
  text: At least one end-to-end pando path is proven under the locked vision using SDK-led authoring and Mother composition validation with passing compile/tests.
  checked: false
- id: svl11-portfolio-disposition
  text: 'Child portfolio disposition is documented and approved: six canon typed children locked as reference baseline, each legacy service child has a migration stance, and grammar lane has a declared long-term contract.'
  checked: false
---
# feat: SDK Vision Lock (BA/WASI Component-First)

> Lock Patina SDK as the canonical MCT developer surface aligned to Bytecode Alliance, WASI, WIT, and component model constraints before further interface/runtime expansion.

## Problem

Patina has multiple concurrent drafts that point in the right direction but still
leave key strategic decisions open: whether SDK or raw bindgen is the primary
authoring path, whether typed toy contracts are default or optional, and how
strictly Mother enforces capability/runtime composition boundaries.

Without an explicit vision lock, future work can drift into mixed execution
models and fragmented developer surfaces.

## Goal

Define and ratify a single forward structure where:

1. SDK is the canonical developer entry point for new first-party child authoring.
2. BA/WASI/component-model standards are non-negotiable baseline constraints.
3. Mother remains strict authority for capability grants and composition safety.
4. Typed toy contracts are default for new data-plane child composition.
5. Legacy service children and grammar children are handled by explicit disposition plans, not drift.

This spec is governance-first: lock the shape, then align implementation specs.

## Status

Draft.

## Non-Goals

- Rewriting all existing children in one pass.
- Delivering full network artifact federation in this spec.
- Redesigning HITL UX/runtime flows (already covered by separate interface specs).
- Replacing the current spec tool before using it to organize this direction.

## Target Shape

### 1) Standards Baseline (hard constraint)

- WASI/component model are the runtime foundation.
- WIT contracts are the extension boundary.
- Patina-specific interfaces are explicit delta, not reinvention of covered WASI scope.

### 2) SDK-First Authoring

- `sdk/patina-sdk` is the first-party child authoring surface for all new children.
- `patina child init` defaults to typed SDK scaffolding; docs/examples match.
- Raw bindgen remains available for advanced/escape-hatch use, not the default path.
- `sdk/patina-sdk-legacy` is maintenance lane only (allowlist-controlled).

### 3) Typed-by-Default Data Plane

- New data-plane children use typed toy imports/exports.
- `handle(action,payload)` lane remains for legacy/control-plane children until migrated.

### 4) Mother-Strict Safety Model

- Mother validates grants from manifest (`[needs].toys`, optional scopes).
- Mother validates typed composition wiring before runtime execution.
- Unauthorized toy links fail closed and are audit-logged.

### 5) Spec-System Discipline

- Existing SDK/typed-composition specs align under this lock.
- Contradictions are removed explicitly, not tolerated as “parallel truths.”
- Drift across specs/docs/templates is a release blocker.

### 6) Child Portfolio Discipline

- Six canon children are the typed baseline reference set.
- Legacy service children get explicit keep/migrate/retire decisions.
- Grammar children get explicit lane decision (typed composition integration vs legacy pipeline containment), with owner and deadline.

## Solution

1. Ratify vision lock statements in this spec (owner-level decisions).
2. Treat this spec as umbrella policy for SDK/typed-composition specs.
3. Align downstream specs and docs to this locked shape.
4. Add CI/policy checks that encode the lock in build-time gates.

## Slice Test Gate (applies to every execution slice)

- Deterministic behavior test proving intended slice outcome.
- Deterministic fail-closed/failure-path test for safety boundaries.
- Fixture conformance lock test when checked-in fixtures are used.
- HITL packet includes exact verification commands and results.
- Default rule: no runtime dependence on local `target/` artifacts for behavior tests.

## Implementation Order

1. **Ratify lock statements** (svl1) and record owner-approved constraints in this file.
2. **Freeze portfolio baseline** (svl11) via `legacy-and-grammar-disposition`: six canon typed children + explicit disposition for legacy service + grammar lanes.
3. **Align SDK surface policy** (svl2, svl7) via `child-init-typed-default` + `sdk-public-surface-alignment`.
4. **Align typed composition safety + audit** (svl3, svl6) via `child-typed-composition` + `mother-grant-audit-coverage`.
5. **Lock world/layout governance** (svl4, svl5) and update docs/templates.
6. **Resolve contradictory spec guidance** (svl8).
7. **Publish migration playbook** (svl9).
8. **Prove one end-to-end path** (svl10).

## Vision Lock Statements (ratified — 2026-04-11)

1. **BA/WASI/component-model-first:** Patina MCT aligns to BA standards and vocabulary as default architecture direction.
2. **SDK is canonical for new first-party child authoring:** examples/templates/guides + `patina child init` route through typed SDK lane.
3. **Typed toy contracts are default for new data-plane composition.**
4. **Mother is strict authority with fail-closed enforcement:** typed wiring validation exists now and audit logging must be completed to full coverage.
5. **Legacy handle lane is transitional/control-plane:** not the preferred path for new data-plane work.
6. **Drift is unacceptable:** spec/doc/template/runtime drift is treated as blocker-level correctness failure.
7. **Portfolio reality is explicit:** six canon children are the typed baseline; legacy service children and grammars require explicit disposition, not implicit carryover.

## Resolved Decisions

- This spec is an umbrella governance spec and should be referenced by downstream implementation specs.
- “Spec drift” across SDK/composition docs/templates/runtime behavior is treated as a blocker, not editorial debt.
- Canon baseline is six typed data-plane children; migration decisions for legacy service children and grammars are mandatory artifacts.

## Verification

```bash
patina spec show sdk-vision-lock
patina spec check sdk-vision-lock --json

# compile baseline
cargo check --workspace -q

# child/sdk alignment checks (targeted tests/scripts added by implementation)
# cargo test --test <sdk-vision-lock-tests>
```

## Exit Criteria

Frontmatter criteria `svl1..svl11` are the source of truth.

## Build Readiness

Medium-High for governance alignment, Medium for implementation alignment.
Most building blocks already exist in draft specs; the primary work is locking
and enforcing one coherent direction before further expansion.
