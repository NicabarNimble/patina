# Patina Mother SvelteKit Frame

Renderer-only SvelteKit frame for [[mother-view-composer]]. Mother owns buffers, shapes, windows, requests, observability gaps, and framed JSON payloads; this app keeps only browser-local frame/window ids and selected UI state.

## Run

```bash
cd frames/sveltekit
npm install
MOTHER_API_BASE_URL=http://127.0.0.1:50051 npm run dev
```

Optional auth:

```bash
MOTHER_API_TOKEN=<token> npm run dev
```

`PATINA_MOTHER` and `PATINA_MOTHER_TOKEN` are also accepted. Values without a scheme are treated as `http://host:port`.

## Verification

```bash
npm run check
npm run build
```

## Frame contract

The browser calls the same-origin `/api/mother/*` proxy. The proxy forwards to Mother `/api/*` endpoints and never persists view state. Buffer rendering uses `GET /api/view-buffers/<buffer_id>/payload`; opening new buffers uses Mother-owned open endpoints.
