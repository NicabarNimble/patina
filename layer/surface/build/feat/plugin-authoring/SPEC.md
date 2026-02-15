---
type: feat
id: plugin-authoring
status: ready
created: 2026-02-13
sessions:
  origin: 20260213-120746
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

## Spec Divergences from Parent

None yet. This spec will refine when earlier build items are complete
and real implementation experience reveals constraints.

## Scope

### Item #5: Capability Approval UX

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

**Key decisions to resolve when ready:**
- Grant storage: `~/.patina/plugin-grants.toml` (flat file, simple) vs
  SQLite (queryable, but another db). Current lean: TOML (matches config
  patterns throughout patina).
- Re-approval trigger: detect capability changes by comparing manifest
  hash vs stored hash at load time.
- `check_capabilities()` in `src/plugin/internal/mod.rs` already validates
  at load time — extend it to read from grants file instead of hardcoded
  auto-grants.

### Item #6: Plugin Template

`patina plugin new <name> --world <world>` scaffolds a minimal plugin:
- Four templates, one per world (pipeline, command, task, mother-child)
- Uses `patina-sdk` crate with feature flags per world
- `register_*!` macros hide bindgen boilerplate
- ~30 lines for a complete plugin

**Resolved by [[patina-sdk]] and [[plugin-template-gallery]]:**
- SDK crate: `patina-sdk` — physical merge of 4 guest API crates into
  one crate with feature flags. Published to crates.io. Resolved by
  [[patina-sdk]] spec.
- Template storage: embedded in binary via `include_str!`. Built in
  [[plugin-template-gallery]].
- Scaffold compiles to valid WASM without edits — verified for all 4 worlds.

### What NOT to Touch

- Plugin runtime code (`src/plugin/internal/`) — approval changes grants
  loading, not engine code
- WIT files — worlds are stable by this point
- Existing compiled-in commands — template generates new plugins, doesn't
  modify existing ones
- Adapter templates (`resources/claude/skills/`) — skill registration
  is in distribution spec, not authoring

## Dependencies

- All four worlds must exist (templates cover all of them)
- `GrantedCapabilities` struct must be finalized (http_domains, query_kinds,
  query_scope, toy_commands all settled)
- This is the natural point to publish `patina-sdk` to crates.io

## Key Files (likely, verify when ready)

| Area | Likely files |
|------|-------------|
| Approval UX | `src/commands/plugin.rs` (new or extend), `src/plugin/internal/mod.rs` (grants loading) |
| Grants storage | `src/paths.rs` (grants file path), new grants parser module |
| Template scaffolding | `src/commands/plugin.rs` (new subcommand), `resources/templates/` (embedded templates) |
| SDK crate | `patina-sdk/` (consolidates 4 guest API crates), workspace Cargo.toml |

## Exit Criteria

- [ ] `patina plugin install` shows capabilities and prompts for approval
- [ ] `~/.patina/plugin-grants.toml` persists approved grants
- [ ] `check_capabilities()` reads from grants file, not hardcoded list
- [ ] `patina plugin new --world <world>` creates working scaffold for all 4 worlds
- [ ] `patina-sdk` crate with feature flags per world (see [[patina-sdk]] spec)
- [ ] Generated scaffold compiles to valid WASM component without edits
- [ ] LLM can generate a working plugin from the template + docs
- [ ] `cargo test --workspace` passes
- [ ] `./resources/git/pre-push-checks.sh` passes

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-13 | design | Extracted from [[plugin-ecosystem]] build order items #5-6. Blocked by pipeline world (all worlds needed for templates). |
| 2026-02-13 | design | Refined in session [[20260213-135136]]. Added key decisions to resolve, likely files, "What NOT to Touch", sharper exit criteria. |
| 2026-02-14 | design | SDK aligned: `patina-guest` → `patina-sdk`. Template and SDK decisions resolved by [[patina-sdk]] and [[plugin-template-gallery]]. |
