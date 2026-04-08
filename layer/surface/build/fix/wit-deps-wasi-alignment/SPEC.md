---
type: fix
id: wit-deps-wasi-alignment
status: draft
created: 2026-04-08
sessions:
  origin: 20260408-064526-677971000
beliefs:
  - "[[wasi-is-foundation-not-option]]"
related:
  - wit/child/deps/
  - wit/toys/wasi-p2/
  - sdk/patina-sdk/wit/child/deps/
  - resources/scripts/check-wit-consistency.sh
exit_criteria:
  - id: wda1-upstream-packages
    text: "WASI interfaces in `wit/child/deps/` reference proper upstream WASI package structure (multi-file directories per package, not flattened single files). Packages match the versions imported in `wit/child/child.wit`."
    checked: false
  - id: wda2-single-source
    text: "WASI interface files exist in ONE canonical location. `wit/toys/wasi-p2/` is the upstream source. `wit/child/deps/` and `sdk/patina-sdk/wit/child/deps/` reference or symlink from there — no independent copies to maintain."
    checked: false
  - id: wda3-patina-extensions-separate
    text: "Patina-specific toy interfaces (`patina:events-stream`, `patina:task`, `patina:measure`, `patina:git`, `patina:peer`, `patina:connect`) are clearly separated from WASI interfaces. Each documents which WASI gap it fills."
    checked: false
  - id: wda4-bindgen-works
    text: "`wasmtime::component::bindgen!` and `wit-bindgen` resolve all imports correctly from the new structure. `cargo check --workspace -q` and `cargo build --target wasm32-wasip2` for all children succeed."
    checked: false
  - id: wda5-pre-push-green
    text: "Pre-push WIT consistency check (`resources/scripts/check-wit-consistency.sh`) passes with the new structure."
    checked: false
---
# fix: Align WIT deps with WASI package conventions

> Use proper WASI package structure for WIT dependencies. Stop
> maintaining flattened copies of upstream interfaces.

## Scope Narrowing (session 20260408-120617)

Original spec scope covered all WIT deps (WASI + Patina extensions).
Narrowed to: **WASI Preview 2 multi-file packages only**. Patina
extension toys stay as flat single files — they're ours, not upstream,
and don't benefit from multi-file structure.

WASI packages that are actually multi-file (condensed from upstream):
- `wasi:http` — types.wit + handler.wit
- `wasi:io` — error.wit + poll.wit + streams.wit
- `wasi:clocks` — monotonic-clock.wit + wall-clock.wit
- `wasi:filesystem` — types.wit + preopens.wit

Naturally single-file: logging, keyvalue, messaging, sql — stay as-is.

### Spike: `wasi:io` multi-file resolution test

Before restructuring all packages, validate that both
`wasmtime::component::bindgen!` and `wit-bindgen::generate!` resolve
multi-file WASI packages from `deps/io/` subdirectories.

Test: replace `wit/child/deps/io.wit` with `wit/child/deps/io/{error,poll,streams}.wit`
using upstream files from `wit/toys/wasi-p2/io/`. Mirror to SDK. Run
`cargo check --workspace -q`.

If this passes, the approach works for all 4 multi-file packages.
If this fails, we learn what the resolver actually needs.

## Why

Patina aspires to align with the Bytecode Alliance component model
standards. Our toys import WASI interfaces — `wasi:logging@0.1.0`,
`wasi:keyvalue@0.2.0`, `wasi:http/outgoing-handler@0.2.8`, etc. But
the way we manage these dependencies doesn't follow BA conventions.

### What we have now

Three copies of WASI interfaces, in different formats:

1. **`wit/toys/wasi-p2/`** — proper upstream structure with directories
   per package (`http/handler.wit`, `http/types.wit`, etc.)
2. **`wit/child/deps/`** — flattened single-file copies (`http.wit`,
   `logging.wit`, etc.) used by host-side `bindgen!`
3. **`sdk/patina-sdk/wit/child/deps/`** — another set of flattened
   copies used by guest-side `wit-bindgen`

When WASI updates, we'd need to update all three. The flattened files
in `deps/` are hand-maintained condensations of the upstream packages.
This is fragile and diverges from how the BA ecosystem manages WIT deps.

### What BA conventions look like

The Bytecode Alliance is building proper package management (warg
registry, wit-deps tool). The convention is:

- Upstream WASI packages live in standard directory structure
- Projects reference them via deps directories with the proper package
  layout (not flattened single files)
- `wit-deps` or similar tooling pulls and pins versions

### What this means for Patina

We should have one canonical copy of each WASI package in proper
structure, and `deps/` directories should reference it — not maintain
independent flattened copies. Patina-specific extensions should be
clearly separated and documented.

## Problem

- Three copies of WASI interfaces to maintain
- Flattened format diverges from upstream package structure
- Patina extensions (`patina:events-stream`, etc.) mixed in with WASI
  deps in the same directory
- When we add new WASI imports or update versions, we must update all
  three locations manually

## Fix

1. Consolidate to one canonical WASI source (`wit/toys/wasi-p2/` or
   restructured equivalent)
2. `wit/child/deps/` references canonical source (symlinks, copies
   managed by script, or `wit-deps` tool)
3. `sdk/patina-sdk/wit/child/deps/` mirrors canonical source via the
   same mechanism
4. Patina-specific extensions live in a separate directory within deps
   (e.g., `deps/patina/`) clearly distinguished from WASI
5. Pre-push consistency check updated for new structure

## Investigation Needed

Before implementation, investigate:

1. **wit-deps tooling** — does `wit-deps` (BA tool) work for our use
   case? Can it manage deps for both host and guest bindgen?
2. **wasmtime bindgen resolution** — does `bindgen!` resolve multi-file
   WASI packages in deps, or does it need flattened files?
3. **wit-bindgen resolution** — same question for the guest-side SDK
4. **Symlinks vs copies** — will the Rust build system and cargo
   component tooling follow symlinks in wit/ directories?

## Non-Goals

- No WASI version upgrades in this spec (versions stay the same)
- No new toy interfaces
- No changes to child.wit world definitions
- No warg registry integration (that's future work)

## Verification

```bash
cargo check --workspace -q
cargo test -q --lib
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
cargo build -p patina-ai-child-dedup-filter --target wasm32-wasip2
bash resources/scripts/check-wit-consistency.sh
```
