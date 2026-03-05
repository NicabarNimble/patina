---
type: refactor
id: continuous-operation
status: draft
created: 2026-03-04
blocked_by:
- data-architecture-v3
- persona-federation
sessions:
  origin: 20260304-120702
beliefs:
- mother-is-connection-and-continuity
- local-first-edge-deployable
- patina-is-beliefs-plus-action
exit_criteria:
- id: mother-daemon-runs-continuously
  text: Mother daemon starts on login (or explicit start), runs in background, survives terminal close
  checked: false
- id: connectors-sync-on-schedule
  text: connector plugins run on configurable schedules via Mother daemon — not just on CLI invocation
  checked: false
- id: belief-streams-flow
  text: belief updates in one persona trigger stream delivery to linked personas via Mother
  checked: false
---
# refactor: Continuous Operation — Mother Daemon and Streaming

> Mother runs continuously. Connectors pull, lakes sync, belief
> streams flow. Always on, not batch.

## Context

**Architecture context:**
- [[session-20260304-120702]] — user: "the reality is this should be
  continuous at the mother level. Projects probably more async. But
  maybe patina apps are different." Established: Mother is the
  heartbeat, always running. Projects are triggered by CLI or events.
- [[mother-is-connection-and-continuity]] — Mother's job is connection
  (federation) and continuity (always running). Not batch commands.
- [[local-first-edge-deployable]] — edge apps need a continuously
  available Mother to sync beliefs and events back.

**What exists today:**
- Mother daemon (`patina mother daemon`) can run mother-child plugins
- `tick()` and `health()` host calls provide heartbeat pattern
- Forge background sync uses `libc::fork()` (Unix-only, fragile)
- No scheduled connector execution
- No belief stream delivery
- No daemon auto-start

## Problem

Patina is currently batch/CLI — user runs a command, work happens,
user sees results. This works for development but breaks for:
1. **Connectors that need periodic sync** — GitHub issues don't wait
   for `patina scrape`. Email arrives continuously.
2. **Belief streams** — when a belief changes in persona A, persona B
   should learn about it without persona B running a command.
3. **Edge apps** — a chat agent on Cloudflare needs to reach Mother
   for fresh beliefs. If Mother isn't running, the agent is stale.
4. **Lake freshness** — data lakes need sync schedules. "Run scrape
   when you remember" isn't an architecture.

## Target State

Mother daemon runs continuously (launchd on macOS, systemd on Linux):
- Starts on login (or explicit `patina mother start`)
- Survives terminal close
- Manages connector schedules (configurable per connector)
- Delivers belief streams between linked personas
- Provides health endpoint for edge apps

```
MOTHER DAEMON (always running)
  ├── Connector scheduler
  │   ├── forge-connector: every 15 min
  │   ├── email-connector: every 5 min
  │   └── custom: per config
  ├── Belief stream router
  │   ├── persona A → persona B (push, scoped to facet:architecture)
  │   └── persona B → persona A (pull, all beliefs)
  ├── Lake sync manager
  │   └── tracks freshness, triggers connector on stale
  └── Health / API endpoint
      └── edge apps query for fresh beliefs
```

## Steps

1. **Prerequisite:** [[data-architecture-v3]] (lake registry),
   [[persona-federation]] (belief streams need persona linking)
2. Implement Mother daemon auto-start (launchd plist / systemd unit)
3. Add connector schedule config to Mother (per-connector intervals)
4. Implement scheduled tick() dispatch to connector plugins
5. Implement belief stream delivery (watch belief files via fs events,
   push to linked personas)
6. Add health/status endpoint (local socket or HTTP for edge apps)
7. Replace forge `libc::fork()` background sync with daemon-managed sync

## Design Decisions (resolved in DESIGN.md)

- **Stay threaded, no async.** The daemon is explicitly blocking/threaded
  today — conscious choice, not accident. Add a scheduler thread with
  a priority queue of (next_tick, child) pairs. Thread pool for
  connector execution. Migrate to hybrid (tokio for scheduler only) if
  concurrency demands exceed ~10 connectors. Don't rewrite the HTTP
  server — it works.

- **Explicit start, then launchd/systemd.** `patina mother start` /
  `patina mother stop` first. Then `patina mother install` writes a
  launchd plist (macOS) or systemd user unit (Linux) from templates in
  `resources/`. The daemon binary is the same either way. PID file
  infrastructure already exists.

- **Belief stream trigger: event-driven.** Watch for `belief.created` /
  `belief.evolved` events in events.db. Mother filters to belief-related
  event types, triggers stream delivery. This is the cleanest approach —
  the eventlog already records belief lifecycle. Polling is the fallback
  if event watching proves too complex initially.

- **Forge migration: daemon-first, fork-fallback.** If Mother daemon
  is running, delegate to it. If not, fall back to `libc::fork()`
  pattern. Check via PID file or socket probe. Remove fork in a future
  version once daemon usage is established.

- **Resource management: keep loaded, unload as escape valve.** For
  3-5 connectors, keep them in memory. The `on_load()`/`on_unload()`
  lifecycle already supports demand loading if needed at scale.

- **Offline: catch up naturally.** Connectors fetch since last_sync
  (gap fills automatically). Belief streams detect gaps via sequence
  numbers. Don't build a durable queue until there's evidence it's
  needed.

- **Belief stream delivery: emit event to project eventlog.** Receiving
  a belief from another persona IS an event — provenance=external,
  source=originating persona. Aligns with [[spec-data-architecture-v3]]
  provenance model.

- **Edge protocol: defer to own spec.** Continuous-operation builds the
  daemon. Edge connectivity (cloudflared tunnel, push-to-R2, webhooks)
  is a separate concern with enough design surface for its own spec.

## Open Questions

- **Scheduler architecture detail.** Priority queue with thread pool
  is the direction. Specific question: does one slow connector block
  the next tick? **Lean toward: no. Pool threads execute concurrently.
  Scheduler picks what's ready, pool runs it.**

- **Daemon upgrade path.** When user installs a new Patina version,
  running daemon is old binary. **Lean toward: manual
  `patina mother restart`.** Simple and predictable.

## Non-Goals

- **Edge deployment infrastructure.** How to deploy Patina apps to
  Cloudflare is a future spec. This spec makes Mother available for
  edge apps to connect to.
- **E2EE on streams.** Future work per [[content-addressed-references]].
  Streams should be designed to allow encryption later.
- **GUI/dashboard.** Mother daemon is headless. Monitoring via CLI
  (`patina mother status`).
- **Windows support.** Mac/Linux first per [[local-first-edge-deployable]].
