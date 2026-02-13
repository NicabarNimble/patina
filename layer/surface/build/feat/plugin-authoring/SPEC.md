---
type: feat
id: plugin-authoring
status: design
created: 2026-02-13
sessions:
  origin: 20260213-120746
blocked_by:
- plugin-pipeline-world
related:
- layer/surface/build/feat/plugin-ecosystem/SPEC.md
- layer/surface/build/feat/plugin-distribution/SPEC.md
beliefs:
- plugin-is-agent-plus-skill
- two-layer-capability-grants
---

# feat: Plugin Authoring (Approval UX + Templates)

> How plugins get created. Capability approval at install time.
> `patina plugin new` scaffolds a plugin project an LLM can write.

## Problem

No way to create new plugins or approve their capabilities. The
install-time UX doesn't exist. Plugin authors have no template.

## Parent Design

Build order items #5-6 from [[plugin-ecosystem]] SPEC.md. Capability
approval UX (lines 679-715) and plugin template (lines 636-677).

## Scope

### Capability Approval UX (item #5)

Install-time capability display and user approval:
```
$ patina plugin install ./pr-reviewer/
  Plugin: pr-reviewer v0.1.0
  World:  task
  Capabilities requested:
    host_query — search: beliefs, context
    host_http  — POST to: api.github.com
    toys       — commands: gh
  Approve? [Y/n]
```

Grants persisted in `~/.patina/plugin-grants.toml`. User not re-prompted
unless capabilities change on update.

### Plugin Template (item #6)

`patina plugin new <name> --world <world>` scaffolds a minimal plugin:
- Four templates, one per world
- Uses `patina-guest` umbrella crate
- `register_*!` macros hide bindgen boilerplate
- ~30 lines for a complete plugin

### Dependencies

- All four worlds must exist (templates cover all of them)
- This is the natural point to publish `patina-guest` to crates.io

## Exit Criteria

- [ ] `patina plugin install` shows capabilities and prompts for approval
- [ ] `~/.patina/plugin-grants.toml` persists approved grants
- [ ] `patina plugin new --world <world>` creates working scaffold for all 4 worlds
- [ ] `patina-guest` umbrella crate with feature flags per world
- [ ] Generated scaffold builds to valid WASM component
- [ ] Pre-push checks pass

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-13 | design | Extracted from [[plugin-ecosystem]] build order items #5-6. Blocked by pipeline world (all worlds needed for templates). |
