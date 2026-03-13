---
type: fix
id: ducklake-child-path-name
status: draft
created: 2026-03-13
sessions:
  origin: 20260313-061738
exit_criteria:
- id: canonical-path-renamed
  text: Canonical DuckLake knowledge-child path is children/ducklake
  checked: false
- id: no-wasm-suffix-references
  text: No runtime/test/workspace references remain to children/ducklake-wasm
  checked: false
- id: legacy-native-path-explicit
  text: Legacy native child path is moved to explicit legacy location/name
  checked: false
- id: verification-green
  text: Workspace and ducklake child verification commands pass after rename
  checked: false
---
# fix: fix: rename ducklake-wasm child path to ducklake

> Align child naming with doctrine by removing wasm suffix from canonical ducklake child path

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
move the existing native legacy child to an explicit legacy name/path (for
example `children/ducklake-native`). Update workspace/runtime/tests/spec
references accordingly.

### Move Matrix

- `children/ducklake-wasm` -> `children/ducklake`
- existing native `children/ducklake` -> `children/ducklake-native` (or another
  explicitly legacy path agreed at implementation time)

### Verification

- `cargo check --workspace`
- `cargo build --target wasm32-wasip2 -p patina-plugin-ducklake`
- `cargo test -q -p patina-ai -- src/plugin/internal/tests.rs`
- `rg "children/ducklake-wasm" src children sdk tests Cargo.toml` returns no matches
- `patina spec check ducklake-child-path-name --json`

## Exit Criteria

Use frontmatter exit_criteria as source of truth.
