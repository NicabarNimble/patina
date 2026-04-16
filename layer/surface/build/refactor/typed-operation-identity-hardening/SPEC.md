---
type: refactor
id: typed-operation-identity-hardening
status: ready
created: 2026-04-14
blocked_by:
- durable-rust-unix-realignment-program
- child-typed-conversion-boundary
related:
- src/child/internal/child.rs
- src/child/runtime.rs
- mother/src/registry.rs
- mother/src/http_api.rs
- src/commands/mother/daemon.rs
- layer/surface/reports/audit/2026-04-14-durable-rust-unix-realignment-audit.md
beliefs:
- '[[dependable-rust]]'
- '[[spec-driven-design]]'
- '[[unix-philosophy]]'
- '[[adapter-pattern]]'
exit_criteria:
- id: toih1-exact-operation-identity
  text: Typed operation lookup uses exact exported interface/function identity; heuristic fallback candidates (`@0.1.0`, underscore↔hyphen swap) are removed from strict path.
  checked: true
- id: toih2-operation-id-validation-hardened
  text: Operation identifier validation is tightened to canonical `<package>:<interface>.<function>` shape with deterministic machine errors for malformed identifiers.
  checked: true
- id: toih3-allowlist-vs-export-validation
  text: Child contract allowlist entries are validated against discovered typed exports at load/startup (or explicit validation command), failing closed on mismatch.
  checked: true
- id: toih4-driver-compat-boundary
  text: Strict typed driver is the canonical production path; compatibility driver modes are explicitly marked transitional and isolated from strict behavior tests.
  checked: true
- id: toih5-errors-structured
  text: Typed identity and lookup failures return stable machine error code + structured detail fields; text remains informational.
  checked: true
- id: toih6-tests
  text: Deterministic tests cover exact-identity success, malformed id failure, unknown export failure, and allowlist/export mismatch failure.
  checked: true
---

# refactor: typed operation identity hardening

## Problem

Typed operation dispatch currently includes convenience heuristics (version alias attempts and function token normalization), and operation authorization remains string-list based without strict export identity validation.

This weakens strict WIT/component-model posture by allowing ambiguous identity resolution behavior.

## Goal

Align typed operation dispatch with strict component-model identity semantics:

- exact export identity matching,
- fail-closed validation,
- deterministic machine-readable failure contracts.

## Scope

- Tighten operation-id parsing/validation.
- Remove heuristic export fallback in strict path.
- Add allowlist/export compatibility validation seam.
- Keep dispatch authority path unchanged (`ChildCallRequest` -> registry policy -> child call).

## Non-goals

- No change to child business contracts.
- No WIT schema redesign.
- No broad transport rewrite.

## Verification commands

```bash
cargo test -p patina-ai child::internal::child -- --nocapture
cargo test -p mother registry::tests -- --nocapture
cargo test -p mother http_api::tests -- --nocapture
```
