# Design: clippy backlog batch 1

## Why This Design

This batch favors low-risk, semantics-preserving edits to improve CI reliability quickly without destabilizing active feature work.

## Build Target

Reduce strict clippy failures by addressing a focused subset of mechanical lints in tests/helpers first.

## Resolved Decisions

- Avoid broad refactors in this batch.
- Keep changes small and verify with strict clippy + bin tests.

## Commits

- Pending.

## Direct Code Targets

- `src/connect/internal/store.rs`
- `src/plugin/internal/tests.rs`
- `src/secrets/vault.rs`
- `src/connect/internal/model.rs`
- `tests/embeddings_integration.rs`

## Verification Plan

- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --bin patina`

## Build Readiness

Ready.

## Open Questions

- None for this low-risk batch.
