---
type: refactor
id: mother-http-api-module-split
status: draft
created: 2026-04-14
blocked_by:
  - durable-rust-unix-realignment-program
beliefs:
  - "[[dependable-rust]]"
  - "[[unix-philosophy]]"
related:
  - mother/src/http_api.rs
  - mother/src/http_routes.rs
  - mother/src/lib.rs
  - layer/surface/reports/audit/2026-04-14-durable-rust-unix-realignment-audit.md
exit_criteria:
  - id: mhams1-endpoint-domain-split
    text: "`http_api.rs` is split by endpoint domain (health, lifecycle, child, inspector, rivet, builtins)."
    checked: false
  - id: mhams2-contract-stability
    text: "Request/response JSON contracts remain backward-compatible for existing routes."
    checked: false
  - id: mhams3-route-wiring-parity
    text: "Router wiring remains equivalent; no route capability regression."
    checked: false
  - id: mhams4-tests
    text: "Handler and route tests pass with deterministic coverage for success + fail-closed paths."
    checked: false
---

# refactor: mother http api module split

## Problem

`mother/src/http_api.rs` currently combines API trait surface, endpoint handlers, error mapping, router table wiring, and large test scaffolds.

## Goal

Recover clear API boundaries by splitting endpoint domains into focused modules while preserving route behavior.

## Candidate split

- `mother/src/http_api/core.rs` (traits + shared errors)
- `mother/src/http_api/health.rs`
- `mother/src/http_api/lifecycle.rs`
- `mother/src/http_api/child.rs`
- `mother/src/http_api/inspector.rs`
- `mother/src/http_api/rivet.rs`
- `mother/src/http_api/tests/*`

## Non-goals

- No new endpoints.
- No auth model changes.
- No transport changes.
