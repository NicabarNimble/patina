---
type: feat
id: spec-atlas-live-server
status: draft
created: 2026-04-11
related:
- layer/surface/build/feat/spec-atlas-mct-visibility/SPEC.md
- src/commands/atlas
- README.md
beliefs:
- '[[sdk-is-mct-entry-point]]'
- '[[children-have-agency-toys-are-capabilities]]'
exit_criteria:
- id: sals1-serve-command
  text: "`patina atlas --serve` runs a local read-only HTTP dashboard server without requiring Mother."
  checked: false
- id: sals2-dashboard-route
  text: "GET `/` serves atlas dashboard HTML backed by current project snapshot."
  checked: false
- id: sals3-json-route
  text: "GET `/atlas.json` serves the same normalized snapshot model used by HTML rendering."
  checked: false
- id: sals4-fail-closed-routes
  text: "Malformed HTTP request lines fail closed (400), unsupported methods fail closed (405), and unknown routes fail closed (404)."
  checked: false
- id: sals5-deterministic-tests
  text: "Deterministic tests cover route parsing/routing behavior and failure paths."
  checked: false
- id: sals6-demo-proof
  text: "Demo walkthrough shows server launch, route retrieval, and browser open command."
  checked: false
---
# feat: Spec Atlas Live Server

> Extend atlas from static artifact generation to a local read-only webserver for live spec+MCT visibility.

## Problem

`patina atlas --html` creates a useful artifact, but review sessions still need manual regeneration and reopen steps.
A local server route improves iterative review and supports command-line + browser workflows from one command.

## Goal

Add a local-only webserver mode:

- `patina atlas --serve`
- default bind `127.0.0.1:7417`
- read-only routes:
  - `/` dashboard HTML
  - `/atlas.json` snapshot JSON
  - `/health`

## Non-Goals

- Mother daemon integration in this slice.
- Authentication/multi-user hosting.
- Persistent process management (launchd/service wiring).

## Verification

```bash
cargo test -q atlas
cargo check -q
patina atlas --serve --port 7417
curl -s http://127.0.0.1:7417/atlas.json | jq '.summary'
```
