---
type: feat
id: atlas-pando-mother-ui
status: draft
created: 2026-04-11
related:
- layer/core/values/spec-driven-design.md
- layer/core/values/dependable-rust.md
- layer/core/values/unix-philosophy.md
- layer/surface/build/feat/spec-atlas-mother-backplane/SPEC.md
- layer/surface/build/feat/legacy-typed-bridge-seam/SPEC.md
- resources/pandos/
- src/commands/mother/
beliefs:
- '[[children-have-agency-toys-are-capabilities]]'
- '[[core-verbs-standalone-mother-additive]]'
exit_criteria:
- id: apmu1-atlas-pando-seeded
  text: "A first-party `atlas` pando manifest is seeded into Mother registry surfaces."
  checked: false
- id: apmu2-atlas-namespace-policy
  text: "`atlas` namespace is owned by pando registration flow while CLI atlas remains additive fallback wrapper."
  checked: false
- id: apmu3-mother-atlas-web-routes
  text: "Mother serves atlas dashboard and snapshot routes (`/atlas`, `/atlas/index.html`, `/atlas/atlas.json`) from the same normalized model."
  checked: false
- id: apmu4-service-posture
  text: "Docs demonstrate always-on Mother posture via supervisor flow (`patina mother install`) instead of atlas-specific ad-hoc serve loops."
  checked: false
- id: apmu5-tests
  text: "Deterministic tests cover pando seeding + atlas web route behavior and fail-closed path handling."
  checked: false
- id: apmu6-hitl-demo
  text: "Demo includes Mother-started atlas dashboard/json retrieval and pando registry visibility checks."
  checked: false
---
# feat: Atlas Pando Mother UI

> Promote Atlas toward MCT shape: pando identity + Mother-hosted web lens + standalone CLI fallback.

## Problem

Atlas visibility exists in CLI and Mother API snapshot form, but not yet as a first-class pando identity with Mother-hosted UI routes and always-on daemon posture.

## Goal

- Seed a first-party `atlas` pando.
- Keep CLI `patina atlas` as additive fallback control surface.
- Serve Mother-hosted atlas dashboard routes from the normalized atlas model.
- Document always-on Mother posture via supervisor lifecycle.

## Non-goals (this slice)

- Full production SvelteKit asset pipeline and build system integration.
- Removing CLI atlas command.
- Forcing network-exposed unauthenticated control-plane access.

## Verification

```bash
cargo check -q
cargo test -q atlas
cargo test -q pando
```
