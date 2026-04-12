# Atlas UI (SvelteKit Scaffold)

This is a thin SvelteKit scaffold for the Atlas top layer.

Current Mother-hosted atlas routes:

- `GET /atlas` (server-rendered fallback HTML)
- `GET /atlas/atlas.json` (snapshot JSON)
- `GET /api/atlas/snapshot` (control-plane JSON)

## Local dev (optional)

```bash
cd ui/atlas
npm install
npm run dev
```

Default page fetches `http://localhost:50051/atlas/atlas.json`.

> Note: Mother TCP mode may require bearer auth depending on runtime config.
