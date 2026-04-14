---
type: feat
id: mother-core-worker-topology
status: active
created: 2026-04-14
sessions:
  origin: 20260413-075041-892082000
beliefs:
  - "[[patina-identity]]"
  - "[[spec-driven-design]]"
  - "[[dependable-rust]]"
  - "[[adapter-pattern]]"
  - "[[safety-boundaries]]"
  - "[[unix-philosophy]]"
related:
  - layer/core/values/patina-identity.md
  - layer/core/values/spec-driven-design.md
  - layer/core/values/dependable-rust.md
  - layer/core/values/adapter-pattern.md
  - layer/core/values/safety-boundaries.md
  - layer/core/values/unix-philosophy.md
  - src/commands/mother/mod.rs
  - src/commands/mother/daemon.rs
  - mother/src/runtime.rs
  - mother/src/http_api.rs
  - mother/src/http_routes.rs
  - layer/surface/build/feat/mother-rivet-integration/SPEC.md
exit_criteria:
  - id: mct1-startup-profile
    text: "`patina mother start` supports explicit startup profile (`full` default, `core` minimal) without changing existing default behavior."
    checked: true
    verify: "patina mother start --help"
  - id: mct2-core-does-not-autowarm
    text: "Core profile starts control plane without automatic child warmup so Mother can run always-on with minimal startup work."
    checked: true
    verify: "patina mother start --profile core && patina mother status"
  - id: mct3-manual-warmup-lifecycle
    text: "Child warmup can be started explicitly after daemon boot through lifecycle API/CLI command."
    checked: true
    verify: "patina mother lifecycle warmup-children"
  - id: mct4-readiness-visibility
    text: "Health/readiness surface reports startup profile and whether child warmup is pending/complete."
    checked: true
    verify: "patina mother status"
  - id: mct5-safe-idempotent-warmup
    text: "Warmup command is idempotent and fail-closed under concurrent invocation (`operation_in_progress`)."
    checked: true
    verify: "cargo test -p mother http_api::tests::lifecycle_reload_maps_operation_in_progress_to_409_envelope -- --nocapture"
  - id: mct6-slice-tests
    text: "Deterministic tests cover profile parsing and warmup lifecycle failure-path behavior."
    checked: true
    verify: "cargo test -p patina-ai daemon_options_default -- --nocapture"
---
# feat: Mother core/worker startup topology

> Keep Mother always-on as a small control plane, and activate heavier child warmup work explicitly when needed.

## Problem

Mother currently starts as one integrated path and launches child warmup automatically. As responsibilities grow (Rivet ingress, typed execution orchestration, lifecycle operations), startup coupling increases operator risk and makes always-on posture less predictable.

## Goal

Split startup into:
- **Core control-plane boot** (always-on, minimal startup), and
- **Worker warmup activation** (explicitly triggered, observable, idempotent).

This preserves current default behavior while enabling safer operations for long-running Mother deployments.

## Status

Active. Slice A targets profile + warmup separation for child activation only.

## Non-Goals

- Full multi-process worker orchestration in this slice.
- Replacing existing typed call runtime boundary.
- Introducing speculative abstraction layers before a second implementation exists.

## Target Shape

- `patina mother start --profile full` (default): existing behavior.
- `patina mother start --profile core`: control plane boot, no child autowarm.
- `patina mother lifecycle warmup-children`: explicit warmup trigger.
- Status/health surfaces expose current profile + warmup posture.

## Solution

1. Add startup profile model to daemon options and CLI.
2. Extract child warmup execution into a reusable lifecycle path.
3. Gate auto warmup by profile.
4. Add explicit lifecycle route/CLI command for warmup trigger.
5. Extend health/readiness details to expose profile/warmup state.

## Implementation Order

1. CLI profile parsing (`start --profile`) + daemon option plumbing.
2. Warmup execution extraction + lifecycle command (`warmup-children`).
3. Health/readiness fields for profile/warmup state.
4. Tests for profile parsing and concurrent warmup fail-closed behavior.

## Resolved Decisions

- Use profile gating, not hidden env vars, for startup behavior.
- Keep `full` as default for backwards compatibility.
- Treat warmup as lifecycle operation with clear API semantics.

## Verification

```bash
patina mother start --help
patina mother start --profile core
patina mother lifecycle warmup-children
patina mother status
```

## Exit Criteria

Use structured frontmatter checks (`mct1..mct6`).

## Build Readiness

Constrained to startup/lifecycle surfaces only; no child contract changes.
