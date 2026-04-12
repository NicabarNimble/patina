---
type: feat
id: atlas-native-mother-spec-lens
status: active
created: 2026-04-11
related:
- layer/surface/build/feat/sdk-vision-lock/SPEC.md
- layer/surface/build/feat/spec-atlas-mother-backplane/SPEC.md
- layer/surface/build/feat/atlas-pando-mother-ui/SPEC.md
- mother/src/http_api.rs
- mother/src/http_routes.rs
- src/commands/atlas/
- ui/atlas/
beliefs:
- '[[core-verbs-standalone-mother-additive]]'
- '[[sdk-is-mct-entry-point]]'
- '[[children-have-agency-toys-are-capabilities]]'
exit_criteria:
- id: anms1-native-mother-module
  text: "Atlas spec lens logic lives in Mother-native module(s), not CLI-local builder internals."
  checked: false
- id: anms2-spec-only-scope
  text: "Atlas payload/model scope is spec-only (inventory, status, criteria, dependencies, lifecycle plan); child/toy inventory is removed from atlas output."
  checked: false
- id: anms3-api-contract
  text: "Mother exposes stable spec-lens API routes (`/api/atlas/specs` and web alias route) with deterministic JSON schema locks."
  checked: false
- id: anms4-cli-thin-client
  text: "`patina atlas` becomes a thin Mother client surface (no local snapshot rebuild path); explicit fail-closed error when Mother unavailable."
  checked: false
- id: anms5-sveltekit-lens
  text: "Atlas web UI is Svelte/SvelteKit-driven and consumes Mother atlas spec-lens API contract."
  checked: false
- id: anms6-tests-and-fail-closed
  text: "Deterministic tests cover success + fail-closed paths (invalid spec, malformed request, unavailable Mother, schema drift)."
  checked: false
- id: anms7-hitl-demo
  text: "Demo packet shows Mother-started atlas spec lens, API retrieval, and UI render workflow with exact commands and outputs."
  checked: false
---
# feat: Atlas Native Mother Spec Lens

> Narrow Atlas to its intended mission: a Mother-native spec visibility lens, with Svelte UI on top and no mixed MCT inventory scope.

## Problem

Atlas grew beyond spec visibility into mixed inventory output (children/toys/specs), which blurred mission and caused implementation confusion.
Current atlas internals also remain CLI-centric in places, instead of being cleanly rooted in Mother-native authority.

## Goal

1. Atlas is Mother-native.
2. Atlas is spec-only.
3. Atlas UI is Svelte/SvelteKit consuming Mother API.
4. CLI is a thin client wrapper, not an alternate snapshot engine.

## Scope corrections

This spec supersedes Atlas scope assumptions from prior slices that mixed spec + child + toy inventory into one lens.
Those artifacts remain historical context, but this spec is the current implementation authority for Atlas behavior.

## Non-goals

- Solving adapter-child migration in this spec.
- Defining a full product analytics platform.
- Redesigning Mother lifecycle primitives.

## Verification

```bash
cargo check -q
cargo test -q atlas
cargo test -q -p mother atlas

# with Mother running
curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/api/atlas/specs | jq '.summary'
patina atlas --json | jq '.summary'
```
