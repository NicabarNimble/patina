# DESIGN — mother-observability-memory-baseline

## Why this shape

This slice keeps to additive, low-risk seams:

- HTTP correlation is implemented at the transport edge (`http_daemon`) and does not require downstream API signature churn.
- Memory signals are additive in health payloads.
- Memory policy is scoped to child warmup, the heaviest startup/lifecycle action and the most immediate operator pain point.

## Planned changes

1. **Request correlation + request logs**
   - File: `mother/src/http_daemon.rs`
   - Add request-id extraction/generation (`X-Request-Id` passthrough or generated UUID).
   - Add response header injection (`X-Request-Id` always present).
   - Add structured `tracing::info!` per request with latency and status.

2. **Health memory telemetry**
   - Files: `mother/src/http_api.rs`, `src/commands/mother/daemon.rs`, `mother/src/lifecycle.rs`, `src/commands/mother/mod.rs`
   - Add additive `memory` object to health details/JSON.
   - Compute process memory snapshot in daemon (`rss` optional, `max_rss` from `getrusage`), classify pressure.
   - Extend `patina mother status` output to print startup/warmup and memory posture.

3. **Soft-limit guard (memory management baseline)**
   - File: `src/commands/mother/daemon.rs`
   - Parse optional env policy:
     - `PATINA_MOTHER_MEMORY_SOFT_LIMIT_MB`
     - `PATINA_MOTHER_MEMORY_SOFT_LIMIT_BYTES`
   - Deny `warmup_children` under high pressure with `resource_exhausted` fail-closed error.

4. **Lifecycle error mapping + tests**
   - File: `mother/src/http_api.rs`
   - Map `resource_exhausted:` errors to HTTP 429 lifecycle envelope.
   - Add deterministic tests for new mapping.

5. **Spec indexing**
   - File: `layer/surface/build/INDEX.md`
   - Add spec entry under Active.

## Risk and rollback

- Additive payload fields are backwards compatible.
- If pressure guard is too strict, operators can unset soft-limit env vars.
- Request-id header/logging is transport-only and can be reverted independently.
