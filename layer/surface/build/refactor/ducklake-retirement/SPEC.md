---
type: refactor
id: ducklake-retirement
status: complete
created: 2026-04-02
sessions:
  origin: 20260402-135124-249836000
beliefs:
  - "[[five-boundaries-no-overlap]]"
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[core-verbs-standalone-mother-additive]]"
related:
  - children/ducklake/
  - src/mother/broker/mod.rs
  - src/commands/connect.rs
  - src/commands/scrape/mod.rs
  - src/commands/mother/mod.rs
  - src/child/internal/tests.rs
exit_criteria:
  - id: dr1-broker-decoupled
    text: "`src/mother/broker/mod.rs` no longer hardcodes `patina-ducklake`, no longer loads `children/ducklake`, and no longer requires `ducklake.sync` checkpoints."
    checked: true
  - id: dr2-connect-decoupled
    text: "`src/commands/connect.rs` binding management no longer writes DuckLake-namespaced runtime keys or enforces DuckLake-specific binding constraints."
    checked: true
  - id: dr3-source-run-still-works
    text: "`patina mother source run` and on-scrape source triggering keep working through non-DuckLake paths."
    checked: true
  - id: dr4-child-removed
    text: "`children/ducklake/` removed from active runtime usage and repository build surface."
    checked: true
  - id: dr5-tests-cleaned
    text: "DuckLake-specific tests and fixtures are removed or rewritten to generic source-routing/federation cases."
    checked: true
  - id: dr6-terminology-clean
    text: "User-facing messages, docs, and errors no longer instruct users to build/install DuckLake child."
    checked: true
  - id: dr7-proof
    text: "`cargo check --workspace -q`, `cargo test -q --lib`, and source-run smoke coverage pass without DuckLake child dependency."
    checked: true
---

# refactor: DuckLake Retirement

## Problem

`patina-ducklake` was an experiment but still anchors active broker/source
paths. This creates architectural drift: canon children are reusable building
blocks, yet production source routing depends on a legacy monolithic child lane.

## Goal

Remove all active runtime dependency on `patina-ducklake` before further
architecture refactors. Keep source run behavior working while migrating to
Mother-managed federation substrate and canon-compatible composition.

## Non-Goals

- Rebuilding full federation feature set in this spec.
- Finalizing child kind/engine consolidation in this spec.

## Required Cut Points

1. Broker/source execution path
2. Connect binding state shape
3. Mother source CLI and scrape trigger integration
4. DuckLake child crate and tests

## Verification

```bash
patina spec check ducklake-retirement --json
cargo check --workspace -q
cargo test -q --lib
```

## Build Readiness

Ready. This is an explicit prerequisite for follow-on refactors to avoid
carrying legacy ducklake coupling into the new Mother/child design.
