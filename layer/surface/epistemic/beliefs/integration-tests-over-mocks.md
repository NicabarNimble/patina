---
type: belief
id: integration-tests-over-mocks
persona: architect
facets: [testing, rust, quality]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-03
revised: 2026-04-03
---

# integration-tests-over-mocks

Prefer integration tests with real implementations over mocks. Mocks are a last resort for when the real system is genuinely unavailable in CI, not a default testing strategy.

## Statement

Prefer integration tests with real implementations over mocks. Mocks are a last resort for when the real system is genuinely unavailable in CI, not a default testing strategy.

## Evidence

- [[session-20260403-070944-045859000]] - Codebase has 272 beliefs, real DuckDB connections, real ONNX inference, real WASM integration tests — and only one MockProvider in the entire src/ tree, used solely to prove trait object safety. (weight: 0.9)

## Supports

- [[adapter-is-dependable-rust-at-external-edges]] — trait boundaries exist to isolate external systems, not to invite mocks
- [[ci-lanes-match-change-scope]] — CI runs real tests against real systems, scoped to what changed

## Attacks

## Attacked-By

- External API rate limits and credential requirements may force mocks in CI for third-party services

## Applied-In

- `.github/workflows/test.yml`: real DuckDB prebuilt binary, real ONNX models, real WASM compilation
- `tests/wasm_integration.rs`: real `duckdb::Connection::open_in_memory()` for parquet verification
- Only mock in codebase: `MockProvider` in `src/connect/providers/mod.rs` — proves trait object safety, not behavior

## Revision Log

- 2026-04-03: Created — metrics computed by `patina scrape`
