---
type: refactor
id: engine-consolidate
status: draft
created: 2026-04-02
sessions:
  origin: 20260402-135104-312151000
blocked_by:
  - ducklake-retirement
  - child-rename
beliefs:
  - "[[children-are-wasm]]"
  - "[[children-have-agency-toys-are-capabilities]]"
  - "[[world-boundary-is-type-safety]]"
  - "[[wasi-is-foundation-not-option]]"
related:
  - src/child/internal/knowledge_child.rs
  - src/child/internal/child.rs
  - src/child/internal/pipeline.rs
  - src/child/internal/mod.rs
  - wit/child/
  - wit/pipeline/
  - sdk/patina-sdk/src/pipeline.rs
  - resources/git/pre-push-checks.sh
exit_criteria:

  - id: ec1-one-wit-world
    text: "wit/pipeline/ merged into wit/child/. Package patina:child@0.1.0 is the only child world. Unified world export signature is handle(action: string, payload: string). Legacy pipeline-style request envelopes are supported via SDK shims during migration."
    checked: false

  - id: ec2-handle-signature
    text: "Unified world uses handle(action: string, payload: string) signature from child world. SDK PipelineChild trait shim adapts handle(request) → handle(action, payload) for legacy grammar plugins during transition."
    checked: false

  - id: ec3-one-engine
    text: "PipelineEngine (src/child/internal/pipeline.rs) merged into ChildEngine. AOT cache logic carried over. Single engine loads and runs all children."
    checked: false

  - id: ec4-capability-gate
    text: "Merged engine links all WIT interfaces at linker build time but gates at call boundary via GrantedCapabilities. cargo test verifies a child with toys=[\"log\"] cannot invoke state/layer-fs/git host functions."
    checked: false

  - id: ec5-grammar-plugins-recompiled
    text: "Grammar plugins recompiled against unified world via SDK. patina setup grammars installs updated binaries. ~/.patina/pipeline/ storage path unchanged."
    checked: false

  - id: ec6-sdk-pipeline-deprecated
    text: "sdk/patina-sdk/src/pipeline.rs marked deprecated. PipelineChild trait and register_pipeline_child! macro kept as temporary shims. Module doc says: removed next minor."
    checked: false

  - id: ec7-ci-updated
    text: "resources/git/pre-push-checks.sh SDK_WORLDS updated to (child). WIT path references use wit/child/."
    checked: false

  - id: ec8-compile-proof
    text: "cargo check --workspace -q passes. cargo test -q --lib passes including capability gate test."
    checked: false

  - id: ec9-runtime-proof
    text: "patina child run doctor health works. patina scrape code works using grammar plugins via unified engine."
    checked: false
---

# refactor: Engine Consolidate

**Blocked by: `child-rename`**

Merge `PipelineEngine` and `ChildEngine` into a single engine. Merge
`wit/pipeline/` into `wit/child/`. Grammar plugins are children — they just
have a minimal toybox.

## Why

`PipelineEngine` is `ChildEngine` with everything stripped out. The split
existed because grammar parsers felt like a different species. They aren't.
The toybox determines behavior, not the engine path. A grammar parser with
`toys = ["log"]` has the same runtime as any other child — it just has fewer
grants.

## The handle() signature problem (AF1)

Pipeline exports `handle(request: string)`. Child exports
`handle(action: string, payload: string)`. These cannot coexist in one world.

**Resolution:** Unified world uses the child signature (richer contract).
SDK provides a temporary `PipelineChild` trait shim that adapts
`handle(request)` to `handle(action, payload)` by passing payload as request.
Grammar plugins migrate to the full `Child` trait by next minor release.
The shim is triage, not permanent support.

## Capability gating (AF2)

Today `PipelineEngine` links only WASI + host_log. The merged engine links all
interfaces (Wasmtime requires imports satisfied at link time) but gates at the
call boundary via `GrantedCapabilities` — the same pattern `ChildEngine`
already uses. A grammar plugin declaring `toys = ["log"]` gets log access
only. A required cargo test (`ec4`) proves this.

## What the merged engine looks like

`ChildEngine` absorbs `PipelineEngine`:
- AOT cache logic (`load_component_cached`) carries over unchanged
- Full linker setup from `ChildEngine` (all ~15 interfaces)
- Capability gate at call boundary (from `ChildEngine`)
- Lifecycle exports (`health`, `tick`, `drain`) — grammar plugins return
  stubs (healthy, [], [])

`src/child/internal/pipeline.rs` is deleted after merge.

## Grammar plugins

Recompile against unified world via SDK. `patina setup grammars` installs
updated binaries. Storage path `~/.patina/pipeline/` stays — it is a
filesystem location, not a kind label.

## Risks

- **AF1 handle signature** — grammar plugins must be recompiled. Old `.wasm`
  artifacts will fail to instantiate (linker error). Users need
  `patina setup grammars --force` after upgrade.
- **Linker complexity** — merging from 1 interface to ~15. Read both engine
  files fully before writing the merged version.
- **Capability widening** — required cargo test (`ec4`) is not optional.
  Must prove toys=[\"log\"] child is blocked from state/layer-fs/git.
