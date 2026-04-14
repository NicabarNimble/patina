---
type: feat
id: folder-watch-actor-child
status: active
created: 2026-04-13
sessions:
  origin: 20260413-075041-892082000
related:
  - children/file-system-monitor/
  - children/folder-watch-actor/wit-contract/watch.wit
  - sdk/patina-sdk/
  - wit/child/
beliefs:
  - '[[children-have-agency-toys-are-capabilities]]'
  - '[[wasi-is-foundation-not-option]]'
  - '[[core-verbs-standalone-mother-additive]]'
exit_criteria:
  - id: fwac1-self-contained-child
    text: "New `folder-watch-actor` child is implemented as a self-contained reusable child actor (no Mother code changes in iteration 1)."
    checked: true
  - id: fwac2-polling-watch-loop
    text: "Child continuously watches by polling on `tick()` and emits only deltas (created/modified/deleted) after baseline snapshot."
    checked: true
  - id: fwac3-stateful-cursors
    text: "Child persists config, snapshot, and counters using keyvalue state so restarts remain idempotent."
    checked: true
  - id: fwac4-emits-events
    text: "Child emits structured file-change events via messaging toy; if no downstream consumers exist, emission remains harmless."
    checked: true
  - id: fwac5-runtime-controls
    text: "Child supports `configure`, `status`, `scan-now`, and `reset` via typed WIT control operations; handle business ingress is disabled."
    checked: true
  - id: fwac6-build-proof
    text: "Child compiles to wasm32-wasip2 and can run with `patina child run` after local install."
    checked: true
  - id: fwac7-non-legacy-sdk-lane
    text: "Implementation uses non-legacy `sdk/patina-sdk` lane (no `patina-sdk-legacy` dependency)."
    checked: true
  - id: fwac8-business-contract-wit
    text: "Business contract types for watcher behavior are defined in WIT (`patina:watch@0.1.0`) and used in child implementation (`configure/status/scan/reset` + file-change payload typing)."
    checked: true
  - id: fwac9-wasi-first-delta-only
    text: "WASI interfaces are used for capabilities (filesystem/keyvalue/logging/messaging); custom WIT is used only for business-domain contracts not covered by WASI."
    checked: true
---
# feat: Folder watch actor child

> Build a reusable folder-watch child actor that polls a folder on each tick and emits structured file change events, without changing Mother in iteration 1.

## Problem

Current `file-system-monitor` in the typed pando path performs a one-shot scan.
For reusable actor-style children we need a long-lived watcher primitive that:

- keeps its own baseline snapshot,
- emits only changes,
- survives restart with state,
- can be reused in multiple compositions.

## Goal

Ship a first-iteration watcher child that proves the actor shape using current runtime:

1. self-contained child actor crate,
2. delta detection + emission,
3. persisted snapshot/config/stats,
4. no Mother modifications.

## Status

Active.

## Non-Goals

- OS-native push file watch integration in Mother.
- New toy or WIT world additions in this iteration.
- Replacing existing `file-system-monitor` behavior yet.
- Building downstream consumers in this spec.

## Target Shape

`folder-watch-actor` (child kind = `child`) with:

- runtime contract: current `patina:child` world (`on_load`, `health`, `handle`, `tick`)
- business contract: `patina:watch@0.1.0` WIT package in child-local `wit-contract/watch.wit`

- `on_load` initializes baseline config/state,
- `tick` performs scan + diff + emit,
- business control is typed-only through `patina:watch/control` operations:
  - `configure`
  - `status`
  - `scan-now`
  - `reset`
- `handle` remains present only for runtime compatibility and denies business ingress.
- `health` reflects last run summary.

Declared toys:
- logging
- keyvalue
- messaging
- measure
- filesystem

## Solution

Implement `children/folder-watch-actor` in the non-legacy SDK lane (`patina-sdk`)
with direct `wit-bindgen` export of current `wit/child` world.

Use WASI interfaces for capability side:
- filesystem (scan)
- keyvalue (state)
- logging (runtime logs)
- messaging (event emission)

Add child-local custom WIT (`patina:watch@0.1.0`) for business contract side:
- watcher config
- watcher stats
- scan outcome
- file-change event type

Use custom WIT only for domain contract typing where WASI has no equivalent
watch-domain schema.

## Implementation Order

1. Scaffold new child crate + manifest.
2. Implement config/snapshot/stats state model.
3. Implement recursive scan + fingerprinting + delta diff.
4. Implement event emission + measure/log hooks.
5. Implement runtime actions and health.
6. Build wasm and run local proof commands.

## Resolved Decisions

- Iteration 1 will prefer child-only implementation over Mother changes.
- Watch semantics are polling-based (`tick`) to fit current runtime.
- Event envelope remains JSON in messaging lane for compatibility.
- WASI-first rule: capabilities via WASI imports; business-domain contracts via custom WIT only when WASI has no equivalent.

## Verification

```bash
cargo build --manifest-path children/folder-watch-actor/Cargo.toml --target wasm32-wasip2

# local install for CLI run proof
mkdir -p ~/.patina/plugins
cp children/folder-watch-actor/target/wasm32-wasip2/debug/patina_ai_child_folder_watch_actor.wasm ~/.patina/plugins/folder-watch-actor.wasm
cp children/folder-watch-actor/child.toml ~/.patina/plugins/folder-watch-actor.toml

patina child call folder-watch-actor patina:watch/control.status '[]'
patina child call folder-watch-actor patina:watch/control.configure '[{"watch-path":"/input","stream-name":"watch.folder","recursive":true,"include-hidden":false,"emit-existing-on-start":false,"extensions":[]}, false]'
patina child call folder-watch-actor patina:watch/control.scan-now '[]'
```

## Exit Criteria

Frontmatter criteria `fwac1..fwac9` are source of truth.

## Build Readiness

High for iteration 1: all required runtime surfaces already exist in current child world.
