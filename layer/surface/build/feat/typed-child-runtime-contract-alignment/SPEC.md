---
type: feat
id: typed-child-runtime-contract-alignment
status: active
created: 2026-04-29
updated: 2026-04-29
sessions:
  origin: 20260428-230202-450986000
related:
- layer/surface/build/feat/child-registry-control-plane-remaining/SPEC.md
- layer/surface/build/feat/slate-pando-migration/SPEC.md
- children/slate-manager/child.toml
- src/child/internal/child.rs
- src/commands/mother/daemon/dispatch.rs
beliefs:
- '[[spec-driven-design]]'
- '[[safety-boundaries]]'
- '[[dependable-rust]]'
- '[[explicit-fail-closed-over-hidden-fallbacks]]'
references:
- layer/core/values/spec-driven-design.md
- layer/core/values/safety-boundaries.md
- layer/core/values/dependable-rust.md
exit_criteria:
- id: tca-r1-versioned-operation-contract
  text: Operation IDs for Slate typed routing are versioned and consistent across child contract manifest, Mother dispatch mapping, and tests (`patina:slate/control@0.1.0.*`).
  checked: false
- id: tca-r2-typed-load-contract
  text: Mother can load Slate as a typed-first child without silent fallback; contract validation remains fail-closed and explicit on mismatch.
  checked: false
- id: tca-r3-routed-execute-proof
  text: Routed execute proof succeeds end-to-end for Slate (`PATINA_SPEC_BACKEND=execute ...`) after external release sync/approve/install/assign.
  checked: false
- id: tca-r4-regression-coverage
  text: Deterministic tests cover contract export validation and dispatch operation mapping to prevent unversioned-ID or export-shape regressions.
  checked: false
---
# feat: Typed child runtime contract alignment for Mother

> Define and implement typed child runtime contract support so Mother can load and route typed-first Slate children without legacy runtime-shape mismatch, unblocking external Slate routed usage proof.

## Problem

External Slate control-plane flow now works through sync/approve/install/assign, but execute-mode routed usage still fails because runtime contract expectations and typed operation declarations are not fully aligned.

## Goal

Make typed contract identity explicit and consistent end-to-end so Slate loads and executes through Mother without weakening fail-closed guarantees.

## Status

- External Slate release pipeline exists and is publishing assets.
- Current blocker is routed runtime activation/proof, not registry control-plane transitions.

## Non-Goals

- No fail-open behavior.
- No SDK surface expansion in this spec.
- No broad refactor outside Slate typed contract path.

## Target Shape

1. One canonical, versioned typed operation ID format.
2. Child contract declaration and Mother routing map match exported component operations.
3. Routed execute path succeeds with external Slate release artifacts.
4. Mismatches still fail closed with explicit reasons.

## Solution

- Normalize and enforce versioned typed operation IDs across child manifest and Mother dispatch routing.
- Align loader/runtime contract checks with typed-first Slate contract while preserving strict validation.
- Re-run external Slate release proof after alignment.

## Implementation Order

1. Contract ID normalization (`@0.1.0`) in child manifest + Mother dispatch.
2. Runtime contract validation/load path alignment for typed Slate.
3. Regression tests for export/operation mapping.
4. External proof rerun + artifact updates.

## Resolved Decisions

- Keep explicit fail-closed behavior on contract mismatch.
- Treat typed operation IDs as versioned API contracts.
- Keep scope tightly focused on unblocking routed execute proof.

## Verification

```bash
cargo fmt --all
cargo check -q
cargo test -p patina-ai commands::mother::daemon::tests --quiet
cargo test -p patina-ai child::internal::tests --quiet
PATINA_SPEC_BACKEND=execute cargo run -q -- spec next --json
patina spec check typed-child-runtime-contract-alignment --json
```

## Exit Criteria

Frontmatter `tca-r1..tca-r4` is the source of truth.

## Build Readiness

- Current breakage is isolated and reproducible.
- Scope is small enough for rapid completion and immediate hand-back to [[child-registry-control-plane-remaining]] step-5 closure.
