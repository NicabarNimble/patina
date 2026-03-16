---
type: refactor
id: child-plugin-sdk-alignment
status: complete
created: 2026-03-13
related:
- sdk/patina-sdk/src/lib.rs
- src/plugin/internal/mod.rs
- src/plugin/internal/knowledge_child.rs
- children/ducklake-wasm/src/lib.rs
- children/belief-verifier-wasm/src/lib.rs
exit_criteria:
- id: all-first-party-children-use-single-sdk-surface
  text: First-party children/plugins compile and run with patina-sdk surface only
  checked: true
- id: no-removed-sdk-identifiers-in-active-code
  text: Active source/workflow files contain no removed split-SDK identifiers
  checked: true
- id: grant-contracts-remain-typed-and-enforced
  text: Host grant enforcement and typed toy/connector contracts remain green under tests
  checked: true
- id: ci-guards-stay-green
  text: check-single-sdk-surface and check-crate-names pass with updated set
  checked: true
---
# refactor: align remaining children/plugins to single patina-sdk

> Finish SDK alignment after native DuckLake removal, without changing broader directory layout.

## Problem

SDK consolidation is partially complete, but architecture confidence depends on verifying all active child/plugin paths are aligned and still functional.

## Goal

Ensure all active first-party child/plugin surfaces are consistently built on `patina-sdk` with grant contracts preserved.

## Non-Goals

- Do not move spec/scrape internals.
- Do not move grammar plugin surfaces.
- Do not execute big folder reorg.

## Verification

- `cargo check --workspace`
- `bash resources/scripts/check-single-sdk-surface.sh`
- `bash resources/scripts/check-crate-names.sh`
- `patina spec check child-plugin-sdk-alignment --json`
