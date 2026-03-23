---
type: refactor
id: ducklake-native-removal-and-verification
status: complete
created: 2026-03-13
updated: 2026-03-13
related:
- layer/surface/build/refactor/ducklake-knowledge-child-cutover/SPEC.md
- layer/surface/build/refactor/mother-child-toy-beliefs-layout/SPEC.md
exit_criteria:
- id: final-parity-snapshot-captured
  text: One explicit parity snapshot (rows, cursor semantics, failure behavior) is captured before legacy native deletion
  checked: true
- id: wasm-path-meets-parity-assertions-before-deletion
  text: Knowledge-child DuckLake path passes parity assertions derived from the snapshot (rows, cursor progression, and failure handling) before native deletion lands
  checked: true
- id: cursor-and-checkpoint-migration-proven
  text: Legacy native cursor/checkpoint state is migrated or honored by the knowledge-child path with tests proving no silent re-ingest or continuity loss
  checked: true
- id: native-ducklake-runtime-removed
  text: Legacy native DuckLake runtime implementation is removed from runtime and workspace membership; only the knowledge-child runtime path remains
  checked: true
- id: knowledge-child-route-only
  text: Broker lake route only uses knowledge-child path with no native fallback path
  checked: true
- id: no-legacy-ducklake-runtime-references
  text: Runtime/workspace/test references to the legacy native DuckLake runtime path are removed (excluding archived history specs/sessions)
  checked: true
- id: ci-blocks-native-reintroduction
  text: CI has a guard that fails if native children/ducklake runtime path is reintroduced
  checked: true
---
# refactor: remove native ducklake runtime and harden wasm-only path

> Remove the legacy native DuckLake runtime, keep the knowledge-child runtime path as the only truth, and lock regressions with CI.

## Historical Note

This lane executed before the follow-up path rename to `children/ducklake`.
References to `children/ducklake-wasm` below reflect the original execution
frame and now correspond to the canonical `children/ducklake` knowledge-child
path.

## Problem

DuckLake had dual runtime identity during this migration lane, which kept
migration complexity and boundary drift alive.

## Goal

Make knowledge-child DuckLake the only runtime path and verify behavior after native deletion.

## Non-Goals

- Do not rename the knowledge-child path in this spec.
- Do not perform broad folder reorg in this spec.
- Do not move spec/scrape/grammar surfaces in this spec.

## Verification

- `cargo check --workspace`
- `patina spec check ducklake-native-removal-and-verification --json`
- `cargo test -q -p patina-ai migration_copies_legacy_cursor_into_per_type_lake_cursors`
- `cargo test -q -p patina-ai migration_is_idempotent_and_does_not_overwrite_existing_cursor`
- parity suite command for wasm path vs snapshot is defined and green in CI before native deletion commit lands
- `rg "children/ducklake-wasm" src children sdk tests Cargo.toml .github` (no active runtime references)
