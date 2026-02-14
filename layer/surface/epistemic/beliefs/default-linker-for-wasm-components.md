---
type: belief
id: default-linker-for-wasm-components
persona: architect
facets: [wasm, toolchain, plugins]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-14
revised: 2026-02-14
---

# default-linker-for-wasm-components

When targeting wasm32-wasip2, never override the linker — the default wasm-component-ld wraps core modules into WASM components that wasmtime can load, while wasm-ld produces incompatible core modules.

## Statement

When targeting wasm32-wasip2, never override the linker — the default wasm-component-ld wraps core modules into WASM components that wasmtime can load, while wasm-ld produces incompatible core modules.

## Evidence

- [[session-20260214-182709]]: grammar-rust plugin failed to load when built with `linker = wasm-ld` in `.cargo/config.toml` — produced version 0x1 core module instead of 0x1000d component. Removing the linker override let the default `wasm-component-ld` produce valid components. (weight: 0.95)
- [[commit-6981fdbe]]: Fix commit removing linker override from `grammar-rust/.cargo/config.toml`. Only CC/AR/CFLAGS env vars needed for wasi-sdk cross-compilation. (weight: 0.9)
- grammar-cairo (Phase 1) had no linker override and worked correctly — the difference proved the root cause. (weight: 0.8)

## Supports

- [[gate-exports-on-target-arch]] — WASM component format is the correct export target
- [[parser-agnostic-interfaces]] — plugins must produce valid components regardless of parser technology
- [[separate-worlds-for-isolation]] — components enforce the isolation boundary that core modules don't

## Attacks

## Attacked-By

## Applied-In

- `grammar-rust/.cargo/config.toml` — only sets CC/AR/CFLAGS, no linker override
- `grammar-cairo/.cargo/config.toml` — only sets `[build] target`, no linker at all

## Revision Log

- 2026-02-14: Created — metrics computed by `patina scrape`
