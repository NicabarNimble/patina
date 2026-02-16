---
type: belief
id: sanitize-at-data-level-not-just-control-flow
persona: architect
facets: [security, architecture, plugin-system]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-13
revised: 2026-02-13
---

# sanitize-at-data-level-not-just-control-flow

At trust boundaries where untrusted input flows through a callback, enforce policy at the data level (strip/override reserved keys) not just control flow (return Err) — control flow catches explicit violations, data sanitization prevents accidental bypass by downstream code that re-parses the same input.

## Statement

At trust boundaries where untrusted input flows through a callback, enforce policy at the data level (strip/override reserved keys) not just control flow (return Err) — control flow catches explicit violations, data sanitization prevents accidental bypass by downstream code that re-parses the same input.

## Evidence

- [[session-20260213-112528]] - External agent review caught gap: scope enforcement was control-flow only (return Err for all_repos), adding data-level stripping of SCOPE_RESERVED_KEYS before dispatch is strictly stronger (weight: 0.95)
- [[commit-80da2ec7]] - First fix: strip `all_repos` from params JSON before callback (weight: 0.9)
- [[commit-6821a38f]] - Generalized to all scope-reserved keys (`all_repos`, `repo`, `project_root`, `db_path`) with 7 regression tests (weight: 0.9)

## Supports

- [[two-layer-capability-grants]] - data-level sanitization is the third layer of defense in depth
- [[lib-owns-policy-binary-owns-wiring]] - sanitization ensures the callback boundary is trustworthy

## Attacks

<!-- None yet -->

## Attacked-By

- Complexity concern: double-parsing JSON (once for control flow check, once for sanitization) is redundant. Mitigated: extracted into `sanitize_query_params()` which handles both in one pass. The control-flow check (return Err for explicit all_repos=true) provides a clear error message; sanitization is the silent safety net.

## Applied-In

- `src/plugin/internal/command.rs` — `SCOPE_RESERVED_KEYS` constant, `sanitize_query_params()` function called before every dispatch
- `src/plugin/internal/tests.rs` — 5 sanitization tests + 2 kind validation tests lock the invariant

## Revision Log

- 2026-02-13: Created — metrics computed by `patina scrape`
