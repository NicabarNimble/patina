---
type: fix
id: retire-lake-manager-stub
status: complete
created: 2026-04-08
related:
  - children/lake-manager/
  - children/lakehouse-catalog/
exit_criteria:
  - id: rlms1
    text: "`children/lake-manager/` directory removed from the workspace"
    checked: true
  - id: rlms2
    text: "Workspace Cargo.toml no longer references the lake-manager child crate"
    checked: true
  - id: rlms3
    text: "No remaining references to lake-manager child in src/, mother/, or tests/"
    checked: true
  - id: rlms4
    text: "`cargo check --workspace -q` and `cargo test -q --lib` pass"
    checked: true
---
# fix: Retire lake-manager stub child

## Problem

`children/lake-manager/` is a stub from "phase-5 manager children"
scaffolding. It has two actions (list, create) backed by simple state
writes with no real logic. The lakehouse-catalog canon child handles
actual lake operations (schema evolution, file registration, SQL catalog).
lake-manager was never developed beyond scaffolding and should be retired.

## Scope

1. Remove `children/lake-manager/` directory.
2. Remove workspace member reference from root `Cargo.toml`.
3. Remove any references in CI, preflight, or test files.
4. Verify clean build and tests.

## Non-Goals

- No changes to lakehouse-catalog or other canon children.
