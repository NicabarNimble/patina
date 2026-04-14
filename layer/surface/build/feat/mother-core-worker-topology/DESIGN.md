# Design: Mother core/worker startup topology

## Why This Design

Mother has transitioned from a thin daemon into the central control plane for typed orchestration. To remain dependable and always-on, startup must keep the control plane small and treat heavy activation work as explicit lifecycle operations.

This follows core values:
- `dependable-rust`: stable CLI/runtime surface, internals can evolve.
- `unix-philosophy`: control plane and warmup are separate jobs.
- `adapter-pattern`: profile seam is only introduced where we already have two real modes (`full`, `core`).

## Build Target

Introduce startup profiles and explicit warmup lifecycle command while keeping default behavior unchanged.

## Resolved Decisions

1. Add `--profile` to `patina mother start`; default remains `full`.
2. `core` profile disables automatic child warmup on boot.
3. Child warmup becomes explicit lifecycle operation (`warmup-children`).
4. Health/readiness should show profile + warmup state.
5. Concurrent warmup requests fail closed with conflict semantics.

## Commits
1. `spec: add mother-core-worker-topology` — authorize slice and criteria.
2. `feat(mother): add startup profiles and warmup lifecycle lane` — core behavior change.
3. `test(mother): cover profile parsing and warmup concurrency` — deterministic proof.

## Direct Code Targets

- `src/commands/mother/mod.rs`
  - add `start --profile`
  - add lifecycle subcommand `warmup-children`
- `src/commands/mother/daemon.rs`
  - parse/propagate profile
  - gate autowarm by profile
  - expose warmup lifecycle implementation and state
- `mother/src/runtime.rs`
  - runtime contract for warmup operation result shape
- `mother/src/http_api.rs`
  - lifecycle API method + handler for warmup route
  - health/readiness payload additions for profile/warmup
- `mother/src/http_routes.rs`
  - add lifecycle warmup route wiring

## Verification Plan

1. CLI surface:
   - `patina mother start --help` shows `--profile`.
2. Core profile boot:
   - start with `--profile core`, verify daemon healthy and no autowarm.
3. Manual warmup:
   - `patina mother lifecycle warmup-children` succeeds.
4. Concurrency guard:
   - second concurrent warmup returns conflict.

## Build Readiness

This slice is additive and backward compatible:
- `full` profile preserves existing startup behavior.
- existing lifecycle commands remain unchanged.

## Open Questions

- Should warmup state move to dedicated worker-state table once separate worker process lands?
- Should `patina mother install` gain profile pinning for launchd in this slice or follow-up?
