---
type: feat
id: legacy-bridge-mother-api
status: draft
created: 2026-04-11
related:
- layer/surface/build/feat/legacy-typed-bridge-seam/SPEC.md
- mother/src/http_api.rs
- mother/src/http_routes.rs
- src/commands/mother/daemon.rs
- src/mother/internal.rs
beliefs:
- '[[children-have-agency-toys-are-capabilities]]'
- '[[core-verbs-standalone-mother-additive]]'
exit_criteria:
- id: lbma1-route
  text: "Mother exposes authenticated bridge translate endpoint (`POST /api/bridge/translate`)."
  checked: false
- id: lbma2-runtime
  text: "Mother runtime resolves bridge translate via mother-managed bridge module fail-closed policy."
  checked: false
- id: lbma3-client
  text: "Mother client supports bridge translate over UDS first with TCP+token fallback."
  checked: false
- id: lbma4-tests
  text: "Deterministic tests validate request parsing and response envelope for bridge translate route."
  checked: false
- id: lbma5-demo
  text: "Demo includes UDS request showing allow and deny verdicts."
  checked: false
---
# feat: Legacy Bridge Mother API

> Add a dedicated Mother control-plane endpoint for legacy-to-typed bridge translation.

## Problem

Bridge seam policy exists, but there is no dedicated Mother API surface for interfaces/tools to request translation decisions.

## Goal

Expose bridge translation as an authenticated Mother endpoint with deterministic fail-closed behavior and client access.

## Verification

```bash
cargo check -q
cargo test -q -p mother bridge_translate
```
