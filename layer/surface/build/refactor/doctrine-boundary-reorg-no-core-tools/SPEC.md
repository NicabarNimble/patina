---
type: refactor
id: doctrine-boundary-reorg-no-core-tools
status: complete
created: 2026-03-13
related:
- layer/surface/build/refactor/mother-child-toy-beliefs-layout/SPEC.md
- src/lib.rs
exit_criteria:
- id: boundary-roots-added-with-compiling-shims
  text: beliefs/mother/child/toys boundary roots exist with compile-safe module shims
  checked: true
- id: core-tools-and-scrape-layout-unchanged
  text: spec and scrape-code remain in current command surfaces for this phase
  checked: true
- id: grammar-surfaces-unchanged
  text: grammar plugin locations and loading surfaces are not moved in this phase
  checked: true
- id: boundary-drift-checks-added
  text: CI contains explicit checks to prevent regression of boundary roots and removed legacy runtime paths
  checked: true
---
# refactor: doctrine folder reorg in small slices (exclude core-tools)

> Perform the folder/module boundary reorg after runtime stabilization, but defer core-tools extraction.

## Problem

The architecture wants explicit beliefs/Mother/Child/Toy boundaries, but doing this before runtime and SDK stabilization increases risk.

## Goal

Apply boundary reorg in minimal compile-safe slices while intentionally deferring core-tools/spec/scrape moves.

## Non-Goals

- Do not move `src/commands/spec/**` in this spec.
- Do not move `src/commands/scrape/**` in this spec.
- Do not move grammar plugin directories in this spec.

## Verification

- `cargo check --workspace`
- `patina spec check doctrine-boundary-reorg-no-core-tools --json`
