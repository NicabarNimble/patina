---
type: fix
id: spec-archive-read-path
status: active
created: 2026-04-14
blocked_by:
  - durable-rust-unix-realignment-program
beliefs:
  - "[[spec-driven-design]]"
  - "[[dependable-rust]]"
related:
  - src/commands/spec/internal/archive.rs
  - src/commands/spec/internal/queries.rs
  - src/commands/spec/internal/mutations.rs
  - src/spec.rs
  - layer/surface/reports/audit/2026-04-14-durable-rust-unix-realignment-audit.md
exit_criteria:
  - id: sarp1-archived-show-works
    text: "`patina spec show <archived-id>` succeeds by reading archived spec content from tag-backed source-of-truth."
    checked: true
  - id: sarp2-archived-check-works
    text: "`patina spec check <archived-id> --json` succeeds and reports criteria state for archived specs."
    checked: true
  - id: sarp3-disk-spec-regression-guard
    text: "On-disk spec show/check behavior is unchanged and covered by tests."
    checked: true
  - id: sarp4-fail-closed-missing-archive
    text: "Missing/invalid archive tag returns explicit deterministic error (no placeholder path read attempts)."
    checked: true
  - id: sarp5-tests
    text: "Deterministic tests cover archived success path + missing archive failure path."
    checked: true
---

# fix: spec archive read path

## Problem

After archiving a spec into `spec/<id>` tag, read paths still expect filesystem-backed `SPEC.md`, causing show/check failures for archived specs.

## Goal

Restore archived-spec read behavior so archived tags remain first-class queryable history.

## Root cause hypothesis

`find_spec` can return placeholder marker `(archived: spec/<id>)`, but downstream load/show paths treat returned value as filesystem path and call `read_to_string`.

## Target shape

- Unified spec load path understands two sources:
  1. disk (`layer/surface/build/.../SPEC.md`)
  2. archived tag content (`git show spec/<id>:...` or equivalent resolved archive path)
- No fake file paths in flow control.

## Verification commands

```bash
PATINA_SPEC_DIRECT=1 ./target/debug/patina spec show mother-rivet-integration
PATINA_SPEC_DIRECT=1 ./target/debug/patina spec check mother-rivet-integration --json
PATINA_SPEC_DIRECT=1 ./target/debug/patina spec show child-construction-canon
cargo test -p patina-ai commands::spec::internal::archive::tests::load_spec_read_only_reads_archived_tag_content -- --nocapture
cargo test -p patina-ai commands::spec::internal::archive::tests::load_spec_read_only_fails_when_archive_tag_missing_spec_path -- --nocapture
```
