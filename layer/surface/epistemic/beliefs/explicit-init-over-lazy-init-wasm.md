---
type: belief
id: explicit-init-over-lazy-init-wasm
persona: architect
facets: [wasm, plugin-system, architecture]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-12
revised: 2026-02-12
---

# explicit-init-over-lazy-init-wasm

WASM plugins should use an explicit init export rather than lazy initialization or constructor tricks — the host controls call order, the macro just generates the export, and there is no cross-crate singleton complexity

## Statement

WASM plugins should use an explicit init export rather than lazy initialization or constructor tricks — the host controls call order, the macro just generates the export, and there is no cross-crate singleton complexity

## Evidence

- [[session-20260212-061458]]: [[commit-f3dda50d]] - register_plugin\! macro generates #[export_name = "init"] matching Zed's init-extension pattern, proven end-to-end (weight: 0.9)

## Supports

- [[sync-first]] — explicit init is synchronous and deterministic
- [[hoststate-cohabits-with-bindgen]] — init export lives in the API crate alongside bindgen

## Attacks

- Lazy initialization approaches (.init_array, static OnceLock, constructor crates) — these add complexity, are platform-dependent, and break in component model

## Attacked-By

- If plugins need expensive init that should be deferred, lazy init could be preferable

## Applied-In

- `wit/mother-child.wit` — `export init: func();` as first export in world
- `patina-plugin-api/src/lib.rs` — `register_plugin!` macro generates `#[export_name = "init"]`
- `src/plugin/internal.rs` — host calls `instance.call_init(&mut store)?` before `call_name()`
- Zed reference: `zed_extension_api` uses identical `init-extension` pattern with `skip: ["init-extension"]`

## Revision Log

- 2026-02-12: Created — metrics computed by `patina scrape`
