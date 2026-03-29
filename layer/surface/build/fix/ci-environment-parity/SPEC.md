---
type: fix
id: ci-environment-parity
status: draft
created: 2026-03-29
sessions:
  origin: 20260329-100923-421215000
related:
- layer/surface/build/refactor/test-suite-tiering/SPEC.md
- .github/workflows/test.yml
- resources/scripts/test-linux.sh
- resources/git/preflight-full.sh
- src/child/internal/tests.rs
- .devcontainer/Dockerfile
- resources/docker/Dockerfile.ci-mirror
beliefs:
- "[[dependable-rust]]"
- "[[measure-first]]"
- "[[spec-needs-code-verification]]"
exit_criteria:
  - id: cep1-ci-green
    text: "CI passes on patina branch with zero test failures."
    checked: false
  - id: cep2-preflight-reproduces-ci
    text: "preflight-full.sh on a clean clone (no pre-built artifacts, no local config) produces the same pass/fail as CI."
    checked: false
  - id: cep3-ci-mirror-dockerfile
    text: "CI-mirror Dockerfile verified — test-linux.sh can run full workspace."
    checked: false
  - id: cep4-wasm-build-in-ci
    text: "CI workflow installs wasm32-wasip2 target and builds required child WASM artifacts before running cargo test."
    checked: true
  - id: cep5-connection-tests-explicit
    text: "Tests requiring external connection config provide fixture data. No silent panics."
    checked: true
  - id: cep6-no-cli-test-deps
    text: "Tests use embedded libraries, not CLI binaries on PATH (no duckdb CLI, no external tools)."
    checked: true
  - id: cep7-wasm-test-isolation
    text: "WASM integration tests (wasmtime-backed) are separated from unit tests so Tier 2 pre-push runs fast and CI can target them independently."
    checked: false
---
# fix: ci-environment-parity

Successor to `test-suite-tiering`. The gate structure is correct; the
CI environment does not match what the tests need.

## Problem

CI was red on `patina` branch. Multiple root causes:

1. Missing WASM toolchain and child builds in CI
2. Ducklake manifest tests assumed `~/.patina/connections/github.toml` existed
3. Tests shelled out to `duckdb` CLI binary (not installed in CI)
4. Schema consistency check ordered after failing test step (never ran)
5. `check_mother_health` dead code producing warnings
6. 11 WASM-backed integration tests mixed into `--lib` binary with 689 fast tests

## Progress (Session 20260329-100539-136413000)

### Done — pushed and verified

| Fix | Commit | Impact |
|---|---|---|
| Connection fixture for ducklake tests | `71e6fa6e` | Tests provide mock `github.toml` via temp `PATINA_HOME` — no implicit env dependency |
| Remove `check_mother_health` dead code | `71e6fa6e` | Eliminates `dead_code` warning in CI |
| WASM target + 7 child builds in CI | `2156c729` | `rustup target add wasm32-wasip2` + builds file-system-monitor, folder-text-to-parquet, content-extractor, schema-enforcer, dedup-filter, record-writer, lakehouse-catalog |
| Schema check before tests | `2156c729` | Moved `cargo run --release --quiet -- schema check` before `cargo test --workspace` |
| CI-mirror Dockerfile | `c8f7543e` | `resources/docker/Dockerfile.ci-mirror` with DuckDB 1.1.3, wasm32-wasip2, system deps |
| Upgraded `test-linux.sh` | `c8f7543e` | Auto-detects CI-mirror image for full workspace; falls back to bare rust image |
| Replace duckdb CLI with embedded queries | `b3e69517` | Three `Command::new("duckdb")` calls → `duckdb::Connection::open_in_memory()` |

### CI results after fixes

- Run 23712316589 (before): 692 passed, 5 failed, 2 ignored
- Run 23715146057 (after): 694 passed, 3 failed, 2 ignored
- Ducklake connection tests: **fixed** (both pass)
- `scan_contract_end_to_end`: failed on `duckdb` CLI not found — **fixed** by embedded query replacement
- `first_split_composes_via_events`: `processed_records` returned 8 instead of 4 — **pre-existing child behavior issue**, passes locally in debug and release
- `six_child_pipeline_composes_via_events`: failed on `duckdb` CLI not found — **fixed** by embedded query replacement

### Remaining work

**cep1 (CI green):** Likely 1 remaining failure (`first_split` 8-vs-4). This is a child behavior issue in `folder-text-to-parquet`, not CI environment. Investigate whether it's test isolation (shared `env_test_mutex` ordering), fixture path difference, or actual child logic divergence on CI.

**cep2 (preflight reproduces CI):** Not yet verified on clean clone.

**cep3 (Dockerfile verified):** Built but not tested. Run `docker build -f resources/docker/Dockerfile.ci-mirror -t patina-ci-mirror .` then `./resources/scripts/test-linux.sh` to verify.

**cep7 (WASM test isolation):** This is the key structural fix. Current state:

- 11 WASM-backed tests in `src/child/internal/tests.rs` run inside `cargo test -p patina-ai --lib`
- They're serialized behind `env_test_mutex` and take ~7 minutes
- Mixed with 689 fast unit tests in the same binary
- Causes SSH timeout on `git push` when Tier 2 escalates to full workspace
- Types used by these tests (`KnowledgeChildEngine`, `ChildManifest`, `FilesystemPreopen`) are in `pub(crate)` module — not accessible from `tests/` integration tests without visibility changes

**Approach for cep7:**
1. Re-export needed types through `child::mod.rs` (scoped, not blanket `pub`)
2. Move 11 WASM tests to `tests/wasm_integration.rs`
3. Move shared helpers (`with_temp_patina_home`, `write_github_connection_fixture`, component path finders) to a test support module
4. Update Tier 2 pre-push to run `--lib` only (fast)
5. Add path-triggered integration lane: when `src/child/`, `children/`, `src/toys/` change, also run `--test wasm_integration`
6. CI runs both targets explicitly

**Reference: wasmtime test patterns.**
The `bytecodealliance/wasmtime` repo (registered at `patina repo show bytecodealliance/wasmtime`)
uses `cargo nextest` in CI explicitly for parallel WASM test execution. See
`.github/workflows/main.yml` — they pin `CARGO_NEXTEST_VERSION` and run per-test-process
parallelism. This eliminates the `env_test_mutex` serialization problem because each test
gets its own process with its own env vars. Study their test organization before implementing
cep7 — they solved the same problem at much larger scale (~500k lines of Rust, heavy WASM
integration tests). Key links:
- Repo: `~/.patina/cache/repos/bytecodealliance/wasmtime/`
- CI: `.github/workflows/main.yml`
- nextest docs: https://nexte.st/

**Why this matters for child-construction-canon:** every new child adds WASM compilation + wasmtime instantiation to the test suite. MVP2 adds 3 children, MVP3 adds 2 more. Without isolation, push times grow linearly and the SSH timeout becomes a permanent blocker.

## Known CI Time Sinks

**`check-ducklake-parity.sh`**: 5 separate `cargo test -q -p patina-ai` invocations, each
re-linking the same 700-test binary to run one test. ~12 minutes for 5 tests. Fix: collapse
into one invocation with a substring filter. Same for **`check-broker-integration.sh`** (3
invocations). Expected savings: ~10 minutes per CI run. Only loss is per-invocation CI step
granularity, which `cargo test` output already provides at the test level.

**Three cargo profiles = three full compiles.** CI currently runs clippy (check mode, ~10 min),
tests (test mode, recompiles), and schema consistency + release build (release mode, ~18 min).
These can't share artifacts across profiles. Potential mitigations:
- Move schema check to run *after* `cargo build --release` (reuse the release binary)
- Consider whether clippy can run on a subset (changed packages) in CI like Tier 2 does locally
- Long-term: `cargo nextest` can replace both `cargo test` and the parity/broker scripts

**CI run 23715146057 step timing (2026-03-29):**
- WASM child builds: 1 min
- DuckLake parity: 11.5 min (5 redundant compilations)
- Clippy: 10 min (full workspace, check mode)
- Schema consistency: 18 min (full release build for `patina schema check`)
- Total wall time before tests even start: ~40 min

## Target CI Lane Structure

Current CI runs everything on every push (~50+ min). Target: three lanes
that match what actually changed.

### Fast lane (every push)
- Structural policy checks (Tier 1 — cargo-free, <1 min)
- `cargo clippy --workspace` (~10 min, unavoidable but cacheable)
- `cargo nextest run --workspace --lib` (689 unit tests, <1 min execution)
- Collapse ducklake parity + broker invariants into single test invocations (~2 min)
- **Target: ~15 min total**

### Integration lane (push touches `src/child/`, `children/`, `src/toys/`, `wit/`)
- Everything in fast lane
- `cargo nextest run --workspace --test wasm_integration` (11 WASM tests, ~2 min with nextest parallelism)
- Path-triggered: use `paths` filter in GitHub Actions or conditional step
- **Target: ~18 min total**

### Full lane (PR merge gate + manual trigger)
- Everything in integration lane
- `cargo build --release` → `patina schema check` (reuse binary, no double compile)
- `cargo install --path . --locked` (install test)
- **Target: ~25 min total**

### What this eliminates
- 40 min of redundant compilation on every push
- 8 separate `cargo test` invocations (ducklake + broker scripts)
- Release build on every push (only needed for merge gate)
- WASM integration tests on pushes that don't touch child code
- Serialized WASM test execution (nextest parallelizes)

### Implementation steps
1. Adopt `cargo nextest` in CI (drop-in replacement, zero code changes)
2. Collapse parity/broker scripts into single invocations
3. Complete cep7 (WASM test isolation into separate target)
4. Reorder CI: build release first, then schema check reuses binary
5. Split `.github/workflows/test.yml` into fast/integration/full lanes with path triggers
6. Add `workflow_dispatch` trigger for manual full-lane runs

## Test Audit (post-MVP1 stability)

After MVP1 stabilizes, audit the existing ~700 unit tests for:
- **Stale tests**: tests that pass but verify behavior that was refactored away (no longer proving anything useful)
- **Implementation-coupled tests**: tests that break on refactor, not on bugs (testing *how*, not *what*)
- **Duplicate coverage**: multiple tests asserting the same invariant through different paths
- **Missing boundary coverage**: the 5 CI failures we fixed were all *untested assumptions* (env deps, CLI deps, connection config) — look for more of these

Don't do this audit while the architecture is still moving. Do it once child-construction-canon
MVP1 exit criteria (ftp4-ftp7) are met and the composition model is stable.

**Test type roadmap:**
- Now: unit tests + composition tests (what we have). Isolate and fix, don't add new types.
- Post-MVP1: property tests for serialization (schemas, manifests, events), contract tests at child/toy boundaries.
- Post-MVP2: end-to-end pipeline tests (objective → children → output), regression tests anchored to real bugs.

## Resolved Decisions

- Local is source of truth for the developer; CI is authority for merge
- Tests that need external config must provide fixtures, not assume env state
- Docker image for local testing mirrors CI, not the other way around
- Tests use embedded libraries, not CLI binaries on PATH
- WASM integration tests are a separate test target from unit tests
- `cargo nextest` replaces `cargo test` in CI for parallel execution
- CI lanes match what changed — not everything on every push
- `--no-verify` push is acceptable when pre-push hook has already passed locally and SSH timeout is the only blocker (temporary until cep7 lands)
