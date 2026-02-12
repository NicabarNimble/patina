---
type: belief
id: wit-bindgen-generate-at-crate-root
persona: architect
facets: [wasm, wit-bindgen, plugin-system]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-12
revised: 2026-02-12
---

# wit-bindgen-generate-at-crate-root

wit_bindgen::generate\! with export\!() must live at crate root, not inside a mod wrapper — the export\! macro generates code referencing crate-root paths and nesting causes resolution failures

## Statement

wit_bindgen::generate\! with export\!() must live at crate root, not inside a mod wrapper — the export\! macro generates code referencing crate-root paths and nesting causes resolution failures

## Evidence

- [[session-20260212-061458]]: [[commit-f3dda50d]] - mod wit { generate\!() } caused 'could not find export in the crate root' error, fixed by moving generate\! to crate root (weight: 0.95)

## Supports

- [[hoststate-cohabits-with-bindgen]] — both beliefs constrain where bindgen code must live

## Attacks

- Contradicts Zed's `mod wit { generate!() }` pattern — Zed may use a different wit-bindgen version or config that avoids the issue

## Attacked-By

- Future wit-bindgen versions may fix crate-root path resolution, making mod nesting work

## Applied-In

- `patina-plugin-api/src/lib.rs` — `wit_bindgen::generate!()` at crate root with `export!(Component)` alongside
- Contrast: `src/plugin/internal.rs` (host side) uses `mod bindings { wasmtime::component::bindgen!() }` which works because wasmtime's bindgen has different path semantics

## Revision Log

- 2026-02-12: Created — metrics computed by `patina scrape`
