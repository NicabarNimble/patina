# Design: Mother Pando Bindings Runtime

## Why This Design

Mother is moving from an intermittently started helper to always-on local
infrastructure. Startup and runtime composition therefore need explicit
separation: control plane should become available quickly, while heavy child
activation proceeds independently with progress visibility.

This design also keeps CLI where it belongs (operator UX) while moving runtime
composition onto typed in-process bindings.

## Build Target

- Control-plane-first startup with background child activation.
- Typed runtime composition boundaries for pando-child-toy operations.
- Deterministic lifecycle operations: load, refresh, single-child reload.
- Additive readiness + startup telemetry suitable for bottleneck analysis.
- Manifest integrity verification (SHA-256) before any bindings granted.

## Resolved Decisions

- Internal runtime composition must not shell out to CLI on hot paths.
- Child warmup failures degrade specific children, not control-plane readiness.
- Observability is emitted through existing `measure.metric` + tracing sinks.
- Lifecycle endpoints are HTTP first; CLI is a thin client (same pattern as federation).
- Child reload by canonical name only; pando aliases resolved before calling reload.
- Reload failure keeps old instance active with existing health status (not degraded).
- Concurrent operations on same target rejected with 409, not queued.
- Manifest integrity: SHA-256 of pando.toml, child.toml, .wasm verified at load time.

## Commits
1. `feat(mother): split startup into control-plane and child warmup phases — MPBR1/MPBR5`
2. `feat(mother): emit per-child startup metrics and slow-child diagnostics — MPBR4`
3. `feat(mother): add MotherRuntime trait and lifecycle operations — MPBR2/MPBR3`
4. `feat(mother): add lifecycle HTTP endpoints and CLI commands — MPBR2/MPBR3`
5. `feat(mother): add manifest integrity verification — MPBR6`
6. `test(mother): add readiness and lifecycle proof coverage — MPBR7`

## Direct Code Targets
- `src/commands/mother/daemon.rs` — startup ordering + readiness state
- `src/commands/mother/loader.rs` — per-child activation instrumentation
- `mother/src/http_api.rs` — readiness payload extensions
- `mother/src/http_routes.rs` — lifecycle route registration (`/api/lifecycle/*`)
- `mother/src/runtime.rs` — `MotherRuntime` trait definition (existing file, extend)
- `src/mother/internal.rs` — control-plane client calls for lifecycle endpoints
- `src/commands/mother/mod.rs` — CLI surface for lifecycle operations
- `mother/src/registry.rs` — safe child reload/load orchestration boundaries
- `src/commands/mother/integrity.rs` — SHA-256 hash computation and verification (new file)
- `src/commands/mother/federation.rs` — updated to align with two-phase startup

## Verification Plan

1. `cargo check --workspace -q`
2. `cargo test -q --lib`
3. Start daemon and assert control-plane readiness before full child warmup
4. Execute lifecycle operations (`load-pando`, `refresh`, `reload-child`) and
   verify deterministic status/diagnostic responses
5. Confirm startup and child metrics in events DB for bottleneck attribution
6. Verify manifest integrity rejection on tampered file

## Build Readiness

Ready. The spec contains target shape, sequencing, and measurable acceptance
criteria. All open questions resolved in SPEC.md Resolved Decisions section.
