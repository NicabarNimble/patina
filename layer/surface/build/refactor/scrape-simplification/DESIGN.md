# Design: Scrape Simplification — Local Capture Only

## Why Scrape Changes

Scrape today has a split personality. `execute_all()` is already clean —
it runs git, code, layer, beliefs. All local. No external API calls.
But `patina scrape forge` is a separate subcommand that shells out to
`gh`, manages background processes via `libc::fork()`, and writes forge
events to events.db. It's 604 LOC of connector logic living under the
scrape umbrella.

[[scrape-is-local-capture]]: "Scrape reads what's inside the project
(git). External data comes through connectors independently." The
architectural line is clear. Scrape = capture + index from local sources.
Connectors = capture from external sources. Both write to events.db.
Both feed beliefs. But they're separate operations with separate
lifecycles.

**Origin:** [[session-20260303-190855]] ("scrape becomes local capture
from git ONLY"), [[session-20260304-120702]] (decomposed scrape by
protocol verb).

## What Scrape Does Today

`execute_all()` (`src/commands/scrape/mod.rs:76-154`) is delta-driven:

```
compute_delta() → classify changed files → dispatch only to scrapers with work

  git::run()          — new commits since last scrape
  code (grammars)     — changed code files, filtered by extension
  layer::run()        — changed layer/ files
  beliefs::run()      — regrounding if code or beliefs changed
```

Delta computation (`delta.rs`) checks git log and working tree. No
forge source-kind exists in the delta system — forge is not part of
the main scrape pipeline.

**Forge is already architecturally separate.** `execute_forge()` is a
distinct subcommand handler at `mod.rs:354`. It has its own flags
(--full, --status, --sync, --log, --limit, --repo), its own background
process model (`libc::fork()`), and its own rate limiting (750ms). It
doesn't participate in delta-driven dispatch.

The one exception: `execute_rebuild()` calls forge as step [5/5] for
ref repos. This is the only coupling point.

## What Changes

### Removal (after forge-plugin-extraction)

Once [[spec-forge-plugin-extraction]] moves forge to a connector plugin:

1. Delete `src/commands/scrape/forge/mod.rs` (604 LOC)
2. Remove `pub mod forge` from `src/commands/scrape/mod.rs`
3. Remove forge step from `execute_rebuild()` (lines 211-218)
4. Remove forge subcommand from clap CLI definition
5. Remove forge helper functions (`get_repo_spec`, `get_db_path`,
   `execute_forge_status`, `execute_forge_background`,
   `execute_forge_limited`, `execute_forge_log`) — all in mod.rs

`execute_all()` needs zero changes. It already doesn't call forge.

### The UX Transition

This is the critical question: what replaces `patina scrape forge`?

**Today's UX surface:**
- `patina scrape forge` — foreground discovery + sync
- `patina scrape forge --sync` — fork to background, PID tracking
- `patina scrape forge --status` — check background sync progress
- `patina scrape forge --log` — tail background sync log
- `patina scrape forge --limit N` — foreground sync with cap
- `patina scrape forge --repo NAME` — sync a ref repo's forge

**After extraction, three paths exist:**

**Path 1: `patina mother run <name>` (explicit)**

Mother runs connector children via pipe protocol. This is the direct
replacement for `patina scrape forge`:

```
patina mother run github           # run github connector child
patina mother status               # show running children and health
```

Implementation: `patina mother run github` spawns the github-connector
child (native binary speaking pipe protocol over stdio), sends
pipe/initialize with credentials, dispatches pipe/fetch. Facts stream
back as pipe/fact notifications and are routed to project events.db.
See [[spec-mother-broker]].

**Path 2: Mother daemon (continuous)**

[[spec-continuous-operation]] handles this. Mother ticks forge-connector
on schedule (e.g., every 15 minutes). No user command needed — data
flows automatically. This is the target state but depends on Dimensions
1-3 of mother-maturation.

**Path 3: `patina scrape --sync` (convenience, optional)**

Sugar for "scrape local + sync connectors" in one shot. Eases
transition for users who expect one command to do everything:

```
patina scrape          # local only (git, code, layer, beliefs)
patina scrape --sync   # local + run all connectors
```

This is syntactic sugar — it calls `execute_all()` then triggers
connector sync. It does NOT couple scrape to connectors architecturally.
Scrape remains local-only; the flag triggers a separate operation.

### The Transition Period

Between forge extraction and continuous operation, there's a gap where
the daemon isn't running yet. During this period:

- `patina mother run github` is the primary path
- `patina scrape` continues to work for all local sources
- The `--sync` convenience flag bridges the UX gap
- `patina scrape forge` shows a deprecation message pointing to
  `patina mother run`

This transition is bounded — it ends when continuous-operation lands
and Mother runs connectors automatically.

## Design Decisions

### 1. Layer/ Parsing — Protocol, Not Domain

SPEC.md asks: "Is layer/ parsing protocol or domain?"

**Decision: Protocol.** layer/ is Patina's own format. Every Patina
project has a `layer/` directory. Parsing markdown with YAML frontmatter
is reading the declaration store. It's like git parsing `.git/` — the
format is the tool's own, not a domain-specific document format.

Grammar plugins parse domain-specific formats (Rust, Python, legal
documents). The layer scraper parses Patina's own format. Different
category entirely.

### 2. Grammar Dispatch — Stays in Scrape

SPEC.md asks: "Should grammar plugins continue to be dispatched by
scrape?"

**Decision: Yes.** Grammar plugins parse local files. They're invoked
during the index phase of scrape — code changed, load grammar plugins
for the relevant extensions, process the files. This is local-only
work: read file from disk, parse AST, emit structured facts.

[[scrape-is-local-capture]] distinguishes local capture (scrape) from
external capture (connectors). Grammar plugins are local. They stay.

The pipeline world already handles grammar dispatch with lazy plugin
loading (only load plugins claiming changed extensions). The delta
system feeds this efficiently. No changes needed.

### 3. Connector Freshness — Connector-Internal

SPEC.md asks about delta system for connectors.

**Decision: Connector-internal.** Scrape's delta system tracks git
commits and file modification times — local filesystem state.
Connectors track API-level freshness: last sync timestamp, pagination
cursors, incremental fetch markers. These are fundamentally different
tracking mechanisms.

Each connector plugin manages its own freshness. The forge connector
already tracks this (it knows which refs are resolved vs pending).
As a plugin, it stores cursor state via host calls or its own data.
Mother's lake registry (from [[spec-data-architecture-v3]]) provides
metadata freshness at the Mother level.

### 4. Rebuild Without Forge

`execute_rebuild()` currently calls forge as step [5/5] for ref repos.
After extraction:

- Rebuild deletes patina.db and regenerates from git + events
- Forge events are already IN events.db (they were emitted by the
  connector). They survive rebuild — they're external-provenance events
  per [[spec-data-architecture-v3]]
- Materialized views (forge_issues, forge_prs) are rebuilt from events.db
  by the projection system
- No re-fetch needed. The rebuild reconstructs projections from existing
  events.

The rebuild step that calls `execute_forge(true, ...)` becomes
unnecessary. External evidence is preserved in events.db. Projections
are rebuilt from events. The connector only needs to run when you want
NEW data from GitHub — not when you're rebuilding projections.

## Key Files

**Scrape pipeline (what stays):**
- `src/commands/scrape/mod.rs` — `execute_all()` (lines 76-154, unchanged)
- `src/commands/scrape/delta.rs` — delta computation (no forge, unchanged)
- `src/commands/scrape/git/` — git commit scraper
- `src/commands/scrape/layer/` — layer/ file scraper
- `src/commands/scrape/beliefs/` — belief regrounding
- `src/commands/scrape/code/` — grammar plugin dispatch

**Forge (removed after extraction):**
- `src/commands/scrape/forge/mod.rs` (604 LOC) — forge subcommand
- `src/commands/scrape/mod.rs` lines 354-601 — forge functions
- `pub mod forge` declaration in mod.rs

**New (Mother broker):**
- `src/broker/mod.rs` — Mother routing engine ([[spec-mother-broker]])
- `patina mother run/status` — connector execution via Mother

**Plugin infrastructure (already exists):**
- `src/commands/plugin.rs` — `patina plugin list`, `patina plugin run`
- `src/plugin/internal/mod.rs` — PluginManifest, PluginEngine

## Resolved Questions

1. **Connector CLI naming.** Resolved in [[spec-pipe-architecture]]
   sessions 9-10: `patina mother run <name>` (not `patina connector`).
   Mother is the broker — she manages and runs children. No separate
   connector command. Users talk to Mother or set up connections.

2. **Scope of connector runs.** Resolved: project-scoped via
   sources.toml declarations. Mother reads sources.toml from registered
   projects, runs only configured connectors. See [[spec-mother-broker]].
