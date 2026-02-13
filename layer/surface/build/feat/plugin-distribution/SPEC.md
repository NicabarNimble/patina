---
type: feat
id: plugin-distribution
status: design
created: 2026-02-13
sessions:
  origin: 20260213-120746
  refined: 20260213-135136
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
sections lines 143-163).

## Spec Divergences from Parent

None yet. The install flow and skill registration design in the
ecosystem spec are aspirational. This spec will ground them in reality
once all worlds and authoring UX exist.

## Scope

### Item #7: Plugin Install

```bash
patina plugin install ./my-plugin/                              # local
patina plugin install github.com/user/patina-plugin-foo         # GitHub
patina plugin install ./plugin.wasm --manifest ./plugin.toml    # pre-built
```

Install flow: resolve source → build if needed → parse manifest →
validate WASM → show capabilities (from authoring spec) → user approval →
place files → register skills → update registry.

**Key decisions to resolve when ready:**
- Build strategy: should `patina plugin install ./src-dir/` invoke
  `cargo build --target wasm32-wasip2` directly? This requires the Rust
  toolchain + wasm target on the user's machine. Alternative: require
  pre-built `.wasm` for install, build is author-side only.
- GitHub install: `gh` CLI dependency vs direct API calls via `reqwest`.
  `reqwest` is already in deps (Cargo.toml line 47). Prefer reqwest to
  avoid external tool dependency.
- Registry format: `~/.patina/plugin-cache.toml` (ecosystem spec) or
  SQLite (queryable). TOML matches project conventions.
- File placement by world: `~/.patina/pipeline/`, `~/.patina/plugins/`,
  `~/.patina/children/` (per ecosystem spec line 621-626). Verify these
  paths exist in `src/paths.rs` when ready.
- WASM validation: check that the component's exports match the declared
  world. wasmtime provides component type introspection for this.

### Item #8: Skill Registration

A plugin bundle = WASM agent + skill prompt + manifest. Skill files
live in `skills/` directory inside the plugin project. Install copies
them to a discoverable location. Adapters can find and invoke them.

```
~/.patina/plugins/pr-reviewer/
├── pr-reviewer.wasm
├── plugin.toml
└── skills/
    └── review-pr.md    # skill prompt
```

Closes the loop: plugin provides intelligence (WASM) + instructions
(skill prompt) + declaration (manifest). The adapter orchestrates.

**Key decisions to resolve when ready:**
- Skill format: adapter-agnostic markdown or per-adapter variants?
  Ecosystem spec open question #1. Initial bet: agnostic, with structured
  sections adapters parse. Test with one real plugin first.
- Skill discovery: adapters need a path to scan for installed skills.
  Currently skills are in `resources/claude/skills/` (compile-time).
  Need a runtime discovery path like `~/.patina/skills/` or scan
  plugin directories.
- Skill invocation: how does the adapter know which plugin's skill to
  invoke? By name matching? By manifest metadata? Needs design once
  a real skill exists.

### What NOT to Touch

- Plugin runtime engines — install writes files, doesn't change engines
- WIT files — stable by this point
- Core scrape/search code — distribution is packaging, not protocol
- Adapter internals beyond skill discovery hooks
- `patina-guest` crate internals (created in authoring spec)

## Dependencies

- Plugin authoring (approval UX + templates) must exist
- All four worlds must be working and tested
- `patina-guest` on crates.io (for `cargo add` in template/install flow)
- At least one real plugin exists to validate the install + skill flow

## Key Files (likely, verify when ready)

| Area | Likely files |
|------|-------------|
| Install command | `src/commands/plugin.rs` (extend with `install` subcommand) |
| Source resolution | New module for GitHub/local/pre-built source handling |
| File placement | `src/paths.rs` (plugin install paths per world) |
| Skill registration | `src/adapters/` (skill discovery hooks), `src/paths.rs` (skill paths) |
| Registry | `src/paths.rs` + new registry module for `plugin-cache.toml` |

## Exit Criteria

- [ ] `patina plugin install` works for local directory source
- [ ] `patina plugin install` works for pre-built .wasm + manifest
- [ ] `patina plugin install` works for GitHub URL source
- [ ] Build from source: `cargo build --target wasm32-wasip2` when source detected
- [ ] Installed plugin appears in `patina plugin list`
- [ ] Skills copied to discoverable location during install
- [ ] At least one adapter can discover and list installed skills
- [ ] Plugin registry persists installed plugin metadata
- [ ] Uninstall: `patina plugin remove <name>` cleans up files + grants
- [ ] `cargo test --workspace` passes
- [ ] `./resources/git/pre-push-checks.sh` passes

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-13 | design | Extracted from [[plugin-ecosystem]] build order items #7-8. Blocked by authoring (needs approval UX). |
| 2026-02-13 | design | Refined in session [[20260213-135136]]. Added key decisions to resolve, likely files, "What NOT to Touch", clarified skill registration design questions. |
