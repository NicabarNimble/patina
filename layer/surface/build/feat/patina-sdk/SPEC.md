---
type: feat
id: patina-sdk
status: ready
created: 2026-02-14
sessions:
  origin: 20260214-130235
related:
- layer/surface/build/feat/plugin-template-gallery/SPEC.md
- layer/surface/build/fix/plugin-template-polish/SPEC.md
- layer/surface/build/feat/plugin-distribution/SPEC.md
beliefs:
- two-layer-capability-grants
- separate-worlds-for-isolation
- compiler-enforced-safety
---

# feat: Patina SDK — Consolidated Plugin Crate on crates.io

> Consolidate the four guest API crates into a single `patina-sdk`
> crate with feature-gated worlds, publish to crates.io, and eliminate
> absolute path dependencies from scaffolded plugins forever.

## Problem

When `patina plugin init` scaffolds a plugin project, the generated
`Cargo.toml` contains an absolute path dependency:

```toml
patina-task-api = { path = "/Users/nicabar/.../patina/patina-task-api" }
```

This is baked from `env!("CARGO_MANIFEST_DIR")` at compile time. It
works on the builder's machine and breaks everywhere else. There's no
portable way for a plugin author to depend on the guest API crates
because they aren't published anywhere.

Meanwhile, every mature WASM plugin system (Spin, Extism, wasmCloud)
publishes a single SDK crate to crates.io. Plugin authors write one
dependency line and it works on any machine:

```toml
spin-sdk = { version = "5.2", features = ["redis"] }
```

Patina has four separate guest API crates that serve the same role —
they should be one published SDK with feature flags selecting the
world.

## Current State

**Four guest API crates** (workspace-internal, never published):

| Crate | World | Size | Deps |
|-------|-------|------|------|
| patina-plugin-api | mother-child | 12K | wit-bindgen |
| patina-command-api | command | 16K | wit-bindgen |
| patina-task-api | task | 20K | wit-bindgen |
| patina-pipeline-api | pipeline | 16K | wit-bindgen, serde |

Each contains:
- `Cargo.toml` (cdylib deps)
- `src/lib.rs` (WIT bindings, trait, register macro, host re-exports)
- `wit/<world>/` (world WIT + `deps/patina-host/host.wit`)

Key properties:
- **Zero coupling** to patina internals — no deps on each other
  or any workspace crate (only wit-bindgen + serde)
- **Self-contained** — each includes all WIT files needed to build
- **host.wit duplicated 4x** — identical 117-line file in each
- **WIT files exist in two places** — repo-root `wit/` for host-side
  bindgen (wasmtime), per-crate `wit/` for guest-side (wit-bindgen).
  Same content, separate copies. This is standard WIT practice.

**Three internal consumers** (bundled plugins):
- `patina-doctor` → patina-command-api
- `patina-plugin-models` → patina-plugin-api
- `patina-plugin-repos` → patina-plugin-api

## Design

### Principle: One SDK, Feature-Gated Worlds

Following Spin's pattern: one crate, feature flags select the world.
Each world gets its own module with WIT bindings, trait, and macro.
Plugin authors enable exactly one feature.

### patina-sdk Crate Structure

```
patina-sdk/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              # Feature-gated public API
│   ├── wasm_cell.rs        # Shared WasmCell<T> (single-threaded WASM global)
│   ├── task.rs             # TaskPlugin trait, register_task!, re-exports
│   ├── command.rs          # CommandPlugin trait, register_command!, re-exports
│   ├── mother_child.rs     # MotherChildPlugin trait, register_plugin!, re-exports
│   └── pipeline.rs         # PipelinePlugin trait, register_pipeline!, re-exports
└── wit/
    ├── task/
    │   ├── task.wit
    │   └── deps/patina-host/host.wit
    ├── command/
    │   ├── command.wit
    │   └── deps/patina-host/host.wit
    ├── mother-child/
    │   ├── mother-child.wit
    │   └── deps/patina-host/host.wit
    └── pipeline/
        ├── pipeline.wit
        └── deps/patina-host/host.wit
```

### Cargo.toml

```toml
[package]
name = "patina-sdk"
version = "0.21.0"
edition = "2021"
license = "MIT"
description = "SDK for building Patina WASM plugins — task, command, pipeline, and mother-child worlds"
repository = "https://github.com/NicabarNimble/patina"
keywords = ["patina", "wasm", "plugin", "component-model"]
categories = ["development-tools", "wasm"]

# Ensure WIT files and license ship with the published crate
include = ["src/**", "wit/**", "Cargo.toml", "README.md", "LICENSE*"]

[features]
default = []
task = []
command = []
mother-child = []
pipeline = ["dep:serde", "dep:serde_json"]

[dependencies]
wit-bindgen = "0.41"
serde = { version = "1.0", features = ["derive"], optional = true }
serde_json = { version = "1.0", optional = true }
```

### lib.rs — Feature-Gated Public API

```rust
//! Patina SDK — build WASM plugins for the Patina ecosystem.
//!
//! Enable one feature to select your plugin world:
//!
//! ```toml
//! # Task plugin (actions + toys, full host access)
//! patina-sdk = { version = "0.21", features = ["task"] }
//!
//! # Command plugin (CLI subcommands, read-only)
//! patina-sdk = { version = "0.21", features = ["command"] }
//!
//! # Pipeline plugin (pure compute, log only)
//! patina-sdk = { version = "0.21", features = ["pipeline"] }
//!
//! # Mother-child plugin (daemon-resident, full access)
//! patina-sdk = { version = "0.21", features = ["mother-child"] }
//! ```

mod wasm_cell;

#[cfg(feature = "task")]
pub mod task;
#[cfg(feature = "task")]
pub use task::{TaskPlugin, Toy, register_task};

#[cfg(feature = "command")]
pub mod command;
#[cfg(feature = "command")]
pub use command::{CommandPlugin, register_command};

#[cfg(feature = "mother-child")]
pub mod mother_child;
#[cfg(feature = "mother-child")]
pub use mother_child::{MotherChildPlugin, register_plugin};

#[cfg(feature = "pipeline")]
pub mod pipeline;
#[cfg(feature = "pipeline")]
pub use pipeline::{PipelinePlugin, register_pipeline};
```

### Codegen: Macro, Not Pre-generated

Two options for how patina-sdk produces WIT bindings:

1. **Macro codegen** (chosen) — `wit_bindgen::generate!()` runs at
   compile time inside patina-sdk. WIT files ship in the crate, paths
   are crate-relative, works after publish. Plugin authors get
   `wit-bindgen` as a transitive build-time dep. This is what all 4
   existing crates do today and what Spin does.

2. **Pre-generated** — Run wit-bindgen as a dev tool, commit generated
   Rust, ship without the macro. Plugin authors don't need wit-bindgen.
   But we maintain generated code and must regenerate on WIT changes.

Macro codegen wins: less maintenance, same approach as today, standard
in the WASM component ecosystem. wit-bindgen is a proc-macro that only
runs at compile time — it doesn't affect the plugin's runtime binary.

### Shared WasmCell

The `WasmCell<T>` pattern (single-threaded WASM global) is currently
duplicated in all 4 guest API crates. Extract to `wasm_cell.rs` and
use from each world module. One copy, one `compile_error!` guard.

### API Version Embedding

Each world module embeds `API_VERSION` in `.patina_api_version` link
section (already exists in each crate). Stays per-module — different
worlds could theoretically version independently, though in practice
they move together.

### Scaffold Template Changes

Before (today):
```toml
# Generated Cargo.toml
[dependencies]
patina-task-api = { path = "/Users/nicabar/.../patina-task-api" }
```

After:
```toml
# Generated Cargo.toml
[dependencies]
patina-sdk = { version = "0.21", features = ["task"] }
```

Changes to scaffold:
- Remove `__GUEST_API_PATH__` placeholder from all templates
- Remove `paths::plugin::guest_api_crate()` function from `src/paths.rs`
- Remove `guest_api_crate_name()` from `src/plugin/scaffold.rs`
- Remove `guest_api_path` parameter from `substitute()` function
- Add `__SDK_VERSION__` placeholder (populated from `env!("CARGO_PKG_VERSION")`)
- Template `Cargo.toml.tmpl` uses single dependency line per world

### Migrate Internal Consumers

Three bundled plugins update their dependency:

```toml
# patina-doctor/Cargo.toml
# Before:
patina-command-api = { path = "../patina-command-api" }
# After:
patina-sdk = { path = "../patina-sdk", features = ["command"] }

# patina-plugin-models/Cargo.toml and patina-plugin-repos/Cargo.toml
# Before:
patina-plugin-api = { path = "../patina-plugin-api" }
# After:
patina-sdk = { path = "../patina-sdk", features = ["mother-child"] }
```

Internal consumers use path deps to the workspace-local `patina-sdk`.
External consumers (scaffolded plugins) use version deps to crates.io.

### Old Guest API Crates

**Physical merge, not re-export.** The code moves INTO patina-sdk.
patina-sdk has zero dependency on the old crates — this is critical
because `publish = false` crates cannot be dependencies of published
crates (crates.io users can't resolve them).

After the merge, mark old crates with `publish = false` in their
Cargo.toml. Keep in workspace temporarily as reference, remove in a
follow-up cleanup once migration is verified.

### Version Strategy

- `patina-sdk` version tracks `patina-ai` major.minor (currently 0.21)
- Patch versions can diverge (SDK can patch independently)
- Plugin manifests already have `patina_min = "0.21.0"` — host checks
  compatibility at load time
- Scaffold emits SDK version matching the host that generated it
  (`env!("CARGO_PKG_VERSION")` → `__SDK_VERSION__`)

### Publishing

**Crates to publish (in order):**

1. `patina-sdk` — the plugin SDK (no deps on patina-ai)
2. `patina-ai` — the CLI binary (depends on everything else but not SDK)

`patina-metal` can be published separately when there's demand. Not
required for the plugin ecosystem.

**CI workflow** (GitHub Actions):

```yaml
# On tag v*:
# 1. cargo test --workspace
# 2. cargo publish -p patina-sdk --dry-run
# 3. cargo publish -p patina-sdk
# 4. Wait for crates.io index update
# 5. cargo publish -p patina-ai --dry-run
# 6. cargo publish -p patina-ai
```

Both crates already have `license`, `description`, `repository` fields.
Add per-crate `README.md` before first publish.

## Files to Change

```
# New crate
patina-sdk/Cargo.toml                    # New: SDK crate manifest
patina-sdk/README.md                     # New: crate docs for crates.io/docs.rs
patina-sdk/src/lib.rs                    # New: feature-gated public API
patina-sdk/src/wasm_cell.rs              # New: shared WasmCell<T>
patina-sdk/src/task.rs                   # From: patina-task-api/src/lib.rs
patina-sdk/src/command.rs                # From: patina-command-api/src/lib.rs
patina-sdk/src/mother_child.rs           # From: patina-plugin-api/src/lib.rs
patina-sdk/src/pipeline.rs               # From: patina-pipeline-api/src/lib.rs
patina-sdk/wit/                          # From: per-crate wit/ directories

# Workspace
Cargo.toml                               # Add patina-sdk to workspace members

# Internal consumers (update dependency)
patina-doctor/Cargo.toml                 # patina-command-api → patina-sdk
patina-doctor/src/lib.rs                 # Update use/import paths
patina-plugin-models/Cargo.toml          # patina-plugin-api → patina-sdk
patina-plugin-models/src/lib.rs          # Update use/import paths
patina-plugin-repos/Cargo.toml           # patina-plugin-api → patina-sdk
patina-plugin-repos/src/lib.rs           # Update use/import paths

# Scaffold
resources/templates/plugin/*/Cargo.toml.tmpl   # patina-*-api path → patina-sdk version
src/plugin/scaffold.rs                   # Remove guest_api_path, add SDK version
src/paths.rs                             # Remove plugin::guest_api_crate()

# Old crates (mark unpublishable)
patina-plugin-api/Cargo.toml             # Add publish = false
patina-command-api/Cargo.toml            # Add publish = false
patina-task-api/Cargo.toml               # Add publish = false
patina-pipeline-api/Cargo.toml           # Add publish = false

# CI
.github/workflows/publish.yml           # New: cargo publish workflow
```

## Build Order

1. **Create patina-sdk crate** — new workspace member, move code from
   4 guest API crates, feature-gate modules, extract WasmCell. Verify
   `cargo build --target wasm32-wasip2 -p patina-sdk --features task`
   (and each feature individually).

2. **Migrate internal consumers** — switch patina-doctor,
   patina-plugin-models, patina-plugin-repos to depend on patina-sdk.
   Update import paths. Verify `cargo build --workspace` and
   `cargo test --workspace`.

3. **Update scaffold** — new Cargo.toml templates with SDK version dep,
   remove guest API path machinery from scaffold.rs and paths.rs.
   Verify `patina plugin init test-thing --world task --build`.

4. **Mark old crates** — add `publish = false` to 4 old guest API crates.

5. **Pre-publish prep** — README.md per published crate, verify
   `cargo publish --dry-run` for patina-sdk and patina-ai.

6. **Publish** — `cargo publish -p patina-sdk`, then `cargo publish -p patina-ai`.

Target: 6 phases, ~8 commits.

## Exit Criteria

### Critical
- [ ] `patina-sdk` crate exists with feature-gated worlds (task,
      command, mother-child, pipeline)
- [ ] Each feature compiles to wasm32-wasip2 independently
- [ ] Scaffold emits `patina-sdk = { version = "X", features = ["<world>"] }`
      with no absolute paths
- [ ] All internal consumers (doctor, models, repos) build against
      patina-sdk
- [ ] `cargo test --workspace` passes

### Important
- [ ] `patina-sdk` published to crates.io
- [ ] `patina-ai` published to crates.io (`cargo install patina-ai` works)
- [ ] WasmCell<T> deduplicated (one copy in SDK)
- [ ] Old guest API crates marked `publish = false`

### Nice-to-have
- [ ] CI workflow for automated publish on tag
- [ ] `patina-metal` evaluated for standalone publish
- [ ] docs.rs renders correctly for patina-sdk

### Pre-push
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace`
- [ ] `cargo test --workspace`

## Supersedes

This spec **supersedes** [[plugin-template-polish]] item 1 (Cargo.toml
path dep comments). The comment was a band-aid for the absolute path
problem — this spec eliminates the absolute path entirely.

[[plugin-template-polish]] items 2 and 3 (capability expansion and
build ergonomics) remain valid and can be built independently.

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-14 | draft | Designed from session discussion. Consolidates 4 guest API crates into 1 SDK, publishes to crates.io. Eliminates absolute path deps from scaffold. |
