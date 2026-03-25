---
type: refactor
id: sdk-mother-child-retirement
status: complete
created: 2026-03-25
related:
  - sdk/patina-sdk/src/mother_child.rs
  - sdk/patina-sdk/src/lib.rs
  - plugins/models/
  - plugins/repos/
  - layer/surface/build/refactor/greenfield-mother-clean-continued/SPEC.md
exit_criteria:
  - id: SMCR1
    text: `sdk/patina-sdk/src/mother_child.rs` deleted and `MotherChild` no longer exported by `patina-sdk`
    checked: true
  - id: SMCR2
    text: `plugins/models/` and `plugins/repos/` are either retired or migrated off `register_mother_child!`
    checked: true
  - id: SMCR3
    text: Breaking SDK change is documented (release note + migration note)
    checked: true
  - id: SMCR4
    text: `cargo build` and `cargo test` pass after retirement changes
    checked: true
---
# refactor: retire SDK mother-child compatibility lane

> Remove remaining `MotherChild` compatibility APIs from `patina-sdk` and clear the last legacy plugin users.

## Problem

`greenfield-mother-clean-continued` completed runtime separation in Mother/daemon surfaces, but `MotherChild` still exists in SDK compatibility code (`sdk/patina-sdk/src/mother_child.rs`) and legacy plugin crates (`plugins/models`, `plugins/repos`). This leaves a conceptual and API seam that no longer maps to active runtime architecture.

## Goal

Fully retire the SDK-level `MotherChild` lane so the public extension API aligns with current child architecture.

## Non-Goals

- Broad plugin->child vocabulary sweep across docs, sessions, and historical artifacts.
- Rewriting archived session/spec history.

## Plan

### SMCR-G1: Remove SDK MotherChild API
- Delete `sdk/patina-sdk/src/mother_child.rs`
- Remove related re-exports from `sdk/patina-sdk/src/lib.rs`
- Keep other SDK surfaces unchanged

### SMCR-G2: Retire or migrate legacy plugin crates
- `plugins/models`: retire (legacy v0.17 lane) or migrate to supported child kind
- `plugins/repos`: retire or migrate off `register_mother_child!`
- Prefer retirement if no active runtime dependency remains

### SMCR-G3: Document breaking change
- Add migration note to SDK docs/changelog surface
- Clearly call out removal of `MotherChild` + replacement path

### SMCR-G4: Verify
- `cargo build -q`
- `cargo test -q`
- quick daemon smoke (`patina mother start`, `/health`, `patina mother stop`)

## Build Readiness

Completed. Legacy SDK MotherChild lane removed; models/repos retired from active workspace; build, tests, and daemon health smoke all pass.

## Residual Historical Surface

- `plugins/models/` and `plugins/repos/` remain in-repo as non-workspace historical directories.
- Their source files may still contain `MotherChild` symbols until the planned physical directory removal slice lands.
- This does not affect runtime/build surfaces because these crates are no longer workspace members.
