# Patina Pre-v1 Closure Proofs

This file records concrete, reproducible evidence for remaining closure ECs.

## EC2 — Per-child WIT Worlds

- [x] `children/ducklake/Cargo.toml` uses `world = "ducklake"`
- [x] `children/belief-verifier/Cargo.toml` uses `world = "belief-verifier"`
- [x] `children/session-writer/Cargo.toml` uses `world = "session-writer"`
- [x] Linker enforcement tests pass:
  - `knowledge_child_linker_fails_when_lake_not_linked`
  - `knowledge_child_linker_succeeds_when_lake_declared`

Proof commands:

```sh
cargo test -q knowledge_child_linker_fails_when_lake_not_linked
cargo test -q knowledge_child_linker_succeeds_when_lake_declared
```

Observed result:

```text
test result: ok. 1 passed; 0 failed (both tests)
```

Notes:
- `child.toml` `kind = "knowledge-child"` is canonical runtime execution contract marker (`plugin.toml`/`world` read-compatible during migration).

## EC7 — DuckLake Queryability via DuckDB CLI

- [x] Fetch-and-store path produces rows in lake DB
- [x] Standalone DuckDB command and output recorded

DuckLake fixture-sync proof test:

```sh
cargo test -q ducklake_fixture_sync_writes_lake_queryable_by_duckdb_cli
```

Observed result:

```text
test result: ok. 1 passed; 0 failed
```

Command proof template:

```sh
duckdb "$HOME/.patina/lakes/default/lake.duckdb" "SELECT COUNT(*) AS c FROM NicabarNimble_patina_issues;"
```

Output:

```text
┌───────┐
│   c   │
│ int64 │
├───────┤
│ 14676 │
└───────┘
```

## EC15 — External Developer Onramp

- [x] `cargo generate` template invocation captured
- [x] Generated child builds for `wasm32-wasip2`
- [x] Install/load proof captured (local or CI)
- [x] End-to-end elapsed time captured (<5 minutes target)

Command proof template:

```sh
cargo generate --path children/template --name generated-child --define child_name=generated-child --define package_name=patina-ai-child-generated-child --define description="Generated child"
cargo check --target wasm32-wasip2 --quiet
```

Verification command:

```sh
cargo test -q cargo_generate_template_builds_for_wasm
```

Output/time:

```text
elapsed_seconds 0.67
returncode 0
test result: ok. 1 passed; 0 failed
```
