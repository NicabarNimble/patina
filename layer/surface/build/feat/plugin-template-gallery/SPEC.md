---
type: feat
id: plugin-template-gallery
status: complete
created: 2026-02-14
sessions:
  origin: 20260214-084147
related:
- layer/surface/build/feat/plugin-authoring/SPEC.md
beliefs:
- plugin-is-agent-plus-skill
---

# feat: Plugin Scaffolding — `patina plugin init`

> One command to create a working plugin project for any world. Templates
> are embedded in the binary — no network, no registry, no cache sync.
> The existing test plugins (`tests/hello-task/`, `tests/echo-pipeline/`)
> prove the pattern: a Cargo.toml, a `src/lib.rs`, a `plugin.toml`, and
> a dependency on the right guest API crate. Automate what we already do
> by hand.

## Problem

Creating a Patina plugin requires:
1. Creating a Cargo project with `crate-type = ["cdylib"]`
2. Adding the correct guest API crate dependency (`patina-task-api`,
   `patina-command-api`, `patina-plugin-api`, or `patina-pipeline-api`)
3. Writing a `plugin.toml` manifest with the right world and capabilities
4. Implementing the correct trait (`Task`, `Command`, `MotherChild`, or
   `Pipeline`) via the `register_*!` macro
5. Building with `--target wasm32-wasip2`

This is 5 steps of boilerplate that an LLM or human must reconstruct from
docs each time. The test fixtures prove the shape — we just need to
parameterize and ship it.

## Design

### Principle: Embed What We Already Have

The test plugins in `tests/hello-task/` and `tests/echo-pipeline/` are
working minimal examples. Templates are these same structures,
parameterized by name, with the boilerplate filled in per world.

Templates are **embedded in the patina binary** via `include_str!`. No
cache directory, no sync, no signatures. The binary is the single source
of truth. This matches Patina's existing pattern: `resources/templates/`
for adapter configs are already embedded at build time.

### `patina plugin init <name> --world <world>`

```
patina plugin init review-bot --world task
```

Creates `./review-bot/` with:
```
review-bot/
├── Cargo.toml          # cdylib, patina-task-api dep
├── plugin.toml         # world = "task", capabilities, provides
└── src/
    └── lib.rs          # register_task! macro, stub impl
```

The project compiles to WASM immediately:
```
cd review-bot
cargo build --target wasm32-wasip2
```

### Four World Templates

Each template is minimal — just enough to compile and handle a request:

| World | Guest API crate | Entry macro | Stub behavior |
|-------|----------------|-------------|---------------|
| `mother-child` | `patina-plugin-api` | `register_plugin!` | Returns name + health |
| `command` | `patina-command-api` | `register_command!` | Prints args, exits 0 |
| `task` | `patina-task-api` | `register_task!` | Returns exit 0, no toys |
| `pipeline` | `patina-pipeline-api` | `register_pipeline!` | Echoes payload |

### Name Substitution

Template files use `__NAME__` and `__NAME_SNAKE__` placeholders:
- `__NAME__` → `review-bot` (kebab-case, used in plugin.toml, Cargo.toml name)
- `__NAME_SNAKE__` → `review_bot` (snake_case, used in Rust module names)

### Optional: `--build`

```
patina plugin init review-bot --world task --build
```

After scaffolding, runs `cargo build --target wasm32-wasip2` and reports
the artifact path. Fails gracefully if the WASM target isn't installed
(prints `rustup target add wasm32-wasip2` hint).

### What NOT to Touch

- Runtime capability enforcement (handled by host_support.rs)
- Plugin distribution/install (separate spec: [[plugin-distribution]])
- Capability approval UX (separate spec: [[plugin-authoring]])
- Template registries, sync, or signing (see child spec [[plugin-template-registry]])
- Dev watch mode (see child spec [[plugin-dev-watch]])

## Files to Change

```
# New — template source files (embedded at build time)
resources/templates/plugin/mother-child/Cargo.toml.tmpl
resources/templates/plugin/mother-child/plugin.toml.tmpl
resources/templates/plugin/mother-child/lib.rs.tmpl
resources/templates/plugin/command/Cargo.toml.tmpl
resources/templates/plugin/command/plugin.toml.tmpl
resources/templates/plugin/command/lib.rs.tmpl
resources/templates/plugin/task/Cargo.toml.tmpl
resources/templates/plugin/task/plugin.toml.tmpl
resources/templates/plugin/task/lib.rs.tmpl
resources/templates/plugin/pipeline/Cargo.toml.tmpl
resources/templates/plugin/pipeline/plugin.toml.tmpl
resources/templates/plugin/pipeline/lib.rs.tmpl

# New — scaffold logic
src/plugin/scaffold.rs              # Template loading, substitution, file writing

# Modified — CLI wiring
src/commands/plugin.rs              # Add `init` subcommand
src/main.rs                         # Wire plugin init dispatch
src/paths.rs                        # Add template resource paths (if needed)
```

## Build Order

1. **Create template files** for all 4 worlds under `resources/templates/plugin/`.
   Derive from existing test plugins. Verify each compiles standalone.
2. **Implement `src/plugin/scaffold.rs`** — load embedded templates, substitute
   names, write to disk. Pure functions, testable without filesystem.
3. **Wire CLI** — add `init` subcommand to `src/commands/plugin.rs`, dispatch
   from `src/main.rs`.
4. **Add `--build` flag** — optionally invoke `cargo build --target wasm32-wasip2`
   after scaffolding.
5. **Tests + pre-push** — unit tests for substitution, integration test that
   scaffolds and compiles a plugin.

Target: 4-5 commits.

## Exit Criteria

### Critical
- [ ] `patina plugin init <name> --world <world>` creates a compiling project for all 4 worlds
- [ ] Generated project builds with `cargo build --target wasm32-wasip2` without edits
- [ ] `plugin.toml` has correct world, default capabilities (`host_log`), and provides section

### Important
- [ ] `--build` flag compiles WASM and reports artifact path
- [ ] Missing WASM target prints `rustup target add wasm32-wasip2` hint
- [ ] Name validation: rejects invalid crate names, existing directories

### Pre-push
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace`
- [ ] `cargo test --workspace`
- [ ] Integration test: scaffold + compile for at least one world

## Child Specs

These are follow-on work that builds on the scaffolding foundation:

- **[[plugin-template-registry]]** (design) — External template sources, sync,
  caching, Ed25519 signing. Only needed when third-party templates exist.
- **[[plugin-dev-watch]]** (design) — `patina plugin dev --watch` for rebuild-on-save
  development loop. Separate concern from scaffolding.

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | design | Rewritten from outside agent draft. Removed fabricated belief (`local-first-by-default`), fake session ID, wrong file paths (`src/plugins/`, `src/crypto/`). Removed scope creep (template registry, signing, dev watch). Split into main + 2 child specs. Grounded in existing test plugin patterns. |
| 2026-02-14 | active | Built in 7 commits: templates for all 4 worlds, scaffold.rs with name validation + PascalCase conversion, CLI wiring with --build flag, guest API path resolution via paths.rs (CARGO_MANIFEST_DIR). All 4 worlds scaffold and compile to wasm32-wasip2. Pre-push checks pass. |
