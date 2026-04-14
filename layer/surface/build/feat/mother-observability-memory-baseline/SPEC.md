---
type: feat
id: mother-observability-memory-baseline
status: active
created: 2026-04-14
sessions:
  origin: 20260413-075041-892082000
beliefs:
  - "[[spec-driven-design]]"
  - "[[dependable-rust]]"
  - "[[safety-boundaries]]"
  - "[[unix-philosophy]]"
related:
  - layer/core/values/spec-driven-design.md
  - layer/core/values/dependable-rust.md
  - layer/core/values/safety-boundaries.md
  - layer/core/values/unix-philosophy.md
  - mother/src/http_daemon.rs
  - mother/src/http_api.rs
  - mother/src/lifecycle.rs
  - src/commands/mother/daemon.rs
  - src/commands/mother/mod.rs
exit_criteria:
  - id: mom1-request-correlation
    text: "Every Mother HTTP response includes `X-Request-Id`; incoming `X-Request-Id` is propagated unchanged, otherwise Mother generates one."
    checked: true
    verify: "cargo test -p mother http_daemon::tests::request_id_header_is_propagated -- --nocapture"
  - id: mom2-request-logging
    text: "Mother emits structured per-request logs with request id, method, path, status, and latency."
    checked: true
    verify: "rg -n 'http_request' ~/.patina/mother/logs/mother.jsonl | tail"
  - id: mom3-health-memory-signal
    text: "`/health` and `patina mother status` expose memory telemetry (`rss`, `max_rss`, optional soft limit, pressure classification)."
    checked: true
    verify: "PATINA_MOTHER_MEMORY_SOFT_LIMIT_MB=1 ./target/debug/patina mother start --host 127.0.0.1 --port 50124 --profile core"
  - id: mom4-soft-limit-policy
    text: "Optional memory soft limit (`PATINA_MOTHER_MEMORY_SOFT_LIMIT_MB` or `_BYTES`) classifies pressure and prevents child warmup when pressure is high."
    checked: true
    verify: "curl -sS -i -H \"Authorization: Bearer $TOKEN\" -H \"Content-Type: application/json\" -d '{}' http://127.0.0.1:50124/api/lifecycle/warmup-children"
  - id: mom5-fail-closed-envelope
    text: "Memory-pressure warmup denial is fail-closed and returns lifecycle error envelope with explicit `resource_exhausted` code."
    checked: true
    verify: "cargo test -p mother http_api::tests::lifecycle_warmup_maps_resource_exhausted_to_429_envelope -- --nocapture"
  - id: mom6-deterministic-tests
    text: "Deterministic tests cover request-id propagation and memory-pressure warmup denial behavior."
    checked: true
    verify: "cargo test -p mother http_daemon::tests::request_id_header_is_propagated -- --nocapture && cargo test -p mother http_api::tests::lifecycle_warmup_maps_resource_exhausted_to_429_envelope -- --nocapture"
---
# feat: Mother observability + memory baseline

> Add immediately useful operational visibility and basic memory pressure governance to Mother.

## Problem

Mother currently has useful readiness details but lacks two operator-critical capabilities:

1. End-to-end request correlation and low-friction request-level visibility.
2. Memory pressure signals and a policy seam to prevent heavy lifecycle operations under pressure.

Without these, Rivet integration and production operations cannot quickly answer “what happened?” and “why did this fail now?”.

## Goal

Ship a pragmatic baseline with no architectural churn:

- Correlatable request IDs at the HTTP boundary.
- Structured per-request observability.
- Health/status memory telemetry.
- Soft-limit guard that blocks child warmup under memory pressure.

## Non-Goals

- Full OpenTelemetry exporter in this slice.
- Full process supervisor and lane-level hard limits in this slice.
- Global admission control for every endpoint.

## Target Shape

- HTTP responses always include `X-Request-Id`.
- Request logs include: `request_id`, `method`, `path`, `status`, `latency_ms`, `response_bytes`.
- `/health` includes additive `memory` object.
- `patina mother status` prints memory and warmup posture.
- Warmup returns fail-closed error if memory pressure is high.

## Verification

```bash
cargo test -p mother http_daemon::tests::request_id_header_is_propagated -- --nocapture
cargo test -p mother http_api::tests::lifecycle_warmup_maps_resource_exhausted_to_429_envelope -- --nocapture
PATINA_MOTHER_MEMORY_SOFT_LIMIT_MB=1 patina mother lifecycle warmup-children
patina mother status
```
