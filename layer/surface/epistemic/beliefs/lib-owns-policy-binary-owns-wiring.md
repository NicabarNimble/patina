---
type: belief
id: lib-owns-policy-binary-owns-wiring
persona: architect
facets: [architecture, rust, plugin-system]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-13
revised: 2026-02-13
---

# lib-owns-policy-binary-owns-wiring

When plugin host functions need engines that live in the binary crate, the library crate defines the port (gating, validation, sanitization) and the binary supplies the adapter (closure capturing actual engines) — the lib never depends on the binary.

## Statement

When plugin host functions need engines that live in the binary crate, the library crate defines the port (gating, validation, sanitization) and the binary supplies the adapter (closure capturing actual engines) — the lib never depends on the binary.

## Evidence

- [[session-20260213-112528]] - QueryDispatchFn pattern emerged from crate boundary constraint: plugin system (lib.rs) cannot reference retrieval/commands (main.rs), solved with callback injection (weight: 0.95)
- [[commit-9ad5f098]] - Initial implementation: `QueryDispatchFn` type, `make_query_dispatch()` closure builder, `run_command()` accepts optional dispatch (weight: 0.9)
- [[commit-80da2ec7]] - Spec amended to document pattern as supported architecture, not a one-off divergence (weight: 0.8)

## Supports

- [[two-layer-capability-grants]] - callback pattern enables load-time + call-time gating split
- [[separate-worlds-for-isolation]] - clean port/adapter seam supports multiple frontends (CLI, daemon, TUI)

## Attacks

<!-- None yet -->

## Attacked-By

- Performance concern: callback indirection adds one dynamic dispatch per query call. Mitigated: query calls are already I/O-bound (SQLite, ONNX), dispatch overhead is negligible.

## Applied-In

- `src/plugin/internal/command.rs` — `QueryDispatchFn` type alias, Host impl delegates to callback after gating
- `src/main.rs` — `make_query_dispatch()` builds closure capturing `QueryEngine` + commands modules
- [[plugin-ecosystem]] SPEC.md — "Query dispatch lives behind a QueryDispatchFn boundary"

## Revision Log

- 2026-02-13: Created — metrics computed by `patina scrape`
