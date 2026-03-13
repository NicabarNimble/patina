---
type: fix
id: clippy-backlog-batch-1
status: active
created: 2026-03-13
sessions:
  origin: 20260312-160150
exit_criteria: []
---
# fix: clippy backlog batch 1

> Reduce low-risk clippy warnings blocking strict CI

## Problem

Strict clippy (`-D warnings`) is failing on a broad lint backlog, preventing clean verification and push confidence.

## Goal

Land a first low-risk batch of no-behavior-change lint fixes that measurably reduces backlog and keeps tests green.

## Status

Active.

## Non-Goals

- Exhaustively clear all clippy warnings in one pass.
- Refactor production behavior or change runtime semantics.

## Solution

Apply targeted mechanical fixes (test assertions, style lints, small API-usage improvements) in the smallest safe slices, validating each batch with clippy + tests.

## Implementation Order

1. Fix deterministic low-risk lints in tests and helper code.
2. Re-run strict clippy to capture remaining backlog.
3. Fix one additional low-risk cluster.
4. Re-run clippy and bin tests.

## Resolved Decisions

- Scope this batch to low-risk mechanical edits only.
- Prefer test-only and formatting-equivalent changes first.

## Verification

- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --bin patina`

## Build Readiness

Ready for low-risk lint cleanup implementation.

## Exit Criteria

- [ ] At least one focused lint cluster is fixed without behavior changes.
- [ ] Strict clippy backlog count is reduced from current baseline.
- [ ] `cargo test --bin patina` passes after edits.
