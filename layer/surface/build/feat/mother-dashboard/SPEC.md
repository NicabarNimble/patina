---
type: feat
id: mother-dashboard
status: design
created: 2026-02-09
sessions:
  origin: 20260209-215657
related:
  - layer/surface/build/feat/mother/SPEC.md
  - layer/surface/build/feat/mother-environment/SPEC.md
  - layer/surface/build/feat/mother-repos/SPEC.md
  - layer/surface/build/feat/mother-beliefs/SPEC.md
beliefs:
  - mother-is-the-daemon
  - four-layer-architecture
---

# feat: Mother Dashboard — Observability & Patrol

> Mother daemon exposes a web dashboard showing the state of the knowledge layer:
> index freshness, belief health, ref repo sync status, model info, and
> cross-project patterns. The daemon heartbeat drives periodic health checks
> (the "deacon patrol" pattern from gastown).

## Problem

### Mother Is Invisible

```
$ patina mother status

Mother daemon: running
   PID: 22096
   Socket: /Users/nicabar/.patina/run/serve.sock
   Version: 0.15.1
   Uptime: 13547s
```

That's everything Mother reports about itself. A daemon running 24/7 that
can't tell you:

- Are any ref repo indexes stale?
- When was the last scrape across all projects?
- How many beliefs exist across all projects?
- What model is loaded and what vector dimension?
- Are there embedding space mismatches between projects?
- What's the graph coverage (edges vs potential edges)?

### No Heartbeat, No Patrol

The daemon accepts connections and responds to requests, but it does nothing
proactively. Gastown's daemon runs a 3-minute heartbeat that checks agent
health, detects stuck sessions, restarts dead processes. Patina's daemon
is purely reactive — it waits.

Stale repos (470 commits behind), outdated indexes, disconnected graph nodes —
all invisible until someone manually checks.

### CLI-Only Interaction

All Mother interaction is CLI-based. For a system that's "always running,"
there's no persistent view. A web dashboard (like gastown's) would show
real-time state without needing to run commands.

## Current State

Mother daemon serves these endpoints:
- `GET /health` — status, version, uptime
- `GET /version` — name and version
- `POST /api/scry` — semantic search (unused — MCP bypasses)
- `GET /secrets/cache` — cached secrets
- `POST /secrets/cache` — store secrets
- `POST /secrets/lock` — clear secrets cache

No dashboard. No heartbeat. No proactive checks.

## Solution

### 1. Enhanced `patina mother status`

Before building a web dashboard, make the CLI status useful:

```
$ patina mother status

Mother daemon: running (PID 22096)
  Uptime: 3.7h | Version: 0.15.2
  Model: e5-base-v2@onnx (768d, warm)
  Secrets: cached (expires in 4m)

Knowledge:
  Beliefs: 87 project + 4 user = 91 total
  Repos: 25 registered (18 fresh, 5 stale, 2 unindexed)
  Graph: 31 nodes, 3 edges

Stale repos:
  ⚠ steveyegge/gastown       470 commits behind
  ⚠ openai/codex              12 commits behind
  ⚠ zed-industries/zed       203 commits behind
```

This requires Mother to query:
- Model state from `~/.patina/cache/models/`
- Secrets cache state from its own in-memory cache
- Belief count from `beliefs.db` (depends on [[mother-beliefs]])
- Repo freshness from registry + git (depends on [[mother-repos]])
- Graph state from `graph.db`

### 2. Daemon Heartbeat

Add a configurable heartbeat loop to the daemon (default: every 30 minutes):

```
Heartbeat cycle:
  1. Check ref repo freshness (git fetch --dry-run)
  2. Check index staleness (last_indexed vs HEAD)
  3. Check model compatibility (meta.json vs current spec)
  4. Log findings to ~/.patina/mother/heartbeat.log
  5. Optionally: auto-pull and re-index stale repos (if configured)
```

Not AI-powered (no deacon agent yet) — just deterministic checks that
the daemon runs on a timer.

### 3. Web Dashboard

`GET /dashboard` serves an HTML page showing:

```
┌─────────────────────────────────────────────────────────┐
│  MOTHER DASHBOARD                              v0.16.0  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  MODEL          e5-base-v2@onnx  768d  warm  ✓         │
│  UPTIME         3h 42m                                  │
│  SECRETS        cached (4m remaining)                   │
│                                                         │
├─────────────────────────────────────────────────────────┤
│  REPOS (25)                                             │
│  ✓ 18 fresh  ⚠ 5 stale  ✗ 2 unindexed                 │
│                                                         │
│  Stale:                                                 │
│  steveyegge/gastown      470 behind    never indexed    │
│  openai/codex             12 behind    3d ago           │
│  ...                                                    │
│                                                         │
├─────────────────────────────────────────────────────────┤
│  BELIEFS (91)                                           │
│  87 project  4 user  0 ref-extracted                    │
│                                                         │
│  Recent:                                                │
│  four-layer-architecture    2h ago    high confidence    │
│  corpus-composition...      1d ago    medium             │
│                                                         │
├─────────────────────────────────────────────────────────┤
│  GRAPH                                                  │
│  31 nodes  3 edges  (97% disconnected)                  │
│                                                         │
│  patina ─LEARNS_FROM→ USearch                           │
│  patina ─TESTS_WITH→ dojo                               │
│  patina ─LEARNS_FROM→ opencode                          │
│                                                         │
├─────────────────────────────────────────────────────────┤
│  HEARTBEAT                                              │
│  Last: 12m ago  Next: 18m  Cycle: 47                   │
│  Issues: 5 stale repos, 2 unindexed repos              │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

Self-contained HTML (no external dependencies). Auto-refreshes. Served on the
existing UDS socket (same transport as everything else) or opt-in TCP.

### 4. Heartbeat Log

```
~/.patina/mother/heartbeat.log

[2026-02-09T23:30:00Z] cycle=47 repos_fresh=18 repos_stale=5 repos_unindexed=2 beliefs=91 graph_nodes=31 graph_edges=3
[2026-02-09T23:00:00Z] cycle=46 repos_fresh=18 repos_stale=4 repos_unindexed=2 beliefs=91 graph_nodes=31 graph_edges=3
```

Structured, append-only, rotated by size. Provides historical view of
knowledge layer health.

## Acceptance Criteria

1. [ ] `patina mother status` shows model, secrets, beliefs, repos, graph summary
2. [ ] Daemon runs heartbeat loop (configurable interval, default 30m)
3. [ ] Heartbeat checks: repo freshness, index staleness, model compatibility
4. [ ] Heartbeat results logged to `~/.patina/mother/heartbeat.log`
5. [ ] `GET /dashboard` serves self-contained HTML dashboard
6. [ ] Dashboard shows: model, repos, beliefs, graph, heartbeat state
7. [ ] Dashboard auto-refreshes (meta refresh or simple JS polling)

## Non-Goals

- AI-powered patrol agent (gastown's deacon) — deterministic checks first
- Real-time WebSocket updates — polling/refresh is sufficient
- Mobile-responsive design — developer tool, desktop browser only
- Authentication for dashboard — UDS provides access control via file permissions
- Alerting/notifications — log it, don't push it (for now)

## Phasing

Can be built incrementally:

1. **Enhanced CLI status** — no new daemon code, just read existing state
2. **Heartbeat loop** — timer in daemon, deterministic checks
3. **Web dashboard** — HTML endpoint on existing server

Each phase is independently useful.
