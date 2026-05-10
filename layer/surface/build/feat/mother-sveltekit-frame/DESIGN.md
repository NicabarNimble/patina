# Design: Mother SvelteKit Frame

## Why This Design

[[mother-view-composer]] needs a concrete frame to prove the Emacs-like split: Mother owns buffers/windows/shapes/payloads, while renderers attach to those buffers. A SvelteKit app is the first frame because it can provide a browser UI quickly while keeping Mother APIs as the source of truth.

The design intentionally avoids a bespoke dashboard. It renders generic Mother view-buffer envelopes, lists Mother records, and sends connect/disconnect/open actions back to Mother.

## Build Target

Add `frames/sveltekit`:

- SvelteKit + TypeScript + adapter-node.
- Same-origin proxy under `/api/mother/*` that forwards to Mother HTTP with optional bearer token.
- Browser UI that lists buffers, windows, shapes, request details, gaps, and renders selected buffer payloads.
- UI-local frame/window identity persisted only in `localStorage`; Mother receives these ids through connect/disconnect calls and remains authoritative for window records.

Add one Mother API:

- `GET /api/view-buffers/<buffer_id>/payload` returns an `OpenedBuffer`-style payload for an existing live/stale/blocked buffer. Replaced/killed buffers fail closed.

## Resolved Decisions

- SvelteKit is a frame, not a shape/buffer library.
- Existing-buffer payload reads should be served by Mother; reopening a shape to see payload would create a new buffer and blur frame vs owner responsibilities.
- The SvelteKit server proxy owns only transport concerns: base URL normalization, optional bearer auth, and error propagation.
- Unknown major modes render as escaped JSON from the Mother payload, never as generated TypeScript/Svelte code.
- No persistent browser state beyond frame/window ids and selected buffer id.

## Commits

1. Pending — add Mother payload endpoint and SvelteKit frame.

## Direct Code Targets

- `mother/src/view_buffer/service.rs` — add existing-buffer payload read.
- `mother/src/http_api.rs` — expose payload read through `ApiRuntime`/`ViewBufferApi` and route table.
- `mother/src/http_api/view_buffer.rs` — add handler.
- `mother/src/http_routes.rs` — route `GET /api/view-buffers/<id>/payload`.
- `src/commands/mother/daemon/dispatch.rs` — persist/load enough state to serve payloads from daemon dispatch.
- `frames/sveltekit/` — new SvelteKit frame app.
- `layer/surface/build/feat/mother-sveltekit-frame/SPEC.md` — criteria and status.
- `layer/surface/build/feat/mother-view-composer/SPEC.md` — mark renderer frame complete after release.

## Verification Plan

- Rust:
  - `cargo fmt`
  - `cargo check -q`
  - `cargo test -q -p mother view_buffer_payload`
  - `cargo test -q -p mother view_buffer`
  - `cargo test -q -p mother`
- SvelteKit:
  - `cd frames/sveltekit && npm run check`
  - `cd frames/sveltekit && npm run build`
- Specs:
  - `patina spec check mother-sveltekit-frame --json`
  - `patina spec check mother-view-composer --json`
  - `allium check layer/allium/mother/mother-view-composer-target.allium`

## Build Readiness

Ready. The final slice closes the product-complete path for [[mother-view-composer]] by making the Mother-owned buffer system visible through one renderer frame.

## Open Questions

- Should the next frame after SvelteKit be TUI or Emacs?
- Should payload streaming/subscription be added as a later slice, or is polling sufficient until buffer sharing semantics mature?
