## Phase 1 WIT lane

This directory is the add-only Phase 1 contract lane for toy collapse.

- `wasi:http/outgoing-handler` and `wasi:filesystem/types` are imported directly in `toybox.wit` to lock explicit WASI adoption targets.
- Each Patina toy is modeled as its own package (`patina:connect`, `patina:store`, `patina:events`, `patina:task`, `patina:peer`, `patina:git`, `patina:log`, `patina:state`) to preserve proposal-ready identity.

Notes:

- This lane intentionally does not modify legacy `wit/toys/` or `wit/worlds/` contracts.
- Vendored WASI WIT deps live under `wit/toys/deps/` so `wasm-tools component wit wit/toys` validates the full package graph locally.
