# Design: CI Environment Parity

## Why This Design

The test-suite-tiering spec solved gate *structure* (Tier 0-4). This spec solves
gate *environment* — making CI, local preflight, and Docker testing produce the
same results on the same code.

The secondary goal is CI *speed*. Current CI runs 50+ minutes because of redundant
compilation, serialized WASM tests, and single-lane "run everything on every push."

## WASM Test Isolation (cep7)

### Problem

11 WASM-backed tests live in `src/child/internal/tests.rs` alongside 689 fast unit
tests. All 700 compile into one binary. The WASM tests are serialized behind
`env_test_mutex` (because they mutate `PATINA_HOME` env var) and take ~7 minutes.
This blocks git push via SSH timeout and makes Tier 2 pre-push impractically slow.

### Visibility Wall

The WASM tests use types from `child::internal` which is `pub(crate)`:
- `KnowledgeChildEngine` (in `child::internal::knowledge_child`)
- `ChildManifest` (in `child::internal::mod`)
- `FilesystemPreopen`, `FilesystemAccessMode` (in `child::internal`)
- `knowledge_child::FilesystemPreopen` (via `super::*`)

Moving tests to `tests/wasm_integration.rs` (a separate compilation unit) requires
these types to be accessible outside the crate.

### Approach

1. Add scoped re-exports in `src/child/mod.rs`:
   ```rust
   // Re-exports for integration testing. Not a public API commitment.
   #[doc(hidden)]
   pub mod testing {
       pub use super::internal::{
           ChildManifest, FilesystemAccessMode, FilesystemPreopen,
           KnowledgeChildEngine,
       };
   }
   ```

2. Move 11 WASM tests to `tests/wasm_integration.rs`.

3. Move shared test helpers to `src/test_support.rs` (or a `tests/support/` module):
   - `with_temp_patina_home`
   - `write_github_connection_fixture`
   - Component path resolution functions (`folder_text_to_parquet_component_path()`, etc.)

4. Update local hooks:
   - Tier 2 pre-push: `cargo test --lib` (or `cargo nextest run --lib`)
   - Path-triggered: when `src/child/`, `children/`, `src/child/toy_host/`, `mother/src/`
     change, also run `--test wasm_integration`

5. Update CI to run both targets explicitly.

### Risks

- `#[doc(hidden)]` re-exports are a convention, not enforcement. A future contributor
  could depend on them. Mitigate with a comment and CI lint if needed.
- `cargo nextest` may need config for tests that currently rely on `env_test_mutex`.
  In per-process mode the mutex is unnecessary (each process has its own env), but
  verify that no test relies on mutex for non-env shared state.

## CI Lane Design

### Current state (single lane, ~50 min)

```
checkout → rust → wasm target → duckdb → cache → models →
wasm child builds → structural checks → ducklake parity (11 min) →
broker (2 sec) → fmt → clippy (10 min) → schema (18 min) →
tests (~10 min) → release build → install test
```

### Target state (three lanes)

**Fast lane** (every push, ~15 min):
```
checkout → rust → duckdb → cache → models →
structural checks → fmt →
clippy (10 min, cached) →
nextest --lib (<1 min) →
ducklake + broker collapsed (2 min)
```

**Integration lane** (path trigger, ~18 min):
```
fast lane +
wasm target → wasm child builds (1 min) →
nextest --test wasm_integration (2 min parallel)
```

Path triggers: `src/child/**`, `children/**`, `src/child/toy_host/**`,
`mother/src/**`, `wit/**`, `sdk/**`

**Full lane** (PR merge + manual, ~25 min):
```
integration lane +
cargo build --release →
patina schema check (reuses release binary) →
cargo install --locked
```

### nextest Adoption

`cargo nextest` is a drop-in replacement for `cargo test` at the invocation level,
but requires validation for this codebase:

- **`env_test_mutex` tests**: In per-process mode each test gets its own env, so the
  mutex is unnecessary. But verify no test uses the mutex for non-env shared state
  (filesystem paths, database files, etc.).
- **Test output parsing**: Any CI scripts that parse `cargo test` output will need
  updating for nextest's format.
- **Config file**: nextest uses `.config/nextest.toml` for per-project config (retries,
  timeouts, profiles). May want a `ci` profile with different settings than local.
- **Installation**: Pin version in CI (wasmtime pins `CARGO_NEXTEST_VERSION`).

### Compilation Savings

| Change | Savings |
|---|---|
| Collapse ducklake parity (5 → 1 invocation) | ~10 min |
| Collapse broker integration (3 → 1 invocation) | ~1 min |
| Build release before schema check (reuse binary) | ~18 min |
| Skip release build + schema on non-merge pushes | ~18 min |
| nextest parallel WASM tests | ~5 min |
| **Total potential** | **~35 min per fast-lane run** |

## Verification

```bash
# After cep7: verify unit tests run without WASM
cargo test --lib  # should complete in <30 seconds

# After cep7: verify WASM tests run separately
cargo test --test wasm_integration  # should complete in ~2 min with nextest

# After lane split: verify fast lane
# (simulate with act or manual workflow_dispatch)

# After nextest adoption:
cargo nextest run --workspace --lib  # parallel unit tests
cargo nextest run --workspace --test wasm_integration  # parallel WASM tests
```

## Open Questions

- Should nextest adoption happen before or after cep7 test isolation? Nextest
  may resolve the serialization problem without moving files (per-process env
  isolation eliminates the mutex need). But test isolation is still valuable for
  CI lane targeting.
- Should the parity/broker scripts be replaced entirely by nextest filter
  expressions, or keep them as thin wrappers for local use?
- The `first_split` 8-vs-4 CI failure needs investigation. Is it a real child
  bug, a test ordering issue, or a CI filesystem characteristic?
