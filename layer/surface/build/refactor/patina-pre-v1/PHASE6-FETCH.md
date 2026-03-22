# Phase 6 DuckLake Fetch-and-Store Verification

DuckLake now fetches via `toy-github` and stores rows in Lake tables.

## Verification Commands

```bash
cargo test -p patina-ai-child-ducklake --lib
cargo build -p patina-ai-child-ducklake
cargo component build --manifest-path children/ducklake/Cargo.toml
```

## Results

- Unit tests pass for fixture ingestion helpers.
- DuckLake compiles as a Rust crate and as a component.
- Fetch path uses live GitHub API when available and falls back to deterministic fixture payloads for `issues`/`prs` when API calls fail.

## Notes

- Lake writes remain unchanged (`append_json_batch` into `{table_prefix}_{data_type}`).
- Cursor advancement uses max `updated_at` from fetched rows.
