# Design: Spec Atlas Live Server

## Build Target

Extend `atlas` command with serve mode:

- `patina atlas --serve`
- optional `--host` and `--port`

Serve mode runs a local blocking TCP listener that responds with atlas views.

## Routing

- `GET /` or `GET /index.html`
  - Build snapshot from repo truth
  - Render dashboard HTML
- `GET /atlas.json`
  - Build snapshot from repo truth
  - Return JSON
- `GET /health`
  - Return `ok`
- Unknown route -> `404`
- Non-GET method -> `405`
- Malformed request line -> `400`

## Safety Model

- Bind default `127.0.0.1` only.
- Read-only over repository files.
- No mutation routes.
- No Mother dependency.

## Direct Code Targets

- `src/main.rs` (atlas serve CLI args)
- `src/commands/atlas/mod.rs` (options)
- `src/commands/atlas/internal.rs` (HTTP serve loop + routing)
- `README.md` (serve command example)

## Tests

- request-line parsing success/failure
- non-GET -> 405
- unknown route -> 404
- dashboard/json routes return 200 and expected content types
