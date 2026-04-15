---
type: refactor
id: mother-daemon-module-split
status: active
created: 2026-04-14
blocked_by:
  - durable-rust-unix-realignment-program
beliefs:
  - "[[unix-philosophy]]"
  - "[[dependable-rust]]"
  - "[[safety-boundaries]]"
related:
  - src/commands/mother/daemon.rs
  - src/commands/mother/mod.rs
  - src/commands/mother/daemon/startup.rs
  - src/commands/mother/daemon/transport.rs
  - src/commands/mother/daemon/dispatch.rs
  - src/commands/mother/daemon/composition.rs
  - src/commands/mother/daemon/health.rs
  - mother/src/http_api.rs
  - layer/surface/reports/audit/2026-04-14-durable-rust-unix-realignment-audit.md
exit_criteria:
  - id: mdms1-module-split
    text: "`daemon.rs` responsibilities are split into startup, transport, dispatch, composition, and health modules."
    checked: true
  - id: mdms2-runtime-behavior-parity
    text: "Warmup, profile handling, rivet dispatch, and typed wiring behavior remain unchanged."
    checked: true
  - id: mdms3-test-surface-preserved
    text: "Existing daemon test coverage is preserved; moved tests keep equivalent assertions."
    checked: true
  - id: mdms4-fail-closed-regression-guard
    text: "Fail-closed paths (`invalid_request`, `operation_in_progress`, `resource_exhausted`) remain deterministic."
    checked: true
---

# refactor: mother daemon module split

## Problem

`src/commands/mother/daemon.rs` has accumulated startup, lifecycle, composition, dispatch, and extensive tests in one file.

## Goal

Improve maintainability and reviewability by splitting module responsibilities without changing externally observable behavior.

## Candidate split

- `src/commands/mother/daemon/startup.rs`
- `src/commands/mother/daemon/dispatch.rs`
- `src/commands/mother/daemon/composition.rs`
- `src/commands/mother/daemon/health.rs`
- `src/commands/mother/daemon/tests/*.rs`

## Non-goals

- No policy changes.
- No endpoint contract changes.
- No profile semantic changes.

## Verification

```bash
cargo check -p patina-ai
cargo test -p patina-ai commands::mother::daemon::tests -- --nocapture
cargo run -q -- mother --help
```
