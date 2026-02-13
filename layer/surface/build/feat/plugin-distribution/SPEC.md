---
type: feat
id: plugin-distribution
status: design
created: 2026-02-13
sessions:
  origin: 20260213-120746
blocked_by:
- plugin-authoring
related:
- layer/surface/build/feat/plugin-ecosystem/SPEC.md
- layer/surface/build/feat/plugin-authoring/SPEC.md
beliefs:
- plugin-is-agent-plus-skill
- skills-for-structured-output
---

# feat: Plugin Distribution (Install + Skills)

> How plugins get shared. Install from GitHub, register skills,
> close the agent + skill loop.

## Problem

No way to install plugins from external sources or register the skill
(prompt) half of the plugin bundle. Plugins are local-only.

## Parent Design

Build order items #7-8 from [[plugin-ecosystem]] SPEC.md. Install
command (lines 596-634) and skill registration (ecosystem spec skill
sections).

## Scope

### Plugin Install (item #7)

```bash
patina plugin install ./my-plugin/                              # local
patina plugin install github.com/user/patina-plugin-foo         # GitHub
patina plugin install ./plugin.wasm --manifest ./plugin.toml    # pre-built
```

Install flow: resolve source → build if needed → parse manifest →
validate WASM → show capabilities → user approval → place files →
register skills → update registry.

### Skill Registration (item #8)

A plugin bundle = WASM agent + skill prompt + manifest. Skill files
live in `skills/` directory inside the plugin project. Install copies
them to a discoverable location. Adapters can find and invoke them.

Closes the loop: plugin provides intelligence (WASM) + instructions
(skill prompt) + declaration (manifest). The adapter orchestrates.

### Dependencies

- Plugin authoring (approval UX + templates) must exist
- All worlds must be working
- `patina-guest` on crates.io (for `cargo add` in install flow)

## Exit Criteria

- [ ] `patina plugin install` works for local directory, GitHub URL, pre-built WASM
- [ ] Build from source: `cargo build --target wasm32-wasip2` for Rust sources
- [ ] Skills copied to discoverable location during install
- [ ] Adapter can discover and list installed skills
- [ ] Plugin registry (`~/.patina/plugin-cache.toml` or similar)
- [ ] Pre-push checks pass

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-13 | design | Extracted from [[plugin-ecosystem]] build order items #7-8. Blocked by authoring (needs approval UX). |
