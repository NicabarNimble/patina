# Demo: Atlas Pando Mother UI

## 1) Build and start Mother

```bash
cargo build -q
target/debug/patina mother start
```

## 2) Verify pando registry includes atlas

```bash
target/debug/patina pando list --json | jq '.pandos[] | .name'
```

Expected includes:
- `"atlas"`
- `"folder-text-to-parquet"`

## 3) Verify Mother-hosted atlas web routes

```bash
curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/atlas | head -n 5
curl -s --unix-socket ~/.patina/run/serve.sock http://localhost/atlas/atlas.json | head -c 200
```

Expected:
- `/atlas` returns HTML containing `Patina Atlas`
- `/atlas/atlas.json` returns JSON payload starting with `{`

## 4) Verify standalone fallback remains available

```bash
target/debug/patina mother stop
target/debug/patina atlas --json | jq '.summary'
```

Expected: atlas summary still available from local snapshot.

## 5) Always-on posture (macOS)

```bash
patina mother install
patina mother status
```

Expected: launchd supervisor installed; Mother daemon managed outside atlas-specific CLI serve loop.
