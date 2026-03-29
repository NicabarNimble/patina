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
beliefs:
- "[[dependable-rust]]"
- "[[measure-first]]"
- "[[spec-needs-code-verification]]"
exit_criteria:
  - id: cep1-ci-green
    text: "CI passes on patina branch with zero test failures — all WASM child artifacts built and connection-dependent tests handled."
    checked: false
  - id: cep2-preflight-reproduces-ci
    text: "preflight-full.sh on a clean clone (no pre-built artifacts, no local config) produces the same pass/fail as CI."
    checked: false
  - id: cep3-ci-mirror-dockerfile
    text: "A CI-mirror Dockerfile exists that installs DuckDB, WASM toolchain, and ONNX runtime so test-linux.sh can run the full workspace."
    checked: false
  - id: cep4-wasm-build-in-ci
    text: "CI workflow installs wasm32-wasip2 target and builds required child WASM artifacts before running cargo test."
    checked: false
  - id: cep5-connection-tests-explicit
    text: "Tests requiring external connection config either mock the config, skip with clear reason, or CI provides the config. No silent panics."
    checked: false
---
# fix: ci-environment-parity

Successor to `test-suite-tiering`. The gate structure is correct; the
CI environment does not match what the tests need.

## Problem

CI is red on `patina` branch. 5 tests fail in `cargo test --workspace`:

- 2 ducklake manifest tests panic because `github` connection is not
  configured in CI (`connection 'github' not found`)
- 3 folder-text-to-parquet pipeline tests panic because WASM child
  artifacts are not built (`file-system-monitor WASM artifact missing`)

These pass locally because dev machines have pre-built WASM artifacts
and `~/.patina/connections/` config. CI has neither.

Additionally, `resources/scripts/test-linux.sh` only runs `patina-pipe`
tests because its Docker image lacks DuckDB, ONNX, and wasmtime. It
cannot reproduce CI failures locally.

## Root Cause

1. **Missing CI setup step**: CI does not install `wasm32-wasip2` target
   or build child WASM components before `cargo test --workspace`.
2. **Implicit connection dependency**: ducklake manifest tests assume a
   `github` connection exists at runtime. No mock or skip guard.
3. **Bare Docker image**: `test-linux.sh` uses `rust:$VERSION` with no
   native deps, so it can only test pure-Rust crates.

## Code-Truth Snapshot (2026-03-29)

CI run 23712316589 on `origin/patina` (commit babd5bb4):
- All new tiering steps passed (retired MCP, broker integration)
- `cargo test --workspace` failed: 692 passed, 5 failed, 2 ignored
- `Check schema consistency` step never ran (ordered after failing test step)
- `check_mother_health` dead-code warning present in broker + test steps

Failing tests (all in `src/child/internal/tests.rs`):
```
ducklake_manifest_runtime_grants_sdk_story_stays_connected
ducklake_manifest_uses_granted_ingress_not_ambient_http
folder_text_to_parquet_first_split_composes_via_events
folder_text_to_parquet_scan_contract_end_to_end
folder_text_to_parquet_six_child_pipeline_composes_via_events
```

## Fix

### CEP-G1: Add WASM build step to CI

- Install `wasm32-wasip2` target via `rustup target add`
- Build required child WASM components before test step
- Children needed: `file-system-monitor`, `folder-text-to-parquet`,
  and any other children referenced by integration tests

### CEP-G2: Handle connection-dependent tests

Either:
- (a) Provide a mock/test connection config in CI, or
- (b) Guard tests with explicit skip + reason when connection is absent
  (no silent panics — use `#[ignore]` or runtime skip with message)

### CEP-G3: CI-mirror Dockerfile

- Extend `resources/scripts/test-linux.sh` image to include:
  - DuckDB (matching CI version 1.1.3)
  - ONNX runtime
  - wasmtime deps
  - `wasm32-wasip2` target
- Goal: `test-linux.sh --workspace` produces same result as CI

### CEP-G4: Fix CI step ordering

- Move `Check schema consistency` before `Run Rust tests` (or use
  `if: always()`) so it runs regardless of test outcome
- Remove `check_mother_health` dead code to clear warning noise

## Implementation Order

1. Fix CI: add WASM target + child builds (gets CI green fastest)
2. Guard connection-dependent tests (prevents future CI regressions)
3. Fix step ordering and dead code
4. Build CI-mirror Dockerfile (local repro capability)
5. Verify preflight-full.sh on clean clone matches CI

## Resolved Decisions

- Local is source of truth for the developer; CI is authority for merge
- Tests that need external config must be explicit about it, not panic
- Docker image for local testing should mirror CI, not the other way around
