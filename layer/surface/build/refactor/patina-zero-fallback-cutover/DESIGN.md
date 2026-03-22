# Design: refactor: Patina Zero-Fallback Cutover

## Why This Design

`patina-pre-v1` intentionally delivered daemon-first routing before full daemon parity. That sequencing preserved behavior but left a hybrid runtime. This design finishes migration with a bounded cutover contract so "done" means no legacy execution fallback for migrated surfaces.

## Build Target

Daemon-only execution for migrated commands (`context`, `scry`, `assay`, `spec`, `measure`, `lake`) with scaffold-free daemon responses and enforced MCP retirement consistency.

## Resolved Decisions

- Use gate-based completion (5 binary gates), not additional broad phase expansion.
- Keep scope strictly migration-completion; do not introduce new features.
- Require poison-pill CI checks so cutover cannot regress silently.

## Commits
1. `cutover: remove embedded fallback branches for migrated commands` — enforce daemon-only command execution.
2. `cutover: replace daemon scaffold handlers for migrated actions` — remove placeholder responses.
3. `cutover: enforce retired MCP invariants across templates/setup` — prevent stale launch behavior.
4. `ci: add zero-fallback poison-pill checks` — fail fast on regressions.
5. `test: add daemon-only e2e matrix for migrated commands` — prove behavior at runtime boundary.

## Direct Code Targets
- `src/commands/context.rs`
- `src/commands/scry/mod.rs`
- `src/commands/assay/mod.rs`
- `src/commands/spec/mod.rs`
- `src/commands/measure/mod.rs`
- `src/commands/lake.rs`
- `src/mother/daemon_client.rs`
- `mother/src/daemon.rs`
- `resources/scripts/check-runtime-boundaries.sh`
- `resources/scripts/check-single-sdk-surface.sh`
- CI workflow under `.github/workflows/`

## Verification Plan

- Run full test suite (`cargo test -q`) after each cutover commit.
- Add targeted daemon integration tests per migrated action.
- Run grep-based poison-pill checks in CI and locally.
- Validate command matrix in daemon-connected mode with no embedded fallback.

## Build Readiness

Build is ready when `patina-pre-v1` is blocked by this spec and cutover criteria are treated as the sole completion path for pre-v1 architecture finalization.

## Open Questions

- For migrated commands, should daemon unavailability always auto-start, or should some commands fail fast with explicit remediation? (must be consistent and tested)
