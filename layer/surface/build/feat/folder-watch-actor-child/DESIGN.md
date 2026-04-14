# Design: Folder watch actor child

## Why This Design

We need a reusable watcher primitive now without waiting for Mother runtime
changes. The current child world already supports durable state, messaging,
logging, measure, and filesystem preopens.

Polling on `tick()` gives actor behavior immediately:
- deterministic
- restart-safe
- composition-friendly

## Build Target

A new child crate:

- `children/folder-watch-actor/`
- kind: `child`
- runtime world: `wit/child` (direct `wit-bindgen` export)
- business contract WIT: `children/folder-watch-actor/wit-contract/watch.wit` (`patina:watch@0.1.0`)
- SDK lane: non-legacy `sdk/patina-sdk` for toy helpers

Runtime behavior:
- baseline snapshot persisted in state
- delta emits (`created|modified|deleted`) over messaging
- operator control via typed `patina:watch/control` operations (`wit-only` ingress)

## Resolved Decisions

1. No Mother code modifications in iteration 1.
2. Polling watch loop (`tick`) is sufficient first implementation.
3. Event payload stays JSON for compatibility with existing messaging lane.
4. Config and snapshot are child-owned state keys.
5. Do not use `patina-sdk-legacy`; stay on non-legacy SDK lane.
6. WASI is used for capabilities; custom WIT is added only for watcher-domain contract typing.

## Commits
1. `feat(folder-watch-actor): add self-contained watcher child actor with polling delta emission`

## Direct Code Targets
- `children/folder-watch-actor/Cargo.toml` — new child crate metadata and dependencies.
- `children/folder-watch-actor/child.toml` — child identity and toy grants.
- `children/folder-watch-actor/wit-contract/watch.wit` — watcher business contract WIT package.
- `children/folder-watch-actor/src/lib.rs` — watcher implementation and WIT contract mapping.

## Verification Plan

1. Compile to wasm32-wasip2.
2. Install wasm + manifest into `~/.patina/plugins`.
3. Run typed calls (`patina child call ...`) for `status`, `configure`, and `scan-now`.
4. Confirm `tick` path executes without crash and updates counters.

## Build Readiness

High for first iteration, bounded by no Mother-side changes.

## Open Questions

1. Should first-run behavior emit existing files by default or baseline silently?
2. Should deleted-file events include prior fingerprint metadata?
3. What final stream/topic naming convention should watcher-family adopt?
