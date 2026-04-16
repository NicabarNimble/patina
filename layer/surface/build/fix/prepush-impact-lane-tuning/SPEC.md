---
type: fix
id: prepush-impact-lane-tuning
status: active
created: 2026-04-15
beliefs:
  - "[[spec-driven-design]]"
  - "[[safety-boundaries]]"
  - "[[dependable-rust]]"
related:
  - resources/git/pre-push-checks.sh
  - resources/git/pre-push-targeted-cargo.sh
  - resources/git/README.md
  - layer/surface/reports/audit/2026-04-14-durable-rust-unix-realignment-audit.md
exit_criteria:
  - id: pilt1-docs-only-skip-cargo
    text: "Tier 2 skips cargo clippy/tests when changed files are non-cargo-impacting (e.g., docs/spec markdown-only changes)."
    checked: true
  - id: pilt2-fail-closed-unresolved-impact
    text: "If a cargo-impacting path cannot be resolved to a package, Tier 2 escalates fail-closed to workspace clippy/tests."
    checked: true
  - id: pilt3-trigger-checks-preserved
    text: "Path-triggered checks (DuckLake parity/schema consistency) remain enforced even when cargo lane is skipped."
    checked: true
  - id: pilt4-deterministic-behavior-locks
    text: "Deterministic script-level behavior locks cover docs-only skip, unresolved-path escalation, and trigger-only schema path."
    checked: true
---

# fix: pre-push impact lane tuning

## Problem

Tier 2 pre-push currently escalates to workspace cargo checks whenever changed files cannot be mapped to a crate. This includes many non-code-only edits (docs/specs), making push lanes slow enough to incentivize bypass behavior.

## Goal

Reduce pre-push runtime for non-code changes while preserving mandatory fail-closed guard coverage for cargo-impacting and trigger-sensitive paths.

## Scope

- Add cargo-impact classification to Tier 2.
- Skip cargo lane for non-cargo-impacting changes.
- Preserve fail-closed escalation for unresolved cargo-impacting paths.
- Keep path-triggered checks active regardless of cargo lane skip.
- Add deterministic behavior-lock script.

## Non-goals

- No CLI command/flag contract changes.
- No reduction of Tier 1 structural checks.
- No merge-gate semantics change in Tier 3 full lane.

## Verification commands

```bash
bash resources/git/test-pre-push-targeted-cargo.sh
```
