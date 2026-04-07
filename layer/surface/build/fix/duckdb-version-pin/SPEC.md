---
type: fix
id: duckdb-version-pin
status: complete
created: 2026-04-03
sessions:
  origin: 20260402-220939-443502000
  active: 20260407-063612-748374000
related:
- .github/workflows/test.yml
- Cargo.toml
exit_criteria:
  - id: dvp1-ci-updated
    text: "All 4 'Install DuckDB' steps in .github/workflows/test.yml updated from v1.1.3 to v1.5.1. Asset libduckdb-linux-amd64.zip confirmed in v1.5.1 GitHub release."
    checked: true
  - id: dvp2-cargo-pinned
    text: "Cargo.toml duckdb dependency updated to version = \"1.10501\". Cargo.lock resolves to duckdb 1.10501.0 and libduckdb-sys 1.10501.0. Crate 1.10501.0 encodes DuckDB v1.5.1 per the new duckdb-rs versioning scheme (1.MAJOR_MINOR_PATCH.crate_patch)."
    checked: true
  - id: dvp3-verify
    text: "cargo check --workspace -q passes with the updated crate version."
    checked: true
---
# fix: Align DuckDB prebuilt and Rust crate to v1.5.1

## Problem

CI has been pinned to `libduckdb v1.1.3` since October 2025. The Rust crate
in Cargo.lock has drifted to `duckdb 1.4.4` and local brew install is `v1.5.1`.
A 3-minor-version gap between prebuilt binary and crate headers is fragile —
any new C API symbol used by the crate since 1.1.3 will cause a link error.
This blocks new DuckDB work (mother-duckdb-ducklake-federation).

## Root Cause

`v1.1.3` was pinned in commit `36846325` when the bundled feature was removed.
It was never updated as the crate dependency drifted forward.

## Crate Version Scheme (researched 2026-04-07)

The `duckdb-rs` crate changed versioning at DuckDB v1.5.0:
- **Before v1.5.0:** crate version = DuckDB version (e.g. crate 1.4.4 = DuckDB v1.4.4)
- **From v1.5.0:** encoded as `1.MAJOR_MINOR_PATCH.crate_patch` (e.g. crate 1.10501.0 = DuckDB v1.5.1)

`libduckdb-sys` shares the same version as `duckdb` in the workspace.

Available on crates.io: 1.10501.0 (latest, DuckDB v1.5.1), 1.10500.0 (DuckDB v1.5.0).

## Fix

1. Update all 4 `wget` lines in `.github/workflows/test.yml` from `v1.1.3` to `v1.5.1`
2. Update `Cargo.toml`: `duckdb = { version = "1.10501" }`
3. Run `cargo update -p duckdb`
4. Run `cargo check --workspace -q`

Single commit: `fix(ci): align DuckDB prebuilt and crate to v1.5.1 — DVP1/DVP2/DVP3`
