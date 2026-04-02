---
type: belief
id: signal-handlers-only-set-flags
persona: architect
facets: [posix, signals, rust, safety]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-31
revised: 2026-03-31
---

# signal-handlers-only-set-flags

Signal handlers must be async-signal-safe: only AtomicBool::store with Ordering::Relaxed — no file I/O, no allocation, no process::exit; all cleanup belongs in the main thread after the accept loop breaks

## Statement

Signal handlers must be async-signal-safe: only AtomicBool::store with Ordering::Relaxed — no file I/O, no allocation, no process::exit; all cleanup belongs in the main thread after the accept loop breaks

## Evidence

- [[session-20260331-080327-949611000]] - MEH-G6 rewrote daemon_lifecycle.rs signal handler from file removal + process::exit to single AtomicBool::store; coordinated shutdown sequence runs in main thread (weight: 0.95)

## Supports

- [[mother-is-the-daemon]] — Mother's reliability depends on correct signal handling; unsafe handlers risk corruption during shutdown
- [[sync-first]] — the pattern works because the accept loop is synchronous; the AtomicBool check is a natural part of the sync poll cycle

## Attacks

## Attacked-By

## Applied-In

- `mother/src/daemon_lifecycle.rs` — `sigint_handler` is exactly one line: `SHUTDOWN_REQUESTED.store(true, Ordering::Relaxed)` ([[commit-525f7fcd]])
- `mother/src/daemon_runner.rs` — coordinated shutdown sequence: drain pool → TRUNCATE checkpoint → stop heartbeat → remove PID/socket → exit ([[commit-525f7fcd]])
- `mother/src/http_daemon.rs` — accept loops check `shutdown_requested.load(Relaxed)` each iteration, break cleanly, return `DrainHandle` ([[commit-525f7fcd]])

## Revision Log

- 2026-03-31: Created — metrics computed by `patina scrape`
