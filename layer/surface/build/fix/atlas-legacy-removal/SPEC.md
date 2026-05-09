---
type: fix
id: atlas-legacy-removal
status: active
created: 2026-05-09
sessions:
  origin: 20260508-144836-859149000
related:
- layer/allium/mother/mother-view-composer-target.allium
beliefs:
- '[[core-verbs-standalone-mother-additive]]'
- '[[spec-driven-design]]'
exit_criteria:
- id: alr1-cli-surface-removed-or-fail-closed
  text: The legacy `patina atlas` CLI surface is removed or replaced with an explicit archived/deprecated fail-closed message; no hardcoded Atlas snapshot generation remains as an active display path.
  checked: true
- id: alr2-mother-routes-removed
  text: Mother Atlas routes (`/atlas`, `/atlas/index.html`, `/atlas/atlas.json`, `/api/atlas/snapshot`) and runtime hooks are removed or return explicit archived/deprecated responses.
  checked: true
- id: alr3-atlas-assets-removed
  text: Legacy Atlas pando manifest and UI scaffold are removed or clearly archived outside active runtime discovery paths.
  checked: true
- id: alr4-docs-updated
  text: README and build/spec docs no longer present Atlas as an active UI/display plan; they point future display work at `mother-view-composer-target.allium`.
  checked: true
- id: alr5-tests-updated
  text: Atlas-specific tests are removed or changed to assert the archived/deprecated behavior, and deterministic test/check commands pass.
  checked: true
---
# fix: Remove legacy Atlas surfaces

> Archive the hardcoded Atlas prototype from active code paths now that Mother view composer buffers are the display authority.

## Problem

Atlas was an early hardcoded visibility prototype for specs, children, toys, JSON, HTML, local serving, Mother routes, pando identity, and a small Svelte scaffold. The Atlas spec lane has now been abandoned because future display work belongs to the Allium-defined Mother view composer.

Leaving Atlas active creates confusing parallel UI architecture and risks agents implementing new display work in the wrong place.

## Root Cause

Atlas mixed several concerns before the view-composer model existed:

- fixed snapshot generation
- fixed HTML rendering
- Mother HTTP routes
- pando identity
- Svelte scaffold
- spec, child, and toy inventory in one lens

The new target model instead uses Mother-owned Emacs-like buffers, Allium view shapes, explicit data requirements, observability gaps, and renderer frames such as SvelteKit.

## Fix

Remove Atlas from active runtime surfaces, or leave only an explicit archived/deprecated fail-closed shim where compatibility requires it.

Future work should reuse useful ideas only by extracting them into:

- Mother data catalog collectors
- local Allium view shapes
- buffer/frame APIs
- SvelteKit frame modes

## Non-goals

- Implementing the Mother view composer runtime in this cleanup.
- Preserving Atlas as a second display product.
- Adding new hardcoded dashboards.

## Exit Criteria

- [ ] `alr1-cli-surface-removed-or-fail-closed`
- [ ] `alr2-mother-routes-removed`
- [ ] `alr3-atlas-assets-removed`
- [ ] `alr4-docs-updated`
- [ ] `alr5-tests-updated`

## Verification

```bash
cargo check -q
cargo test -q -p patina-ai atlas
cargo test -q -p mother atlas
cargo test -q -p mother bridge::tests
rg -n "atlas|Atlas" README.md src mother ui resources layer/allium -g '!target' -g '!resources/models/**'
```
