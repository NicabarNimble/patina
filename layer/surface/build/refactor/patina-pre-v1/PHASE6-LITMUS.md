# Phase 6 DuckLake End-to-End Litmus

DuckLake now runs on `toy-github` and writes `issues`/`prs` rows to lake tables.

## Build Litmus

```bash
cargo component build --release --manifest-path children/ducklake/Cargo.toml
wc -c target/wasm32-wasip1/release/patina_ai_child_ducklake.wasm
```

Result:

- `target/wasm32-wasip1/release/patina_ai_child_ducklake.wasm`: `273,594` bytes

## Data Litmus Path

1. Configure source with owner/repo:
   - `{"source_id":"gh-main","table_prefix":"github","owner":"anthropics","repo":"claude-code","data_types":["issues","prs"]}`
2. Trigger fetch via `handle("fetch-source", {"source_id":"gh-main"})`.
3. Verify writes in checkpoint `ducklake.sync` and lake tables `github_issues`, `github_prs`.

If live GitHub API access fails (missing credential/rate limit), DuckLake automatically replays fixture payloads for `issues` and `prs` so ingest remains deterministic for validation.
