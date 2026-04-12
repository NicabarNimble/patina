---
type: feat
id: spec-atlas-mother-backplane
status: draft
created: 2026-04-11
related:
- layer/surface/build/feat/spec-atlas-mct-visibility/SPEC.md
- layer/surface/build/feat/spec-atlas-live-server/SPEC.md
- mother/src/http_api.rs
- src/commands/mother/daemon.rs
- src/mother/internal.rs
beliefs:
- '[[core-verbs-standalone-mother-additive]]'
- '[[children-have-agency-toys-are-capabilities]]'
exit_criteria:
- id: samb1-mother-api-route
  text: "Mother HTTP API exposes authenticated atlas snapshot route (`GET /api/atlas/snapshot`)."
  checked: true
- id: samb2-runtime-atlas-source
  text: "Mother runtime resolves atlas snapshot from project truth via atlas command module (single normalized model)."
  checked: true
- id: samb3-cli-wrapper-fallback
  text: "`patina atlas` uses Mother snapshot when available and falls back to local snapshot when unavailable."
  checked: true
- id: samb4-fail-closed
  text: "Mother snapshot failures return explicit API errors; CLI fallback path is deterministic and non-panicking."
  checked: true
- id: samb5-tests
  text: "Deterministic tests cover atlas request parsing/routing/failure paths and existing atlas local tests remain green."
  checked: true
- id: samb6-demo
  text: "Demo walkthrough shows Mother snapshot retrieval and atlas server behavior with Mother-backed data path."
  checked: true
---
# feat: Spec Atlas Mother Backplane

> Move atlas snapshot computation into a Mother-managed API lane while keeping CLI as thin wrapper/fallback surface.

## Problem

Atlas currently computes locally in the CLI path. That is useful for standalone operation,
but we also need a Mother-managed control-plane source so multiple interfaces can consume
a single runtime authority surface.

## Goal

- Add Mother atlas snapshot API route.
- Resolve snapshot through the same normalized atlas model.
- Keep CLI additive: prefer Mother when available, local fallback otherwise.

## Verification

```bash
cargo test -q atlas
cargo check -q

# with mother running
curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/api/atlas/snapshot | jq '.summary'
```
