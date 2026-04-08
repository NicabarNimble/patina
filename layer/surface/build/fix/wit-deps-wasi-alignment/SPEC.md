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
  - wit/child/child.wit
  - wit/child/deps/
  - wit/toys/wasi-p2/
  - sdk/patina-sdk/wit/child/deps/
  - resources/git/pre-push-checks.sh
exit_criteria:
  - id: wda1-upstream-packages
    text: "WASI P2 multi-file packages (http, io, clocks, filesystem) use proper upstream directory structure in `wit/child/deps/` and `sdk/patina-sdk/wit/child/deps/` — not flattened single files."
    checked: false
  - id: wda2-http-package-fix
    text: "`wasi:http` upstream files in `wit/toys/wasi-p2/http/` have missing `package` declaration. Fixed before use as deps source."
    checked: false
  - id: wda3-bindgen-works
    text: "`wasmtime::component::bindgen!` (host) and `wit-bindgen::generate!` (guest SDK) resolve all imports correctly. `cargo check --workspace -q` and `cargo build --target wasm32-wasip2` for all children succeed."
    checked: false
  - id: wda4-pre-push-updated
    text: "Pre-push WIT consistency check (`resources/git/pre-push-checks.sh` steps 1-2) handles deps directories alongside flat files. Passes clean."
    checked: false
  - id: wda5-pipeline-mirror
    text: "`wit/pipeline/deps/` and `sdk/patina-sdk/wit/pipeline/deps/` updated if they carry any of the affected WASI packages."
    checked: false
---
# fix: WASI P2 multi-file package structure in deps

> Replace hand-flattened single-file WASI copies in `deps/` with proper
> upstream multi-file package directories. Patina extensions untouched.

## Deprioritized

Direction shifted during session 20260408-120617. The real BA alignment
work is typed WIT at the child level (`child-component-composition`),
not deps file structure. The io spike proved multi-file deps work, but
the remaining packages are secondary. deps/ is per-crate by design
(wasmtime does the same). Return to this after composition explore.

## Problem

WASI Preview 2 interfaces that are naturally multi-file packages
(http, io, clocks, filesystem) are maintained as hand-condensed single
files in `wit/child/deps/`. These condensations strip `@since`
annotations, merge interfaces, add unnecessary `world` blocks, and
diverge from the upstream source at `wit/toys/wasi-p2/`. The SDK
mirror at `sdk/patina-sdk/wit/child/deps/` duplicates the same flat
files.

When WASI versions change, someone must re-condense upstream packages
by hand — fragile and error-prone.

## What Changes

Replace 4 flattened files with upstream directory packages:

| Flat file | Replaced by | Source |
|---|---|---|
| `deps/io.wit` | `deps/io/{error,poll,streams}.wit` | `wasi-p2/io/` |
| `deps/http.wit` | `deps/http/{types,handler}.wit` | `wasi-p2/http/` (needs package decl fix) |
| `deps/clocks.wit` | `deps/clocks/{monotonic-clock,wall-clock}.wit` | `wasi-p2/clocks/` |
| `deps/filesystem.wit` | `deps/filesystem/{types,preopens}.wit` | `wasi-p2/filesystem/` |

Both `wit/child/deps/` and `sdk/patina-sdk/wit/child/deps/` get the
same directory structure. Pre-push check updated to handle directories.

## What Does NOT Change

- **Patina extensions** — `patina-connect.wit`, `patina-events-stream.wit`,
  `patina-git.wit`, `patina-measure.wit`, `patina-peer.wit`,
  `patina-task.wit` stay as flat files. They're ours, not upstream.
- **Naturally single-file WASI** — `logging.wit`, `keyvalue.wit`,
  `messaging.wit`, `sql.wit` stay as flat files.
- **child.wit world definitions** — no changes to imports or exports.
- **WASI versions** — same versions, just proper file structure.
- **No wit-deps tooling** — copies managed manually, not by registry.
- **No single-source consolidation** — SDK mirror remains a copy
  (required for crates.io). Reducing from "3 hand-maintained formats"
  to "upstream + 1 scripted mirror" is the win.

## Spike Result

`wasi:io` completed as proof (commit b7f06bcd):
- Replaced `deps/io.wit` with `deps/io/{error,poll,streams}.wit`
  from `wasi-p2/io/` upstream files
- Both `wasmtime::component::bindgen!` and `wit-bindgen::generate!`
  resolve correctly
- `cargo check --workspace -q` and `wasm32-wasip2` child build clean
- wit-parser (v0.227–0.244) handles mixed deps/ layouts: directories
  alongside flat files

## Known Issue: `wasi:http` package declaration

Upstream `wit/toys/wasi-p2/http/{types,handler}.wit` have **no
`package wasi:http@0.2.8;` declaration**. Other packages (io, clocks,
filesystem) all have `package` declarations. The http files need
`package wasi:http@0.2.8;` added to at least one file before wit-parser
can resolve them as a package from a directory.

## Build Plan

1. ~~`wasi:io` — spike (done, validated)~~
2. `wasi:clocks` — copy upstream, remove flat file, verify
3. `wasi:filesystem` — copy upstream, remove flat file, verify
4. `wasi:http` — fix package declaration in upstream, copy, remove flat file, verify
5. Update pre-push check for directory-aware deps matching
6. Mirror all changes to `sdk/patina-sdk/wit/child/deps/`
7. Check `wit/pipeline/deps/` for affected packages
8. Full verification: workspace check, wasm builds, pre-push, tests

## Verification

```bash
cargo check --workspace -q
cargo test -q --lib
cargo build -p patina-ai-child-schema-enforcer --target wasm32-wasip2
cargo build -p patina-ai-child-dedup-filter --target wasm32-wasip2
bash resources/git/pre-push-checks.sh --structural-only
```
