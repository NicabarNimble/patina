---
type: fix
id: plugin-template-polish
status: ready
created: 2026-02-14
sessions:
  origin: 20260214-113605
related:
- layer/surface/build/feat/plugin-template-gallery/SPEC.md
beliefs:
- two-layer-capability-grants
- separate-worlds-for-isolation
---

# feat: Plugin Template Polish

> Three focused improvements to `patina plugin init` scaffolding before
> Tier 1 extractions (yolo, upgrade). Addresses review feedback on the
> initial [[plugin-template-gallery]] build.

## Problem

The PTG templates work (all 4 worlds scaffold and compile) but have
three rough edges:

1. **Absolute path dep** — `Cargo.toml` emits `path = "/Users/.../patina-task-api"`.
   Works on the builder's machine, breaks everywhere else. No comment
   explaining why or what to switch to.

2. **Minimal capabilities** — Every `plugin.toml` defaults to just
   `host_log = true`. A plugin author scaffolding a command plugin
   doesn't see `host_layer`, `host_query`, or the `[capabilities.toys]`
   section. They'll reverse-engineer it from existing plugins or docs.

3. **Debug-only --build** — Always runs `cargo build` (debug). No
   `--release` option. Rustup hint only appears on failure. Artifact
   path only printed on success.

## Design

### 1. Guest API Path Comment

Add a comment block above the dependency line in every Cargo.toml
template:

```toml
[dependencies]
# NOTE: This path dependency points to the patina source tree where
# the binary was compiled. When the guest API crates are published
# to crates.io, replace with:
#   patina-task-api = "0.1"
patina-task-api = { path = "__GUEST_API_PATH__" }
```

Four templates affected (one per world). No code change — template
files only.

### 2. Per-World Capability Scaffolding

Expand each world's `plugin.toml` template to show the full capability
surface for that world, with realistic defaults and commented-out
options:

**mother-child** (daemon children — full access):
```toml
[capabilities]
host_log = true
host_layer = true
# host_query = ["scry", "context", "assay"]
# host_http = ["api.github.com"]
```

**command** (read-only CLI — layer + query, no HTTP):
```toml
[capabilities]
host_log = true
host_layer = true
# host_query = ["scry", "context", "assay"]
```

**task** (action plugins — full access + toys):
```toml
[capabilities]
host_log = true
host_layer = true
# host_query = ["scry", "context", "assay"]
# host_http = ["api.github.com"]

# [capabilities.toys]
# commands = ["echo", "curl"]
```

**pipeline** (pure compute — log only, no expansion):
```toml
[capabilities]
host_log = true
```

Pipeline stays minimal — it only gets `host_log` per
[[separate-worlds-for-isolation]].

### 3. Build Ergonomics

Changes to the `--build` flag in `src/main.rs`:

- Add `--release` flag: `patina plugin init foo --world task --build --release`
- When `--release`, pass `--release` to cargo and look in
  `target/wasm32-wasip2/release/` for the artifact
- Always print artifact location (both success and where it would be)
- Proactive rustup check: before invoking cargo, test if the
  `wasm32-wasip2` target is installed (`rustup target list --installed`)
  and print the hint BEFORE attempting the build if missing

## Files to Change

```
# Templates (4 Cargo.toml + 3 plugin.toml — pipeline unchanged)
resources/templates/plugin/mother-child/Cargo.toml.tmpl
resources/templates/plugin/mother-child/plugin.toml.tmpl
resources/templates/plugin/command/Cargo.toml.tmpl
resources/templates/plugin/command/plugin.toml.tmpl
resources/templates/plugin/task/Cargo.toml.tmpl
resources/templates/plugin/task/plugin.toml.tmpl
resources/templates/plugin/pipeline/Cargo.toml.tmpl

# CLI dispatch
src/main.rs    # --release flag, proactive rustup check, artifact path
```

## Build Order

1. **Cargo.toml comments** — add path dep comment to all 4 templates
2. **Capability expansion** — update 3 plugin.toml templates (skip pipeline)
3. **Build ergonomics** — add --release flag, proactive rustup check

Target: 3 commits.

## Exit Criteria

### Critical
- [ ] Every generated Cargo.toml has a comment explaining the path dep
      and showing the future crates.io syntax
- [ ] command/task/mother-child plugin.toml templates show all
      capabilities their world supports (enabled or commented)
- [ ] `--release` flag produces release-mode WASM artifact

### Important
- [ ] Proactive rustup hint when wasm32-wasip2 target is missing
      (before build attempt, not after failure)
- [ ] Artifact path always printed (success message or "will be at" path)

### Pre-push
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace`
- [ ] `cargo test --workspace`

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | ready | Created from review feedback on PTG build. Three focused improvements: path dep comment, per-world capabilities, build ergonomics. |
