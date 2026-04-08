---
type: fix
id: mother-runtime-store-rename
status: complete
created: 2026-04-08
related:
  - mother/src/state.rs
  - mother/src/
  - src/
exit_criteria:
  - id: mrs1
    text: "`KnowledgeRuntimeStore` renamed to `MotherRuntimeStore` across the entire codebase — zero references to the old name remain"
    checked: true
  - id: mrs2
    text: "`cargo check --workspace -q` and `cargo test -q --lib` pass"
    checked: true
---
# fix: Rename KnowledgeRuntimeStore to MotherRuntimeStore

## Problem

`KnowledgeRuntimeStore` is a holdover from when Mother's state was part of
the "knowledge system." The struct is Mother's SQLite-backed operational
database (tasks, sessions, lake cursors, project registration, startup
diagnostics). The name should reflect ownership (Mother) and purpose
(runtime store), not the retired "knowledge" domain.

## Scope

Mechanical rename across ~80 references in ~25 files. No behavior changes.

## Non-Goals

- No schema changes.
- No API changes.
- No state migration.
