# Patina Pre-v1 Closure Proofs

This file records concrete, reproducible evidence for remaining closure ECs.

## EC2 — Per-child WIT Worlds

- [ ] `children/ducklake/Cargo.toml` uses `world = "ducklake"`
- [ ] `children/belief-verifier/Cargo.toml` uses `world = "belief-verifier"`
- [ ] `children/session-writer/Cargo.toml` uses `world = "session-writer"`
- [ ] Linker enforcement tests pass:
  - `knowledge_child_linker_fails_when_lake_not_linked`
  - `knowledge_child_linker_succeeds_when_lake_declared`

Notes:
- `child.toml` `kind = "knowledge-child"` is canonical runtime execution contract marker (`plugin.toml`/`world` read-compatible during migration).

## EC7 — DuckLake Queryability via DuckDB CLI

- [ ] Fetch-and-store path produces rows in lake DB
- [ ] Standalone DuckDB command and output recorded

Command proof template:

```sh
duckdb <path>/lake.duckdb "SELECT COUNT(*) FROM <table_name>;"
```

Output:

```text
<paste command output>
```

## EC15 — External Developer Onramp

- [ ] `cargo generate` template invocation captured
- [ ] Generated child builds for `wasm32-wasip2`
- [ ] Install/load proof captured (local or CI)
- [ ] End-to-end elapsed time captured (<5 minutes target)

Command proof template:

```sh
cargo generate --path children/template --name generated-child --define child_name=generated-child --define package_name=patina-ai-child-generated-child --define description="Generated child"
cargo check --target wasm32-wasip2 --quiet
```

Output/time:

```text
<paste command output and elapsed time>
```
