# Design: Mother Startup Observability

## Principle Alignment

- [[dependable-rust]]: isolate startup diagnostics behind small, explicit data structures.
- [[safety-boundaries]]: diagnostics are read-only and additive; no startup side effects beyond existing behavior.
- [[spec-driven-design]]: fix targets only startup visibility gaps before bootstrap, not runtime architecture changes.
- [[session-capture]]: preserve current scripts/workflow and improve actionable meaning in existing output.

## Why This Design

Mother observability is strong once the daemon is running (`mother.jsonl`, `/health`, heartbeat logs, event metrics), but weak in pre-bootstrap startup where child discovery/`on_load` can block before PID/socket creation. The design adds stage and per-child boundary telemetry to existing sinks so operators can identify "where it hung" without replacing the logging stack.

## Build Target

1. Emit startup stage begin/success/failure events with duration for pre-bootstrap phases.
2. Emit per-child discovery/register/on_load boundaries with child identity and elapsed time.
3. Persist last startup attempt diagnostics in existing Mother state and surface in `patina mother status` when down.
4. Keep all current telemetry sinks and status behavior backward compatible.

## Additive Architecture (No Replacement)

### Existing Surfaces (kept)

- `~/.patina/mother/logs/mother.jsonl` tracing sink.
- `/health` API and current lifecycle probe behavior.
- Heartbeat logs and runtime task/run state.
- `measure.metric` writes at child handle boundary.

### New Additions

- Startup stage tracker in `src/commands/mother/daemon.rs` around pre-bootstrap flow.
- Per-child startup boundary logs in `mother/src/daemon_bootstrap.rs` and `mother/src/registry.rs`.
- Last-startup diagnostics table/record in `mother/src/state.rs` (small additive schema).
- Status output enrichment in `src/commands/mother/mod.rs` + `mother/src/lifecycle.rs` when daemon is not running.

## Direct Code Targets

- `src/commands/mother/daemon.rs`
  - Wrap `run_server` pre-bootstrap phases with stage begin/end/fail logging.
  - Capture stage timing and failure reason for persistence.
- `mother/src/daemon_bootstrap.rs`
  - Add per-child discovery/register boundary logs (child name, wasm path, manifest path, duration).
- `mother/src/registry.rs`
  - Add per-child `on_load` boundary logs with duration and failure details.
- `mother/src/state.rs`
  - Add additive startup diagnostics storage (last attempt timestamp/stage/error).
- `mother/src/lifecycle.rs`
  - Extend status probe with optional recent startup diagnostics lookup when daemon is down.
- `src/commands/mother/mod.rs`
  - Print concise startup failure hint (`last stage`, `time`, `error excerpt`, log path).

## Commit Plan

1. `spec(mother-startup-observability): lock design and code targets`
2. `feat(mother): add startup stage telemetry before bootstrap — MSO1`
3. `feat(mother): add per-child load boundary telemetry — MSO2`
4. `feat(mother): persist and surface last startup failure diagnostics — MSO3/MSO4`
5. `test(mother): add startup observability regression coverage — MSO6`
6. `spec(mother-startup-observability): check criteria mso1-mso6`

## Verification Plan

- `cargo check --workspace -q`
- `cargo test -q --lib`
- Manual failure drill: force one child load failure and verify
  - stage failure appears in `mother.jsonl`
  - `patina mother start` prints concise failure summary
  - `patina mother status` (daemon down) prints last startup diagnostics.
- Manual success drill: clean startup still shows existing output and no telemetry regression.

## Out of Scope

- No transport/lifecycle redesign.
- No new telemetry backend (OpenTelemetry/metrics daemon/etc.).
- No pando lifecycle semantics changes in this fix.
