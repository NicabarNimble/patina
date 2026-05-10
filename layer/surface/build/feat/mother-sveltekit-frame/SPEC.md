---
type: feat
id: mother-sveltekit-frame
status: ready
created: 2026-05-09
target: mother-view-composer
release_bump: patch
sessions:
  origin: 20260508-144836-859149000
related:
- mother-view-composer
- layer/allium/mother/mother-view-composer-target.allium
- mother-view-buffer-runtime
- mother-view-request-ux
- mother-view-maturation
exit_criteria:
- id: mskf1-frame-app
  text: A SvelteKit TypeScript frame exists in-repo with build/check scripts and documentation for connecting to a Mother daemon.
  checked: true
- id: mskf2-mother-owned-state
  text: The frame reads buffers, windows, shapes, requests, and observability gaps from Mother APIs and keeps only renderer/session-local selection state.
  checked: true
- id: mskf3-connect-disconnect
  text: The frame connects and disconnects SvelteKit windows through Mother-owned `/api/view-buffers/connect` and `/api/view-buffers/disconnect` calls.
  checked: true
- id: mskf4-render-framed-json
  text: The frame renders Mother `framed_json` payload envelopes for live/stale/blocked buffers without inventing rows when payload data is unavailable.
  checked: true
- id: mskf5-open-actions
  text: The frame can open an existing shape or request-linked shape only by calling Mother APIs and rendering the returned Mother-owned buffer/payload outcome.
  checked: true
- id: mskf6-verification
  text: SvelteKit check/build, Rust tests for added Mother API surface, `patina spec check`, and Allium check pass.
  checked: true
validated_against_commit: '75011474'
---
# feat: Mother SvelteKit Frame

> Implement the first TypeScript/SvelteKit frame that renders Mother-owned view buffers without owning buffer or shape state.

## Problem

[[mother-view-composer]] now has a Mother-owned backend substrate for shapes, requests, buffers, windows, gaps, revisions, maturation, and framed JSON payloads, but no visible frame. The system is therefore still not product-complete: users and agents can create buffers, but no renderer proves that a frame can attach to them without taking ownership of display state.

The old Atlas prototype was removed because it was a hardcoded visibility UI. The replacement must be a generic renderer frame over Mother view-buffer APIs.

## Goal

Build the first SvelteKit frame as a renderer/client for Mother-owned buffers:

- list Mother buffers, shapes, request details, windows, and gaps;
- connect/disconnect a SvelteKit window to an existing buffer through Mother APIs;
- render Mother `framed_json` payload envelopes for connectable buffers;
- open shapes/request-linked shapes only by asking Mother to open a buffer;
- keep only UI-local frame/window/selection state in the SvelteKit app.

## Status

Implemented in [[commit-75011474]]. Pending spec completion/release as `v0.70.7 — Mother View Composer: SvelteKit Frame`.

## Non-Goals

- Do not make SvelteKit the source of truth for buffers, shapes, requests, revisions, gaps, windows, or maturation.
- Do not generate arbitrary Svelte/TypeScript per request.
- Do not mutate Mother operational facts from the frame.
- Do not invent payload rows when Mother cannot provide framed JSON.
- Do not solve future TUI/Emacs/multiplayer frame semantics.

## Target Shape

A small `frames/sveltekit` app runs as a SvelteKit server/proxy and browser UI:

```text
Browser frame UI
  -> SvelteKit same-origin proxy
    -> Mother HTTP API
      -> Mother-owned buffers/shapes/windows/payloads
```

The proxy forwards configured auth to Mother, but persists no buffer or shape state. Browser-local state may include a generated `frame_id`, generated `window_id`, selected buffer id, and transient error/loading state.

## Solution

1. Add a Mother payload-read endpoint for existing buffers so frames can reconnect after page reload without reopening shapes.
2. Add `frames/sveltekit` with SvelteKit, TypeScript, and adapter-node.
3. Add typed Mother API client/proxy code for view-buffer endpoints.
4. Render buffer metadata plus framed JSON payloads for table/list/log/markdown/document/custom modes with safe fallback rendering.
5. Wire connect/disconnect/open actions to Mother APIs.
6. Document environment variables and verification commands.

## Implementation Order

1. Mother API: `GET /api/view-buffers/<buffer_id>/payload`.
2. SvelteKit package scaffold and typed API client.
3. Frame page/components and CSS.
4. Spec/design updates and verification.
5. Release as a patch under [[mother-view-composer]], then update the umbrella to `8/8`.

## Resolved Decisions

- The frame is a renderer, not a state owner.
- Payload fetch for existing buffers belongs in Mother because Mother owns payload envelopes and catalog-backed data.
- SvelteKit uses a same-origin server proxy so Mother auth tokens are not exposed to the browser.
- Rendering unknown/custom payloads is allowed only as literal JSON from Mother, not generated UI code.

## Verification

```bash
cargo fmt
cargo check -q
cargo test -q -p mother view_buffer_payload
cargo test -q -p mother view_buffer
cargo test -q -p mother
cd frames/sveltekit && npm run check && npm run build
patina spec check mother-sveltekit-frame --json
allium check layer/allium/mother/mother-view-composer-target.allium
```

## Exit Criteria

- [x] `mskf1-frame-app`
- [x] `mskf2-mother-owned-state`
- [x] `mskf3-connect-disconnect`
- [x] `mskf4-render-framed-json`
- [x] `mskf5-open-actions`
- [x] `mskf6-verification`

## Build Readiness

Ready after this draft validates and is promoted. The slice is intentionally small: one generic SvelteKit frame over existing Mother view-buffer APIs.
