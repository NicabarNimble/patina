---
type: refactor
id: integration-test-domain-split
status: active
created: 2026-04-15
beliefs:
  - "[[unix-philosophy]]"
  - "[[dependable-rust]]"
  - "[[spec-driven-design]]"
related:
  - tests/wasm_integration.rs
  - tests/wasm_integration/
  - tests/pando_parity.rs
  - tests/pando_parity/
  - layer/surface/reports/audit/2026-04-14-durable-rust-unix-realignment-audit.md
exit_criteria:
  - id: itds1-sut-axis-declared
    text: "Split axis is explicitly by subject-under-test (diagnostic ownership), not by file-size bucket."
    checked: true
  - id: itds2-wasm-integration-split
    text: "`tests/wasm_integration.rs` is decomposed into SUT modules with shared helpers isolated in `tests/wasm_integration/common.rs`."
    checked: true
  - id: itds3-pando-parity-split
    text: "`tests/pando_parity.rs` is decomposed into SUT modules with shared composition/parity helpers isolated in `tests/pando_parity/common.rs`."
    checked: true
  - id: itds4-no-behavior-change
    text: "Existing integration semantics remain unchanged: all previous wasm integration and pando parity tests still pass after split."
    checked: true
  - id: itds5-diagnosability-improved
    text: "Failing tests now map to subsystem-oriented module names (watch actor/sink, folder pipeline, models/repos, schema/dedup/extractor/write/catalog)."
    checked: true
---

# refactor: integration test domain split

## Problem

`tests/wasm_integration.rs` and `tests/pando_parity.rs` had grown into monolithic files, making failure diagnosis slower and ownership less obvious.

## Decision

Split by **subject-under-test (SUT)** so failure locations identify architectural subsystem boundaries directly.

## Module shape

### wasm integration
- `tests/wasm_integration/common.rs`
- `tests/wasm_integration/session_writer.rs`
- `tests/wasm_integration/watch_actor.rs`
- `tests/wasm_integration/watch_sink.rs`
- `tests/wasm_integration/folder_text_pipeline.rs`
- `tests/wasm_integration/wasm_models.rs`
- `tests/wasm_integration/wasm_repos.rs`
- `tests/wasm_integration/pipeline_echo.rs`
- `tests/wasm_integration/trap_handling.rs`
- `tests/wasm_integration/performance.rs`

### pando parity
- `tests/pando_parity/common.rs`
- `tests/pando_parity/schema_enforcer.rs`
- `tests/pando_parity/dedup_filter.rs`
- `tests/pando_parity/file_system_monitor.rs`
- `tests/pando_parity/content_extractor.rs`
- `tests/pando_parity/parquet_writer.rs`
- `tests/pando_parity/lakehouse_catalog.rs`

## Non-goals

- No test semantic changes.
- No fixture contract changes.
- No runtime/CLI behavior changes.

## Verification

```bash
cargo test -p patina-ai --test wasm_integration -- --nocapture
cargo test -p patina-ai --test pando_parity -- --nocapture
```
