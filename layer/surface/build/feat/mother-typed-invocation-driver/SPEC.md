---
type: feat
id: mother-typed-invocation-driver
status: active
created: 2026-04-13
sessions:
  origin: 20260413-230000-000000000
beliefs:
  - "[[patina-identity]]"
  - "[[spec-driven-design]]"
  - "[[dependable-rust]]"
  - "[[adapter-pattern]]"
  - "[[safety-boundaries]]"
  - "[[unix-philosophy]]"
related:
  - layer/core/values/patina-identity.md
  - layer/core/values/spec-driven-design.md
  - layer/core/values/dependable-rust.md
  - layer/core/values/adapter-pattern.md
  - layer/core/values/safety-boundaries.md
  - layer/core/values/unix-philosophy.md
  - src/child/internal/child.rs
  - mother/src/registry.rs
  - mother/src/http_api.rs
  - mother/src/http_routes.rs
  - src/commands/mother/daemon.rs
  - children/folder-watch-actor/src/lib.rs
exit_criteria:
  - id: mtid1-driver-seam
    text: "Typed invocation in WASM child runtime routes through an explicit InvocationDriver seam with at least two real implementations (fail-closed + typed-component, with handle-bridge compatibility lane)."
    checked: true
  - id: mtid2-generic-op-resolution
    text: "Operation ids are resolved generically (`<package>:<interface>.<function>`), with strict validation and typed error taxonomy."
    checked: true
  - id: mtid3-observability-surface
    text: "Mother records typed invocation outcomes (success/error/denied), deny reasons, policy/invoke timing, and exposes a query surface for recent calls."
    checked: true
  - id: mtid4-folder-watch-proof
    text: "folder-watch-actor typed calls (`configure`, `status`, `scan-now`, `reset`) work end-to-end with no watcher-specific Mother binding branches."
    checked: true
---
# feat: mother typed invocation driver

## Problem

`patina child call` existed, but WASM child runtime still fail-closed for generic typed business operations. That blocked `mother-wit-dispatcher` criteria (`mwd2`, `mwd5`).

## Goal

Add a contract-agnostic invocation lane that:
- resolves operation ids generically,
- enforces fail-closed argument/operation validation,
- preserves Mother domain neutrality,
- and adds Rivet-style operational visibility.

## Scope lock

- No watcher-specific binding branches in Mother runtime.
- No capability broadening.
- Keep compatibility lane (`handle`) during migration.

## Verification

```bash
cargo test -p patina-ai resolve_typed_operation_ -- --nocapture
cargo test -p patina-ai encode_typed_args_for_handle_ -- --nocapture
cargo test -p mother observed_typed_call_emits_success_metrics -- --nocapture
cargo test -p mother handle_mode_denies_typed_call -- --nocapture
cargo test -p mother inspector_typed_calls_route_returns_history -- --nocapture
cargo test -p patina-ai folder_watch_actor_typed_call_contracts_end_to_end -- --nocapture

# optional compatibility lane toggle
PATINA_TYPED_CALL_DRIVER=handle-bridge cargo test -p patina-ai folder_watch_actor_typed_call_contracts_end_to_end -- --nocapture
```
