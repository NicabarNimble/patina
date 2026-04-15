---
type: refactor
id: main-command-router-split
status: active
created: 2026-04-14
blocked_by:
  - durable-rust-unix-realignment-program
beliefs:
  - "[[unix-philosophy]]"
  - "[[dependable-rust]]"
related:
  - src/main.rs
  - src/commands/
  - layer/surface/reports/audit/2026-04-14-durable-rust-unix-realignment-audit.md
exit_criteria:
  - id: mcrs1-main-thin-shell
    text: "`main` is reduced to bootstrap + top-level dispatch shell; command-family routing is moved into dedicated modules."
    checked: false
  - id: mcrs2-command-parity
    text: "CLI behavior parity is preserved for representative command families (mother/spec/scrape/measure/project)."
    checked: false
  - id: mcrs3-no-behavioral-scope-creep
    text: "Refactor introduces no new command features/flags."
    checked: false
  - id: mcrs4-tests
    text: "Deterministic routing tests validate old/new dispatch equivalence for selected command payloads."
    checked: false
---

# refactor: main command router split

## Problem

`src/main.rs` currently concentrates too much routing logic, reducing readability and increasing change blast radius.

## Goal

Recover Unix-style command composition by decomposing top-level routing into focused modules while preserving behavior.

## Candidate module split

- `src/main_dispatch/mod.rs`
- `src/main_dispatch/mother.rs`
- `src/main_dispatch/spec.rs`
- `src/main_dispatch/scrape.rs`
- `src/main_dispatch/child.rs`
- `src/main_dispatch/dev.rs`

## Non-goals

- No command UX changes.
- No option renames.
- No protocol behavior changes.

## Verification

```bash
cargo check -p patina-ai
cargo test -p patina-ai main_dispatch::tests -- --nocapture
cargo run -q -- mother --help
cargo run -q -- spec --help
cargo run -q -- scrape --help
cargo run -q -- measure --help
cargo run -q -- init --help
```
