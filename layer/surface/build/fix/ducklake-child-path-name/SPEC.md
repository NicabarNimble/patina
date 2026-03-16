---
type: fix
id: ducklake-child-path-name
status: complete
created: 2026-03-13
updated: 2026-03-13
sessions:
  origin: 20260313-061738
blocked_by: []
related:
- layer/surface/build/refactor/ducklake-native-removal-and-verification/SPEC.md
exit_criteria:
- id: canonical-path-renamed
  text: Canonical DuckLake knowledge-child path is children/ducklake
  checked: true
- id: no-wasm-suffix-references
  text: No runtime/test/workspace references remain to children/ducklake-wasm
  checked: true
- id: legacy-native-path-explicit
  text: Legacy native child runtime path remains removed with no fallback native location introduced
  checked: true
- id: verification-green
  text: Workspace and ducklake child verification commands pass after rename
  checked: true
---
# fix: fix: rename ducklake-wasm child path to ducklake

> Align child naming with doctrine by removing wasm suffix from canonical ducklake child path

## Status Note

Native DuckLake removal and wasm-only stabilization are now complete, so this
rename is unblocked and ready for execution.

## Problem

Child naming doctrine says children are children; canonical identity paths
should not include runtime implementation suffixes. The current path
`children/ducklake-wasm` leaks implementation detail and invites drift.

## Root Cause

During the plugin-to-child path realignment we used `-wasm` to avoid collision
with the existing native legacy child path at `children/ducklake`. That
temporary compromise now conflicts with the desired naming model.

## Fix

Rename canonical DuckLake knowledge-child directory to `children/ducklake` and
update workspace/runtime/tests/spec references accordingly while preserving the
native-runtime removal decision from the prior spec.

### Move Matrix

- `children/ducklake-wasm` -> `children/ducklake`
- no native fallback path introduced

### Verification

- `cargo check --workspace`
- `cargo build --target wasm32-wasip2 -p patina-ai-child-ducklake`
- `bash resources/scripts/check-ducklake-parity.sh`
- `rg "children/ducklake-wasm" src children sdk tests Cargo.toml` returns no matches
- `patina spec check ducklake-child-path-name --json`

## Exit Criteria

Use frontmatter exit_criteria as source of truth.
