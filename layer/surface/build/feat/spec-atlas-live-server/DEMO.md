# Demo: Atlas Live Server

Run from repo root.

## 1) Start local server

```bash
patina atlas --serve --port 7417
```

Expected startup output:
- `Atlas server listening on http://127.0.0.1:7417`
- `Routes: /, /atlas.json, /health`

## 2) Verify routes

```bash
curl -s http://127.0.0.1:7417/health
curl -s http://127.0.0.1:7417/atlas.json | jq '.summary'
curl -i http://127.0.0.1:7417/missing | head -n 1
curl -i -X POST http://127.0.0.1:7417/atlas.json | head -n 1
```

Expected:
- `/health` -> `ok`
- `/atlas.json` -> snapshot JSON with `summary`
- `/missing` -> `404 Not Found`
- `POST /atlas.json` -> `405 Method Not Allowed`

## 3) Open dashboard

```bash
open http://127.0.0.1:7417/
```

Dashboard auto-refreshes every 3 seconds in server mode.

## 4) Verification commands

```bash
cargo test -q atlas
cargo check -q
```
