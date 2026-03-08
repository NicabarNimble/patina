# Design: Continuous Operation — From Batch CLI to Always-On Daemon

## Why This Spec Exists

This is Dimension 3 (Always On) of [[spec-mother-maturation]]. The
capstone. Dimensions 1 and 2 give Mother things to manage — lakes
(data-architecture-v3) and personas (persona-federation). This
dimension gives her LIFE. Without continuous operation, Mother is
a batch command: run, sync, exit. With it, she's the nervous system
— always running, always routing, always aware.

[[mother-is-connection-and-continuity]]: "at the Mother level,
connectors are pulling, lakes are syncing, belief streams are flowing.
Always on." The word "continuity" is in the belief name. It's not
an optional feature — it's half of Mother's identity.

[[patina-is-beliefs-plus-action]]: "Without action, beliefs are static
documents." Continuous operation is what makes Mother's action layer
real. Scheduled connector syncs, reactive belief streams, health
monitoring — these are the actions that keep the belief ecosystem
alive between CLI invocations.

**Origin:** [[session-20260304-120702]] ("the reality is this should be
continuous at the mother level"), [[session-20260303-190855]] ("it is
so important for patina to connect").

## What Exists Today

The daemon infrastructure is NOT greenfield. The mother-child world
exists and runs. The question is scope expansion — from "daemon that
loads children" to "daemon that IS the continuous process."

### Mother Daemon (`src/commands/mother/daemon.rs`, 760 LOC)

A working HTTP microserver with two transport modes:

| Feature | Implementation | Status |
|---------|---------------|--------|
| UDS transport | `~/.patina/run/serve.sock` (default) | Working |
| TCP transport | `--host/--port` with bearer token auth | Working |
| PID file | `~/.patina/run/serve.pid`, SIGINT/SIGTERM cleanup | Working |
| Child registry | `ChildRegistry` — register, load_all, route | Working |
| Heartbeat | 60s `tick_all()` → toys → `spawn_toy_tracked()` | Working |
| Health endpoint | `GET /health` — uptime, version, child health | Working |
| Scry endpoint | `POST /api/scry` — federated search via daemon | Working |
| Child routing | `GET/POST /child/{name}/{action}` — generic | Working |
| Secrets child | Compiled-in, caches secrets in daemon memory | Working |
| WASM children | Discovered from `~/.patina/children/*.wasm` | Working |

**Key architectural observations:**

The daemon is **blocking, not async.** `daemon.rs:8` explicitly states
"Design: Blocking HTTP microserver (no async/tokio)." Each connection
gets a thread. The heartbeat runs in its own thread. Toy execution
gets per-toy threads. This is a conscious choice — no tokio runtime,
no async complexity, just threads and blocking I/O.

The `ChildRegistry` (`src/commands/mother/registry.rs`) wraps children
in `Arc<RwLock<Box<dyn MotherChild>>>`. `handle()` takes read locks
(concurrent). `tick()` and `on_load()`/`on_unload()` take write locks
(sequential in heartbeat loop).

The `MotherChild` trait (`src/mother/child.rs`, 149 LOC) defines the
child interface:
- `name()` — routing identity
- `on_load(host)` / `on_unload()` — lifecycle
- `health()` → `ChildHealth` — Healthy/Degraded/Unhealthy
- `handle(request)` → `ChildResponse` — request routing
- `tick()` → `Vec<Toy>` — heartbeat work requests

Children never spawn processes directly — they return `Toy` structs
from `tick()` and Mother spawns/monitors them. This is the security
boundary: children declare WHAT to run, Mother controls HOW.

### WASM Child Adapter (`src/plugin/internal/mother_child.rs`, 532 LOC)

The bridge between WIT world and native Rust trait. `WasmChild` wraps
a `Store<HostState>` + `bindings::MotherChild` behind a `Mutex`. The
WIT world (`wit/mother-child/mother-child.wit`) imports:
- `patina:host/log` — structured logging
- `patina:host/layer` — read-only project data
- `patina:host/query` — capability-gated search (scry, context, assay)
- `patina:host/http` — domain-allowlisted HTTP
- `patina:host/measure` — measurement reporting

And exports the child interface: `init`, `name`, `on-load`,
`on-unload`, `health`, `handle`, `tick`.

Capability enforcement happens at two levels:
1. **Load-time:** `check_capabilities()` validates manifest against
   world-allowed capabilities
2. **Call-time:** `GrantedCapabilities` gates each host function

### Forge Background Sync (`src/forge/sync/internal.rs`, 616 LOC)

The pattern continuous-operation replaces. Currently uses
`libc::fork()` to background sync:

```rust
match unsafe { libc::fork() } {
    0 => {
        // Child: setsid(), redirect stdout/stderr to log, sync, exit
        unsafe { libc::setsid() };
        // ... drain_forge() ...
        std::process::exit(0);
    }
    child_pid => Ok(child_pid as u32),
}
```

This has multiple problems:
- **Unix-only** — `#[cfg(not(unix))]` returns an error
- **No lifecycle management** — forked process is fire-and-forget
- **PID file collision risk** — `pid_file_path()` per-repo, but no
  coordination with Mother daemon's PID
- **No schedule** — sync only happens when user runs `patina scrape
  forge --background`
- **No health reporting** — daemon can't observe forge sync status

The continuous-operation model replaces this with daemon-managed
connector scheduling. The forge connector becomes a child (or a
connector plugin managed by Mother) that ticks on schedule instead
of forking on demand.

## What Changes

### From Thread-per-Connection to Scheduled Work

The current daemon model is reactive: listen for HTTP requests,
dispatch to children. Continuous operation adds a proactive dimension:

```
CURRENT:
  User/CLI → HTTP request → daemon → route to child → response

FUTURE (adds):
  Timer → scheduler → tick connector → emit facts → route to projects
  Belief change → watcher → stream router → push to linked personas
  Stale lake → scheduler → trigger connector → refresh lake
```

The heartbeat loop (`spawn_heartbeat`, 60s interval) is the seed.
Today it ticks all children and spawns their toys. Continuous
operation extends this into a scheduler with per-child/per-connector
intervals.

### Connector Scheduling

Mother gains a schedule config — which connectors run and how often:

```toml
[connectors.forge]
interval = "15m"
plugin = "forge-connector"   # or built-in

[connectors.email]
interval = "5m"
plugin = "email-connector"
```

The scheduler replaces the forge `libc::fork()` pattern. Instead of:
1. User runs `patina scrape forge --background`
2. Process forks, syncs, exits

It becomes:
1. Mother daemon starts, loads connector schedule
2. Every 15 minutes, Mother ticks the forge connector
3. Connector syncs, emits facts via `host_emit`
4. Mother routes facts to relevant projects

### Belief Stream Delivery

When a belief changes in persona A, Mother delivers it to linked
personas per the `persona_links` routing table
([[spec-persona-federation]] DESIGN.md):

```
Persona A project → belief file changes (git commit)
   → Mother detects change (mechanism TBD — see Open Questions)
   → Mother looks up persona_links: A → B (push, scope: architecture)
   → Mother delivers belief update to persona B's projects
   → Persona B's project receives belief as external evidence
```

The delivery mechanism builds on the existing heartbeat. A "belief
stream child" (compiled-in or plugin) watches for belief changes
and pushes updates through Mother's routing table.

### Health and Status Expansion

The existing `GET /health` endpoint returns uptime + child health.
Continuous operation adds:

- Connector sync status (last run, next run, success/failure)
- Lake freshness (last sync, staleness)
- Belief stream status (queued, delivered, failed)
- Persona health (linked, reachable, last sync)

`patina mother status` becomes a comprehensive dashboard — not just
"daemon is running" but "here's everything Mother knows about the
state of the system."

## Design Decisions

### 1. Async or Threads?

The daemon is explicitly blocking/threaded today. Continuous operation
adds scheduled work, multiple timers, and potentially fs watching.
Does this push toward async/tokio?

**Option A: Stay threaded.** Add a scheduler thread. Use
`std::thread::sleep` loops for timers. One thread per scheduled
connector. Simple, matches existing architecture.

**Option B: Introduce tokio.** Async timers, spawn tasks, channels
for inter-component communication. Better for many concurrent
tasks.

**Option C: Hybrid.** Keep the HTTP server threaded (it works).
Add a separate tokio runtime for the scheduler and watchers.

**Lean toward A initially, migrate to C if needed.** The current
thread model works for what exists. A scheduler thread with a
priority queue of "next tick" times is simple and predictable.
If the number of concurrent connectors grows past ~10, or if
belief stream delivery needs non-blocking I/O, consider tokio
for the scheduler only. Don't rewrite the HTTP server — it's
fine.

The key insight: the daemon's concurrency model is already
thread-per-connection with a heartbeat thread. Adding a scheduler
thread is incremental, not architectural. Going full-async is a
rewrite that buys nothing until we hit scale problems.

### 2. Daemon Lifecycle — How Does Mother Start?

**Option A: Explicit start.** `patina mother start` / `patina
mother stop`. User manages lifecycle.

**Option B: Auto-start on login.** launchd plist (macOS) / systemd
unit (Linux). Mother runs when the machine runs.

**Option C: Start-on-first-use.** First `patina` command in any
project checks if Mother is running. If not, starts her in
background.

**Lean toward A first, then B.** Explicit start is simplest to
implement and debug. Auto-start via launchd/systemd is the target
for "always on" — but it's a deployment concern, not an architecture
concern. The daemon binary is the same either way. Ship A, provide
a `patina mother install` that writes the launchd plist.

The PID file infrastructure already exists (`write_pid_file()`,
`cleanup_pid_file()`, SIGINT/SIGTERM handlers). The daemon already
does graceful shutdown with socket cleanup. The lifecycle management
is mostly there — just needs a `start`/`stop`/`status` CLI wrapper
and a launchd plist generator.

### 3. Belief Stream Trigger — What Detects Changes?

**Option A: Filesystem watcher.** Watch `layer/surface/epistemic/`
for file changes. Immediate detection. But may miss git operations
(checkout, rebase) that modify files without editor saves.

**Option B: Git hook.** Post-commit hook notifies Mother when
beliefs change. Catches all git-mediated changes. But requires hook
installation and doesn't catch direct file edits.

**Option C: Event-driven.** `belief.created` / `belief.evolved`
events in events.db. Mother watches the eventlog for belief-related
events. Catches everything that goes through `patina scrape`.

**Option D: Polling.** Mother periodically checks belief file
modification times or git log for recent belief commits.

**Lean toward C, fall back to D.** Event-driven is cleanest — the
eventlog already records belief lifecycle events. Mother watches
for new events, filters to belief-related types, and triggers stream
delivery. This is the "belief streams flow from evidence" model that
[[patina-is-beliefs-plus-action]] implies. Polling is the fallback
if event watching proves too complex initially.

### 4. Forge Sync Migration Path

The `libc::fork()` pattern in `src/forge/sync/internal.rs` needs a
deprecation path. It can't disappear overnight — users may not run
the daemon.

**Option A: Daemon-first, fork-fallback.** If Mother daemon is
running, `patina scrape forge` delegates to the daemon's scheduler.
If not, falls back to the fork pattern.

**Option B: Remove fork immediately.** Require daemon for background
sync. `patina scrape forge --background` fails without daemon.

**Lean toward A.** Graceful migration. The daemon model is better
but the fork pattern works TODAY. Check for running daemon (via PID
file or socket probe), delegate if available, fork if not. Remove
fork in a future version once daemon usage is established.

### 5. Resource Management — Memory Budget

Mother daemon runs continuously. WASM children are loaded into
memory. Connectors might need heavy resources (HTTP clients,
embedder models).

**Option A: Keep everything loaded.** Simple. Memory cost is the
sum of all children. Acceptable for a handful of plugins.

**Option B: Load on schedule, unload after.** Connectors load when
their tick fires, unload when done. Saves memory, adds latency.

**Option C: LRU with memory budget.** Keep recently-used children
loaded. Evict idle ones. Complex.

**Lean toward A, with B as escape valve.** The secrets child is
trivial (in-memory HashMap). WASM children are the concern — each
wasmtime `Store` holds compiled code. For a user with 3-5 connectors,
keeping them loaded is fine. If someone has 20 connectors, B becomes
necessary. The `on_load()`/`on_unload()` lifecycle already supports
B — children designed for it can release resources in `on_unload()`
and reclaim in `on_load()`.

## Key Files

**Daemon (current implementation):**
- `src/commands/mother/daemon.rs` (760 LOC) — HTTP server, heartbeat,
  child loading, signal handling, PID management
- `src/commands/mother/registry.rs` — ChildRegistry, tick_all,
  health_all, handle routing
- `src/commands/mother/microserver.rs` — HTTP request/response parsing

**Child trait (the interface continuous-operation extends):**
- `src/mother/child.rs` (149 LOC) — MotherChild trait, ChildHealth,
  ChildRequest/Response, Toy, MotherHost
- `src/plugin/internal/mother_child.rs` (532 LOC) — WasmChild adapter,
  PluginEngine, bindgen

**Forge sync (the pattern being replaced):**
- `src/forge/sync/internal.rs` (616 LOC) — `libc::fork()` background
  sync, PID file infrastructure, drain_forge()
- `src/forge/sync/mod.rs` — sync entry points, SyncStats

**WIT world (child interface definition):**
- `wit/mother-child/mother-child.wit` — imports (log, layer, query,
  http, measure) and exports (init, name, on-load, health, handle, tick)

**Mother registries (scheduler config will live here):**
- `src/mother/graph.rs` (1,927 LOC) — graph.db schema and queries
- `~/.patina/registry.yaml` — project/repo registry

## Open Questions

1. **Scheduler architecture.** A priority queue of (next_tick, child)
   pairs, popped by a scheduler thread? Or per-connector timers? The
   priority queue is simpler to reason about and avoids thread
   proliferation, but means one slow connector blocks the next tick.
   **Lean toward: priority queue with a thread pool for execution.
   Scheduler picks what to run, pool threads do the work. Heartbeat
   thread becomes the scheduler thread.**

2. **Offline resilience.** What happens when Mother daemon is down?
   Projects must still function — that's the sovereignty principle.
   But what about queued belief streams? Connector sync gaps? Options:
   (a) queue to disk, replay on restart; (b) lose the window, catch
   up on next sync; (c) projects detect daemon-down and fall back to
   local operation. **Lean toward (b) initially.** Connectors catch
   up naturally (they fetch since last_sync). Belief streams can
   detect gaps via sequence numbers. Don't build a durable queue
   until there's evidence it's needed.

3. **Edge app protocol.** How do Cloudflare Workers reach Mother?
   The daemon listens on UDS (local) or TCP (opt-in). Edge apps need
   internet-reachable Mother. Options: reverse proxy (cloudflared
   tunnel), push-to-R2, webhook callbacks. This is the biggest
   unexplored territory — could be its own spec. **Lean toward:
   defer to a dedicated edge-interface spec. Continuous-operation
   builds the daemon. Edge connectivity builds on top.**

4. **Belief stream protocol.** When Mother delivers a belief update
   to a linked persona's project, what does the delivery look like?
   Options: (a) write a file to the project's inbox directory;
   (b) emit an event to the project's eventlog; (c) HTTP POST to
   a project-level daemon; (d) git operation (add belief file, commit).
   **Lean toward (b).** Events are the project's autobiography.
   Receiving a belief from another persona IS an event — provenance
   = external, source = the originating persona. This aligns with
   [[spec-data-architecture-v3]]'s provenance model.

5. **launchd/systemd generation.** `patina mother install` should
   generate platform-appropriate service definitions. On macOS, a
   LaunchAgent plist. On Linux, a systemd user unit. How much
   platform-specific code is acceptable? **Lean toward: template
   files in `resources/`, installed by `patina mother install` with
   path substitution. Keep the templates simple — just start the
   binary with the right flags.** The `resources/` directory already
   exists (e.g., `resources/git/pre-push-checks.sh`).

6. **Daemon upgrade path.** When the user installs a new Patina
   version, the running daemon is the old binary. Options:
   (a) `patina mother restart` — stop, start with new binary;
   (b) automatic detection — daemon checks its binary hash on health;
   (c) graceful reload — daemon re-execs itself. **Lean toward (a).**
   Manual restart is simple and predictable. Auto-detection is nice
   but adds complexity for a rare event (Patina upgrades).
