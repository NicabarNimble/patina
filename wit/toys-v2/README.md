## Phase 1 WIT lane

This directory is the add-only Phase 1 contract lane for toy collapse.

- `wasi:http/outgoing-handler` and `wasi:filesystem/types` are imported directly in `toybox.wit` to lock explicit WASI adoption targets.
- `patina:host-v2` owns the non-WASI interfaces (`connect`, `log`, `state`, `store`, `events`, `task`, `peer`, `git`).

Notes:

- This lane intentionally does not modify legacy `wit/toys/` or `wit/worlds/` contracts.
- Vendored WASI WIT deps live under `wit/toys-v2/deps/` so `wasm-tools component wit wit/toys-v2` validates the full package graph locally.
