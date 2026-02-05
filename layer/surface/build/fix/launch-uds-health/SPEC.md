---
type: fix
id: launch-uds-health
status: complete
created: 2026-02-04
sessions:
  origin: 20260204-193822
related:
  - src/commands/launch/internal.rs
  - src/commands/serve/mod.rs
  - src/mother/internal.rs
upstream:
  - refactor/security-hardening
beliefs:
  - spec-before-code
---

# fix: launcher health check broken after UDS migration

> `patina` fails with "Failed to start mother daemon" — health check uses TCP but serve defaults to Unix socket.

## Problem

Phase 2 of [[security-hardening]] (commits a21f4d81..cdc2efc7, Feb 3) moved `patina serve` from TCP (rouille on port 50051) to Unix domain socket (`~/.patina/run/serve.sock`) by default. The mother client in `src/mother/internal.rs` was updated to use `uds_get()`/`uds_post()`, but the launcher's `check_mother_health()` was missed.

**Symptom:** Every `patina` invocation fails:
```
⏳ Starting mother...
Error: Failed to start mother daemon
```

**Root cause:** `check_mother_health()` sends HTTP GET to `http://127.0.0.1:50051/health` via reqwest. The daemon spawned by `start_mother_daemon()` runs `patina serve` with no args, which binds UDS only (no TCP). Health check never connects, times out after 5s (10 retries x 500ms).

## Fix

Replace reqwest TCP health check with UDS connection to `~/.patina/run/serve.sock`, matching the pattern in `src/mother/internal.rs:uds_get()`.

### Changes

1. **`src/commands/launch/internal.rs`**
   - Add `use patina::paths` and `use std::io::Read as _`
   - Replace `check_mother_health()`: connect via `UnixStream` to `paths::serve::socket_path()`, send `GET /health HTTP/1.1`, check for `200` in response
   - Remove reqwest dependency from launch path

## Verification

```
$ patina
  ⏳ Starting mother...
  ✓ Mother started
🚀 Launching Claude Code in /Users/nicabar/Projects/Sandbox/AI/RUST/patina
```

## Exit Criteria

- [x] `patina` starts mother daemon successfully
- [x] Health check connects via UDS, not TCP
- [x] No reqwest usage remaining in launch module
- [x] `cargo build --release` clean
- [x] `cargo clippy` no new warnings
