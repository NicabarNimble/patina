---
type: feat
id: pando-delivery-policy
status: active
created: 2026-04-13
sessions:
  origin: 20260413-210000-000000000
beliefs:
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[wasi-is-foundation-not-option]]"
related:
  - mother/src/pando.rs
  - src/commands/mother/daemon.rs
  - resources/pandos/folder-text-to-parquet/pando.toml
  - layer/core/values/safety-boundaries.md
  - layer/core/values/spec-driven-design.md
exit_criteria:
  - id: pdp1-manifest-policy-shape
    text: "Typed pando wiring supports `delivery = required|best-effort|dead-letter` with default `required`, plus optional `[composition.dead-letter]` target."
    checked: true
  - id: pdp2-runtime-policy-enforcement
    text: "Composer enforces `required` as fail-closed, drops `best-effort` routes on incompatibility, and attempts dead-letter reroute for `dead-letter` policy."
    checked: true
  - id: pdp3-audit-visibility
    text: "Typed wiring audit records include policy-aware reasons for grants/denies and dead-letter reroute outcomes."
    checked: true
  - id: pdp4-tests
    text: "Parser tests cover delivery/dead-letter schema and daemon tests cover dead-letter reroute behavior."
    checked: true
---
# feat: pando delivery policy

## Problem

Typed wiring currently treated every mismatch as hard failure. Actor-style routing needs explicit per-edge semantics:

- `required`: hard fail
- `best-effort`: drop route, continue
- `dead-letter`: reroute to configured sink when primary route fails

## Goal

Add policy-aware typed wiring semantics without introducing Mother domain logic.

## Semantics

- Default policy is `required` when omitted.
- `best-effort` never aborts composition for that edge.
- `dead-letter` attempts reroute to `[composition.dead-letter]` using dead-letter toy (or source edge toy if omitted).
- Missing dead-letter config remains fail-safe via deny audit (no silent success).

## Verification

```bash
cargo test -p mother pando::tests::parses_typed_wiring_delivery_policy -- --nocapture
cargo test -p mother pando::tests::parses_composition_dead_letter_target -- --nocapture
cargo test -p patina-ai typed_wiring_dead_letter_reroutes_when_primary_target_missing -- --nocapture
cargo test -p patina-ai typed_wiring_unknown_from_emits_deny_audit_event -- --nocapture
```
