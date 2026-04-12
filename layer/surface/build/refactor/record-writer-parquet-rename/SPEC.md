---
type: refactor
id: record-writer-parquet-rename
status: active
created: 2026-04-11
related:
- children/record-writer/
- resources/pandos/folder-text-to-parquet/pando.toml
- tests/pando_parity.rs
- layer/core/values/spec-driven-design.md
beliefs:
- '[[unix-philosophy]]'
- '[[dependable-rust]]'
- '[[children-have-agency-toys-are-capabilities]]'
exit_criteria:
- id: rwpr1-runtime-name-rename
  text: "Child runtime identity is renamed from `record-writer` to `parquet-writer` (manifest `[child].name`) to match actual behavior."
  checked: false
- id: rwpr2-crate-and-artifact-rename
  text: "Child crate/package/artifact naming is aligned to parquet terminology (`patina-ai-child-parquet-writer`), with build/test references updated."
  checked: false
- id: rwpr3-pando-wiring-updated
  text: "`folder-text-to-parquet` pando child list/wiring references `parquet-writer` without behavior change."
  checked: false
- id: rwpr4-compat-alias-window
  text: "Compatibility alias for `record-writer` is provided for one migration window (clear deprecation note + removal target)."
  checked: false
- id: rwpr5-docs-and-fixtures
  text: "Docs, fixtures, and test names stop presenting `record-writer` as generic writer and reflect parquet-specific role."
  checked: false
- id: rwpr6-proof
  text: "`cargo check --workspace -q`, `cargo test -q pando_parity`, and a local pando run path pass with renamed child identity."
  checked: false
---
# refactor: Rename record-writer to parquet-writer

## Problem

`record-writer` implies generic sink behavior, but the implementation is explicitly parquet-oriented
(Arrow schema + parquet writer + parquet output naming). The current name hides a concrete boundary,
which increases architecture ambiguity when evaluating child seams and composition strategy.

## Goal

Rename the child to **`parquet-writer`** so name reflects actual responsibility, while preserving
pipeline behavior and a short compatibility window.

## Scope

- Child runtime identity, crate/package/artifact naming.
- Pando manifests and wiring references.
- Tests/fixtures/docs referencing the old name.
- One migration alias window for existing local installs/scripts.

## Non-goals

- Changing `patina:records/write` WIT contract shape.
- Splitting/merging pipeline stages in this slice.
- Reworking adapter strategy in this slice.

## Verification

```bash
cargo check --workspace -q
cargo test -q pando_parity

# verify pando manifest/wiring uses parquet-writer
rg "record-writer|parquet-writer" resources/pandos/folder-text-to-parquet/pando.toml children/ tests/
```
