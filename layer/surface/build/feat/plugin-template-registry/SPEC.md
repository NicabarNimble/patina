---
type: feat
id: plugin-template-registry
status: design
created: 2026-02-14
sessions:
  origin: 20260214-084147
blocked_by:
- plugin-template-gallery
related:
- layer/surface/build/feat/plugin-template-gallery/SPEC.md
- layer/surface/build/feat/plugin-distribution/SPEC.md
beliefs:
- plugin-is-agent-plus-skill
---

# feat: Plugin Template Registry — External Sources + Signing

> Extends `patina plugin init` with external template sources beyond
> the embedded defaults. Templates can be pulled from git repos,
> cached locally, and verified via Ed25519 signatures.

## Problem

The embedded templates from [[plugin-template-gallery]] cover the 4 stock
worlds. But community or organization-specific templates (e.g., a
company's standard webhook handler, a chatops bot skeleton) need a way
to be distributed, cached, and verified without rebuilding the binary.

## Scope

- `patina templates sync <git-url>` — clone/pull template packs into
  `~/.patina/templates/`
- `patina templates list` — show embedded + external templates
- `patina templates verify` — Ed25519 signature verification for external
  templates
- Local templates (user-created, no signature) marked as "local"
- Cache-first: once synced, all operations work offline

## What NOT to Touch

- Core scaffolding (`patina plugin init`) — that's [[plugin-template-gallery]]
- Plugin runtime or capability enforcement
- Plugin install/distribution

## Dependencies

- [[plugin-template-gallery]] must land first (establishes template format
  and `patina plugin init` CLI)

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | design | Split from plugin-template-gallery. Registry/signing is separate from core scaffolding. |
