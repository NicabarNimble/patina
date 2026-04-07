---
type: fix
id: mother-startup-observability
status: active
created: 2026-04-07
beliefs:
  - "[[mother-manages-artifact-install-and-runtime]]"
  - "[[children-are-portable-wasm-artifacts]]"
related:
  - src/commands/mother/daemon.rs
  - mother/src/registry.rs
  - mother/src/daemon_bootstrap_config.rs
  - mother/src/daemon_runner.rs
  - mother/src/lifecycle.rs
  - mother/src/http_api.rs
  - mother/src/state.rs
blocks:
  - pando-platform
exit_criteria:

  - id: mso1-startup-stage-events
    text: "Mother startup emits structured stage events (begin/success/failure + duration) for pre-bootstrap phases: child discovery, child registration, child on_load, registry load_all, router build, transport bootstrap. Events are additive and logged through existing tracing JSON logs."
    checked: false

  - id: mso2-child-load-granularity
    text: "Per-child startup observability exists for discovery and on_load boundaries: child name, wasm path, manifest path, and elapsed time are logged. A blocked child can be identified from logs without a debugger."
    checked: false

  - id: mso3-prebootstrap-failure-surface
    text: "If startup fails before PID/socket creation, `patina mother start` prints a concise failure summary pointing to log location and last startup stage. No silent hangs."
    checked: false

  - id: mso4-additive-status-introspection
    text: "`patina mother status` includes additive startup diagnostics when daemon is not running but recent startup attempts failed (last stage, timestamp, error excerpt). Existing status output remains backward compatible."
    checked: false

  - id: mso5-no-telemetry-replacement
    text: "Existing telemetry surfaces are preserved: `mother.jsonl` tracing logs, `/health`, heartbeat logs, and `measure.metric` eventlog writes. New startup observability adds fields/events but does not replace sinks or formats."
    checked: false

  - id: mso6-regression-proof
    text: "`cargo check --workspace -q` and `cargo test -q --lib` pass. Added tests cover startup stage emission and pre-bootstrap failure reporting behavior."
    checked: false
---
# fix: Mother Startup Observability

## Problem

Mother can fail or hang before daemon bootstrap completes (before PID/socket
creation). In that pre-bootstrap window, existing telemetry is incomplete:
operators see "stopped" status with weak causal detail, and the blocking child
or startup stage is not obvious.

This blocks operational confidence and slows debugging of real-world startup
issues in a recently reworked Mother path.

## Scope

Add startup observability by extending existing telemetry and status surfaces.
Do not replace logging backends, state stores, or runtime architecture.

## Add, Don't Replace

This fix is additive:

- Keep existing `mother.jsonl` tracing sink.
- Keep `/health` and current status semantics.
- Keep heartbeat + eventlog metric flows.
- Add startup-stage and per-child load instrumentation around existing code.

## Existing Telemetry To Lean On

- Structured tracing logs (`mother/src/daemon_bootstrap_config.rs`).
- Runtime health/status API (`mother/src/http_api.rs`, `mother/src/lifecycle.rs`).
- Child handle metrics in eventlog (`mother/src/registry.rs`).
- Heartbeat run/checkpoint logs (`mother/src/daemon_heartbeat.rs`).

## Design Notes

1. Add a startup-stage tracker around `run_server` pre-bootstrap path.
2. Persist minimal "last startup attempt" diagnostics in existing Mother state.
3. Surface last failed stage in `patina mother status` when daemon is down.
4. Emit per-child load boundaries around discovery/register/on_load.

## Non-Goals

- No new observability stack (no OpenTelemetry migration, no new log daemon).
- No replacement of lifecycle transport.
- No pando lifecycle redesign in this fix.
