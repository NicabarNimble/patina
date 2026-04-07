# Design: pando-platform-phase-a

## Why This Design

- Capture completed foundational work as its own spec boundary so remaining
  pando-platform work (routing/migration/slate) can proceed independently.
- Keep traceability explicit: PP1/PP2 map directly to shipped parser,
  registry, collision, lifecycle, and list-command behavior.

## Commits (Executed)

1. `spec(pando-platform): add Phase A design lock — PP1/PP2`
2. `feat(pando-platform): add pando.toml manifest parser — PP1`
3. `feat(pando-platform): add Mother pando registry collision model — PP2`
4. `feat(pando-platform): wire Mother pando registry protocol — PP2`
5. `feat(pando-platform): add patina pando list command — PP2`
6. `spec(pando-platform): check pp1 pando manifest parser — PP1`
7. `spec(pando-platform): check pp2 Mother pando registry — PP2`

## Direct Code Targets (Delivered)

- `mother/src/pando.rs`
- `mother/src/http_api.rs`
- `mother/src/http_routes.rs`
- `src/commands/mother/daemon.rs`
- `src/mother/internal.rs`
- `src/commands/pando.rs`
- `src/main.rs`
- `src/paths.rs`
- `crates/patina-protocol/src/lib.rs`

## Verification

- `cargo check -p patina-protocol`
- `cargo check -p mother`
- `cargo check -p patina-ai`
- `cargo test -p mother`
- `patina pando list`
