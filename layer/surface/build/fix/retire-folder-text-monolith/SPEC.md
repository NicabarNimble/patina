---
type: fix
id: retire-folder-text-monolith
status: complete
created: 2026-04-08
related:
  - children/folder-text-to-parquet/
  - resources/pandos/folder-text-to-parquet/pando.toml
  - layer/surface/build/feat/child-construction-canon/SPEC.md
exit_criteria:
  - id: rtfm1
    text: "`children/folder-text-to-parquet/` directory removed from the workspace"
    checked: true
  - id: rtfm2
    text: "Workspace Cargo.toml no longer references the folder-text-to-parquet child crate"
    checked: true
  - id: rtfm3
    text: "No remaining imports or references to the monolith child in src/, mother/, or tests/"
    checked: true
  - id: rtfm4
    text: "`cargo check --workspace -q` and `cargo test -q --lib` pass"
    checked: true
---
# fix: Retire folder-text-to-parquet monolith child

## Problem

`children/folder-text-to-parquet/` is a monolith child that predates the
pando composition model. The 6 canon children (file-system-monitor,
content-extractor, schema-enforcer, dedup-filter, record-writer,
lakehouse-catalog) were extracted from it, and the
`resources/pandos/folder-text-to-parquet/pando.toml` now composes them.
The monolith is superseded and should be retired.

## Scope

1. Remove `children/folder-text-to-parquet/` directory.
2. Remove workspace member reference from root `Cargo.toml`.
3. Remove any test or code references to the monolith child.
4. Verify clean build and tests.

## Non-Goals

- The pando at `resources/pandos/folder-text-to-parquet/` is kept — that
  is the canonical composition.
- No changes to the 6 canon children.
