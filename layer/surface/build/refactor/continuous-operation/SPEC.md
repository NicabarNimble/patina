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

## Exploration Needed (SIGNIFICANT)

- **Daemon architecture.** launchd on macOS, systemd on Linux. Or a
  cross-platform approach (Rust background process with PID file)?
  The forge sync already does PID-based background management. Could
  Mother daemon generalize that pattern. **Needs investigation of
  each platform's daemon story.**

- **Edge app protocol.** How do edge apps (Cloudflare Workers) talk
  to local Mother? Options: WebSocket tunnel, HTTP API with ngrok/
  cloudflared, push to R2/D1 that edge reads. Each has different
  latency and complexity. **This is the edge interface design — major
  exploration needed. Could be its own spec.**

- **Belief stream semantics.** What triggers a stream delivery? File
  change in git (fs watcher on `layer/surface/epistemic/`)? Event
  in events.db (belief.created, belief.evolved)? Git commit hook?
  **Needs design. fs watcher is simplest but may miss git operations.**

- **Resource management.** Mother daemon running continuously means
  CPU/memory budget. Connector plugins loaded into memory. How many
  can run simultaneously? Should idle connectors be unloaded?
  **Lean toward: load on schedule, unload after sync. Keep memory
  footprint minimal.**

- **Offline resilience.** What happens when Mother daemon is down?
  Projects must still function (sovereignty principle). Connector
  data is stale but available. Belief streams queue and deliver on
  restart. **Design the degraded-operation story.**

- **Streaming data patterns.** User has crypto/streaming background
  and is interested in real-time data flows. Mother's continuous
  operation could use event streaming patterns (pub/sub, reactive
  streams) rather than polling. **Explore: is Tokio + channels the
  right internal architecture? Or simpler timer-based polling?**

## Non-Goals

- **Edge deployment infrastructure.** How to deploy Patina apps to
  Cloudflare is a future spec. This spec makes Mother available for
  edge apps to connect to.
- **E2EE on streams.** Future work per [[content-addressed-references]].
  Streams should be designed to allow encryption later.
- **GUI/dashboard.** Mother daemon is headless. Monitoring via CLI
  (`patina mother status`).
- **Windows support.** Mac/Linux first per [[local-first-edge-deployable]].
