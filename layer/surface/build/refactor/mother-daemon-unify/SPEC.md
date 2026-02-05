---
type: refactor
id: mother-daemon-unify
status: draft
created: 2026-02-04
sessions:
  origin: 20260204-193822
related:
  - src/commands/serve/
  - src/commands/mother.rs
  - src/commands/launch/internal.rs
  - src/mother/
beliefs:
  - mother-is-the-daemon
  - patina-is-knowledge-layer
  - transport-security-by-trust-boundary
upstream:
  - refactor/mother-naming
  - refactor/security-hardening
---

# refactor: Unify serve under mother

> `patina serve` and `patina mother` are the same thing seen from different angles. Merge them.

## Problem

Two commands, one concept:

| Command | What it does | What it should be |
|---------|-------------|-------------------|
| `patina serve` | Starts the daemon (UDS/TCP, health, scry, context, secrets, MCP) | `patina mother start` |
| `patina mother` | Graph CRUD (sync, link, unlink, stats) | `patina mother graph` |

The `serve` help text already says "Start the **Mother** daemon." The launcher calls `start_mother_daemon()` which spawns `patina serve`. The naming is confused because they grew independently.

**Precedent:** Docker (`docker` is CLI + daemon management), Ollama (`ollama serve` starts daemon, all other commands talk to it).

## Design

### New command tree

```
patina mother                    # Show daemon status (running? pid? uptime? projects?)
patina mother start              # Start daemon (what 'serve' does today)
  --host <HOST>                  # Opt-in TCP (default: UDS only)
  --port <PORT>                  # TCP port (default: 50051)
  --mcp                          # MCP mode (JSON-RPC over stdio)
  --foreground                   # Don't daemonize (for debugging)
patina mother stop               # Graceful shutdown
patina mother status             # Health, uptime, connected projects, cached models
patina mother logs               # Tail daemon logs (if we add logging)
patina mother graph              # Current 'mother' subcommand
patina mother graph sync
patina mother graph link <a> <b>
patina mother graph unlink <a> <b>
patina mother graph stats
patina mother graph learn
```

### What happens to `serve`?

Option A: **Alias** — `patina serve` becomes a hidden alias for `patina mother start` (backward compat for MCP configs that shell out to `patina serve --mcp`)

Option B: **Remove** — break the old command, update MCP configs during `patina adapter refresh`

Recommend **Option A** — MCP server configs reference `patina serve --mcp` and breaking those silently is worse than keeping a hidden alias.

**Deprecation plan:** The alias must have a removal date. Track via GitHub issue.
1. v0.11.x: `patina serve` works but prints deprecation warning to stderr on first use
2. v0.12.0 (or v1.0): remove the alias, `patina serve` → "unknown command, did you mean `patina mother start`?"
3. `patina adapter refresh` rewrites MCP configs from `serve --mcp` to `mother start --mcp`

Tracked: [#85](https://github.com/NicabarNimble/patina/issues/85)

### What happens to bare `patina mother`?

Today it shows the graph subcommand help. After unification, bare `patina mother` should show **daemon status** — is it running, PID, uptime, how many projects connected, model cache size. This is the Docker `docker info` / Ollama `ollama ps` equivalent.

### Launcher update

`start_mother_daemon()` in `src/commands/launch/internal.rs` currently spawns `patina serve`. After unification it spawns `patina mother start`. The health check (already fixed to UDS in this session) stays the same.

## Implementation

### Phase 1: Command restructure

1. Move `src/commands/serve/` logic into `src/commands/mother/`
2. Add `MotherCommands` enum: `Start`, `Stop`, `Status`, `Graph { subcommand }`
3. Current `src/commands/mother.rs` graph logic becomes `MotherCommands::Graph`
4. Keep `patina serve` as hidden alias (`#[command(hide = true)]`)

### Phase 2: Daemon lifecycle

1. `patina mother start` — same as current `serve`, but writes PID file to `~/.patina/run/serve.pid`
2. `patina mother stop` — read PID, send SIGTERM, wait, cleanup socket
3. `patina mother status` — check socket, show health info, project count, model cache

### Phase 3: Launcher integration

1. `start_mother_daemon()` spawns `patina mother start`
2. `check_mother_health()` unchanged (already UDS)
3. Bare `patina` auto-starts mother via `patina mother start` (current behavior, new command)

## Exit Criteria

- [ ] `patina mother start` starts the daemon (UDS default, TCP opt-in)
- [ ] `patina mother stop` gracefully shuts down
- [ ] `patina mother status` shows running state
- [ ] `patina mother` (bare) shows daemon status
- [ ] `patina mother graph` shows graph subcommands
- [ ] `patina serve` still works as hidden alias
- [ ] `patina serve --mcp` still works (MCP configs unbroken)
- [ ] Launcher uses `patina mother start` internally
- [ ] Health check still works via UDS

## Open Questions

1. **Logging** — should mother write logs? Where? `~/.patina/run/mother.log`? Or rely on journald/launchd?
2. **launchd plist** — should `patina mother start` offer to install a launchd plist for auto-start on Mac?
3. **Version system** — discussed separately. Version is the odd one out in patina's command set. May become a plugin or get simplified. Not part of this spec.
4. **Platform vision** — mother-as-daemon opens the door to a plugin/module system where version, spec, etc. are modules that register commands and MCP tools. Out of scope here but this refactor is a prerequisite.
