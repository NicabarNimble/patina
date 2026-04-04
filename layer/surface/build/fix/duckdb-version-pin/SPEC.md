---
type: fix
id: duckdb-version-pin
status: draft
created: 2026-04-03
sessions:
  origin: 20260402-220939-443502000
related:
- .github/workflows/test.yml
- Cargo.toml
exit_criteria:
  - id: dvp1-ci-updated
    text: "All 4 'Install DuckDB' steps in .github/workflows/test.yml updated from v1.1.3 to v1.5.1 (lint, test, release, benchmark jobs). Asset name libduckdb-linux-amd64.zip verified to exist in the v1.5.1 GitHub release."
    checked: false
  - id: dvp2-cargo-pinned
    text: "Cargo.toml duckdb dependency updated from version = \"1\" to version = \"=1.5.1\". cargo update -p duckdb --precise 1.5.1 run to resolve Cargo.lock to exactly 1.5.1."
    checked: false
  - id: dvp3-verify
    text: "cargo build -p patina-ai --no-default-features --features bundled-doctor -q passes (confirms non-bundled linkage against system libduckdb)."
    checked: false
---
# fix: Bump DuckDB prebuilt from v1.1.3 to v1.5.1, align Cargo pin

## BLOCKED — Crate Version Scheme Changed

The `duckdb` Rust crate changed its versioning scheme. Latest on crates.io is
`1.10501.0` (encoding DuckDB v1.5.1 as `1.10501`), not `1.5.1`. The exit
criteria and fix steps below assume `=1.5.1` which does not exist on crates.io.
dvp2 needs correcting once the version mapping is understood. The CI binary
target (v1.5.1) is confirmed — `libduckdb-linux-amd64.zip` exists in the
GitHub release. The Cargo-side pin is what's wrong.

Needs: research into duckdb-rs crate versioning scheme before execution.

---

## Problem

CI has been pinned to `libduckdb v1.1.3` since October 2025. The Rust crate
in Cargo.lock has drifted to `duckdb 1.4.4` and local brew install is `v1.5.1`.
A 3-minor-version gap between prebuilt binary and crate headers is fragile —
any new C API symbol used by the crate since 1.1.3 will cause a link error.

## Root Cause

`v1.1.3` was pinned in commit `36846325` when the bundled feature was removed.
It was never updated as the crate dependency drifted forward.

## Fix

1. Verify `libduckdb-linux-amd64.zip` exists in the [v1.5.1 GitHub release](https://github.com/duckdb/duckdb/releases/tag/v1.5.1)
2. Update all 4 `wget` lines in `.github/workflows/test.yml` from `v1.1.3` to `v1.5.1`
3. Update `Cargo.toml`: `duckdb = { version = "=1.5.1" }`
4. Run `cargo update -p duckdb --precise 1.5.1`
5. Run `cargo build -p patina-ai --no-default-features --features bundled-doctor -q`

Note: `release.yml` has no DuckDB install steps — no changes needed there.

Single commit: `fix(ci): bump DuckDB prebuilt from v1.1.3 to v1.5.1, align Cargo pin`
