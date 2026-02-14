---
type: feat
id: plugin-dev-watch
status: abandoned
created: 2026-02-14
blocked_by:
- plugin-template-gallery
sessions:
  origin: 20260214-084147
related:
- layer/surface/build/feat/plugin-template-gallery/SPEC.md
beliefs:
- plugin-is-agent-plus-skill
---

# feat: Plugin Dev Watch — Rebuild-on-Save Loop

> `patina plugin dev --watch` rebuilds WASM when source changes and runs
> a smoke test. Fast iteration loop for plugin authors.

## Problem

Plugin development requires manually running `cargo build --target
wasm32-wasip2` after each change, then copying the artifact and
restarting whatever is consuming it. This is the inner loop tax that
makes plugin authoring feel heavy compared to scripting.

## Scope

- `patina plugin dev` — one-shot build + optional smoke test
- `patina plugin dev --watch` — file watcher, rebuild on `.rs` or
  `plugin.toml` changes
- Smoke test: instantiate the built WASM via the correct engine
  (PluginEngine, CommandEngine, TaskEngine, PipelineEngine) and call
  a basic function (name, health, or handle with empty payload)
- Reports build time, artifact size, test result

## What NOT to Touch

- Scaffolding (`patina plugin init`) — that's [[plugin-template-gallery]]
- Template registry — that's [[plugin-template-registry]]
- Hot-reload of running daemon children (future work)

## Dependencies

- [[plugin-template-gallery]] must land first (establishes the project
  structure that dev-watch operates on)

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | design | Split from plugin-template-gallery. Dev loop is separate from scaffolding. |
