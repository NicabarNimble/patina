---
type: fix
id: sdk-public-surface-alignment
status: draft
created: 2026-04-10
sessions:
  origin: 20260409-143847-707078000
related:
- feat/patina-sdk-rebuild
- feat/cloudflare-worker-child
beliefs:
- '[[compiler-enforced-safety]]'
- '[[children-have-agency-toys-are-capabilities]]'
exit_criteria:
- id: spa1-toys-public
  text: "sdk/patina-sdk/src/lib.rs exposes toys as public API; types module is pub(crate) or otherwise non-public."
  checked: false
- id: spa2-no-dead-reexport
  text: "No public pub use types::* or public prelude re-export of patina:records types in patina-sdk."
  checked: false
- id: spa3-clean-compile
  text: "cargo check --workspace 2>&1 | grep 'warning.*sdk/patina-sdk' returns empty. No SDK-originated warnings."
  checked: false
- id: spa4-no-breakage
  text: "All 6 children build to wasm32-wasip2 (patina-ai-child-{file-system-monitor,content-extractor,schema-enforcer,dedup-filter,record-writer,lakehouse-catalog}) and cargo nextest run passes. No wit_bindgen with: mappings added."
  checked: false
- id: spa5-docs-aligned
  text: "sdk/patina-sdk/README.md no longer references prelude or type exports. Documents toy helpers as the public surface."
  checked: false
- id: spa6-stale-refs-cleaned
  text: "No spec or doc on disk references patina_sdk::prelude or patina_sdk::types as consumer-facing API. Known stale ref: cloudflare-worker-child/SPEC.md."
  checked: false
---
# fix: Align SDK Public Surface with Implementation Reality

## Problem

`patina-sdk-rebuild` (cs1) specified that the SDK re-exports `patina:records`
types as public API via a `prelude` module. In practice, no consumer imports
these types — children use their own WIT-local types from `wit_bindgen::generate!`
in `impl Guest` signatures. The re-exports produce an unused-import warning
on `sdk/patina-sdk/src/types.rs:7` that is currently unsuppressed.

## Root Cause

Rust treats types from separate `wit_bindgen::generate!` invocations as distinct
even when generated from the same WIT source. The SDK's re-exported
`RecordEnvelope` and a child's locally-generated `RecordEnvelope` are different
Rust types. Children cannot use SDK types in trait impl signatures without
`wit_bindgen`'s `with:` mapping, which adds coupling without current product need.

The SDK's actual delivered value is **toy helpers** — `toys::log`, `toys::keyvalue`,
`toys::measure`, `toys::config` — all of which operate on primitives (`&str`,
`f64`, `Vec<u8>`, `bool`). No domain types cross the SDK boundary.

## Decision

- `patina-sdk` public surface is toy helpers (`toys::log`, `toys::keyvalue`,
  `toys::measure`, `toys::config`).
- `types` module remains as internal SDK substrate — it holds the
  `wit_bindgen::generate!` bindings that toy helpers call. Visibility: `pub(crate)`.
- Do not publicly re-export `patina:records` types from `patina-sdk` for now.

## Rationale

- Children keep their own `wit_bindgen::generate!` and authoritative WIT-local
  types in Guest signatures.
- Public SDK type re-exports are not naturally consumable without deliberate
  `with:` coupling.
- Forcing `with:` now would add coupling without current product need.
- This keeps the API honest, small, and aligned with `dependable-rust` (small
  public interface) and `unix-philosophy` (one job: toy wrappers).

## Authority Check

This does not change Mother authority. Toy grants and enforcement still come
from `child.toml` `[needs].toys` and the host runtime linker. The SDK wraps
what Mother grants — it does not expand or restrict the grant surface.

## Semver

`patina-sdk` is v0.21.0 (pre-1.0) and consumed only within this workspace.
Removing public exports is a breaking change under strict semver, but 0.x
allows breaking changes by convention. No external consumers exist.

## Fix

1. `sdk/patina-sdk/src/lib.rs`: `pub mod types` → `pub(crate) mod types`.
   Remove `pub use types::*` and the `prelude` module.
2. `sdk/patina-sdk/src/types.rs`: remove `pub use patina::records::types::*`.
3. `sdk/patina-sdk/README.md`: remove prelude/type-export references. Document
   toy helpers as the public surface.
4. `layer/surface/build/feat/cloudflare-worker-child/SPEC.md`: update stale
   `use patina_sdk::prelude::*` example to use WIT-local types + `patina_sdk::toys`.
5. Verify: `cargo check --workspace` (grep for SDK warnings → empty), all 6
   named children build to wasm32-wasip2, `cargo nextest run` passes.

## Future Note

If cross-crate canonical record type identity is needed (e.g., shared business
logic functions across children), open a dedicated spec for `wit_bindgen` `with:`
type-sharing and evaluate coupling tradeoffs explicitly. Do not bolt it onto the
toy-helper SDK.
