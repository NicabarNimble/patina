# Design: Data Architecture v2

## Approach

This is a vision spec. It doesn't ship code directly — it establishes the
architectural framing and spawns focused sub-specs that each ship independently.

The design work here is:
1. Get the three-system model right (events, projections, federation)
2. Map dependencies between implementation areas
3. Identify cross-cutting decisions that affect multiple sub-specs
4. Surface open questions before implementation begins

## Sub-Spec Dependency Graph

```
                    ┌─────────────────────┐
                    │  data-architecture  │
                    │  -v2 (this spec)    │
                    │  VISION / FRAMING   │
                    └─────────┬───────────┘
                              │
              ┌───────────────┼────────────────┐
              │               │                │
              ▼               ▼                ▼
   ┌──────────────┐  ┌───────────────┐  ┌─────────────┐
   │ Area 1:      │  │ Area 3:       │  │ Area 5:     │
   │ DB Split     │  │ Mother Schema │  │ Fast Incr.  │
   │              │  │ Alignment     │  │ + Hooks     │
   │ FOUNDATION   │  │               │  │             │
   │ (must be     │  │ (independent) │  │ (last —     │
   │  first)      │  │               │  │  perf opt)  │
   └──────┬───────┘  └───────────────┘  └─────────────┘
          │
          ▼
   ┌──────────────┐
   │ Area 2:      │
   │ Emission     │
   │ Completeness │
   │              │
   │ (needs       │
   │  events.db)  │
   └──────┬───────┘
          │
          ▼
   ┌──────────────┐
   │ Area 4:      │
   │ Measure as   │
   │ LLM Query    │
   │ Surface      │
   │              │
   │ (needs full  │
   │  event flow) │
   └──────────────┘
```

**Critical path:** Area 1 → Area 2 → Area 4

**Parallel-safe:** Area 3 (mother) can proceed alongside Areas 1-2.
Area 5 (performance) is independent and can happen anytime but benefits
from Areas 1-2 being stable.

## Sequencing Strategy

### Phase A: Foundation (Area 1)
Create events.db, rewire runtime writers, ATTACH for readers, migrate
existing events, update rebuild. This is the structural change everything
else depends on.

**Gate:** `rm patina.db && patina scrape` works correctly AND runtime
events survive the rebuild.

### Phase B: Fill the stream (Area 2) + Fix federation (Area 3)
These can happen in parallel:
- Area 2 wires emissions into events.db (the stream now exists)
- Area 3 aligns mother's schema with project beliefs (independent data path)

**Gate for Area 2:** `patina measure` shows capture events for all scrapers.
**Gate for Area 3:** `patina mother graph sync` syncs grounding + verification.

### Phase C: Surface (Area 4)
With events flowing and mother aligned, build the LLM query surface.
`patina measure --full` returns structured JSON covering all domains.

**Gate:** An LLM can call `patina measure --full` and answer "is this
project healthy?" with specific, data-backed claims.

### Phase D: Performance (Area 5)
Optimize incremental scrape, add git hooks. This is polish — the
architecture is correct by Phase C.

**Gate:** Incremental scrape after a commit completes in <2s.

## Cross-Cutting Design Decisions

### Event type registry

All sub-specs emit events. We need a consistent naming convention:

```
domain.action[.detail]

measure.capture          — scraper ran, here are metrics
measure.search           — eval/bench searched, here are results
measure.index            — oxidize indexed, here are stats
measure.believe          — belief audit ran
measure.evolve           — session lifecycle event
scry.query               — search executed
scry.use                 — result selected
scry.feedback            — explicit feedback given
forge.issue              — GitHub issue cached
forge.pr                 — GitHub PR cached
session.start            — session began
session.end              — session archived
audit.*                  — future audit trail
```

**Decision: Registry-first.** The SPEC event type table is the canonical
registry. New event types are added to the SPEC before code. This taxonomy
is locked before Area 2 begins — no ad-hoc event types in production.

### Event data schema conventions

Events carry a `data` JSON blob. These conventions are binding:

- **Duration fields:** Always in milliseconds (`duration_ms`)
- **Count fields:** Always integers (`items_processed`, `errors`)
- **Identifiers:** Use existing IDs where available (`session_id`, `belief_id`)
- **Tool identification:** `tool` field identifies the emitting command
- **Mode/variant:** `mode` field for sub-modes (e.g., `measure.capture` with `tool=scrape, mode=git`)

**Decision: Forward-compatible JSON, not formal per-type schemas.** The `data`
blob is flexible — new fields are additive, existing fields never removed or
renamed. Readers ignore unknown fields and default missing fields. This gives
us schema evolution without version negotiation. We don't need protobuf for
2,400 events/year in SQLite — we need discipline in the conventions above.

### ATTACH pattern

Multiple sub-specs will use SQLite ATTACH to read across databases:

```sql
ATTACH DATABASE 'events.db' AS events READONLY;
-- query: SELECT * FROM events.eventlog WHERE ...
-- joined with: SELECT * FROM beliefs WHERE ...
```

**Design constraint:** ATTACH is connection-scoped. Each command that needs
cross-db access must ATTACH at the start of its operation. This is fine for
CLI commands but needs thought for long-running MCP server connections.

**Decision needed:** Does the MCP server hold a persistent connection with
ATTACH, or ATTACH per-request?

### Migration strategy

Area 1 needs a one-time migration (copy runtime events from patina.db to
events.db). Conventions for future migrations:

- **Schema version tracking:** Where? In the DB itself (pragma user_version)?
  In a metadata table? In a file?
- **Forward-only:** Migrations go forward. No rollback. If something breaks,
  rebuild from sources + events.
- **Automatic:** Migrations run on first command invocation after upgrade.
  No manual step.

**Decision needed:** How do we track schema versions across two databases?

### Telemetry enforcement

"Capture everything" needs a mechanism, not just a principle.

**Enforcement via `patina doctor`:**

Doctor already audits project health (WASM plugins, configuration checks).
Add an emission coverage audit:

- For every command that has side effects (scrape, session, forge sync),
  doctor verifies a corresponding `measure::emit` call exists in the code path
- For every event type in the SPEC registry, doctor checks that at least one
  event of that type exists in events.db (post-Area 2)
- Missing emissions surface as doctor warnings with specific guidance:
  "scrape git has no measure.capture emission — see Area 2 spec"

This isn't CI gating — patina is a dev tool, not a pipeline. It's an
always-available audit that makes gaps visible. The same data feeds into
`patina measure` health assessment, so an LLM asking "are the tools working?"
can see emission gaps as part of the health picture.

**Why doctor, not compile-time:**

Compile-time enforcement (e.g., a trait that requires emit) is too rigid.
Patina evolves fast — new commands appear, old ones change shape. Doctor
audits are flexible (pattern-match on code structure) and produce actionable
warnings rather than build failures.

## Key Files (Current Architecture)

These are the files that the sub-specs will modify:

### Event system
- `src/eventlog.rs` — eventlog table creation, event insertion, feedback views
- `src/measure.rs` — measure::emit() — the primary runtime event emitter

### Runtime event writers
- `src/commands/scrape/forge/` — forge.issue, forge.pr events
- `src/commands/scry/internal/logging.rs` — scry.query, scry.use events
- `src/mcp/server/scry.rs` — scry events via MCP path

### Projection system
- `src/commands/scrape/mod.rs` — scrape orchestration, rebuild logic
- `src/commands/scrape/database.rs` — database path constants, connection helpers
- `src/commands/scrape/code/mod.rs` — code scraper
- `src/commands/scrape/git/mod.rs` — git scraper
- `src/commands/scrape/layer/mod.rs` — layer scraper
- `src/commands/scrape/beliefs/mod.rs` — beliefs scraper

### Health surface
- `src/commands/measure/internal.rs` — measure command, health computation

### Federation
- `src/mother/graph.rs` — graph.db sync, belief federation

## Open Questions

### Architectural
1. **Feedback views** — `create_feedback_views()` joins scry.query (runtime)
   with git.commit (source-derived). After the split, these live in different
   databases. Cross-db views via ATTACH? Move them? Rethink them?

2. **Event compaction** — events.db is append-only forever. At what point (if
   ever) do we compact old events? Or is "tens of thousands of rows in SQLite"
   genuinely fine forever? (Probably fine — but worth stating explicitly.)

3. **Backup story** — events.db is the irreplaceable file. What's the backup
   strategy? Git-tracked? Periodic copy? Just "it's one small file, handle it
   yourself"?

### Schema
4. ~~**Event versioning**~~ **Resolved.** Forward-compatible JSON. New fields
   additive, existing fields never removed/renamed. Readers ignore unknown
   fields, default missing fields. See cross-cutting decisions above.

5. **Session ID for events** — scry.query currently requires session_id.
   Area 2 removes this requirement. Should other events also drop the
   session_id requirement? Or is session_id valuable as optional context?

### Deferred to sub-specs (acknowledged risks)
10. **Migration cutover** — The riskiest operation (splitting a live database)
    has no executable plan yet. Dual-write strategy, backfill verification,
    seq monotonicity during split — all belong in Area 1's sub-spec DESIGN.md.
    Flag: this is the highest-risk implementation detail in the entire
    architecture.

11. **Measure JSON contract** — `patina measure --full` is promised as the LLM
    surface but has no schema, versioning policy, or failure semantics. Belongs
    in Area 4's sub-spec. Flag: downstream consumers (LLMs, scripts) can't
    implement against a contract that doesn't exist yet.

### Federation
6. **Mother scale testing** — We say "1000s of projects." Has anyone tested
   graph.db with 200K beliefs? The design says SQLite handles it — should we
   validate with a synthetic load test?

7. ~~**Sync conflict resolution**~~ **Resolved.** Projects are sovereign —
   conflicting belief evolutions coexist in mother. Mother reports divergence
   ("these projects disagree, here's the evidence"), never reconciles. Identity
   is `(source, id)` composite key. Cross-project discovery uses semantic
   similarity, not ID matching. See SPEC § Federation identity model.

### Measure
8. **What questions should measure answer?** — The spec says "is this project
   healthy?" but the specific questions matter. What's the full list of
   questions an LLM should be able to ask?

9. **Temporal queries** — Measure currently shows point-in-time health. With
   events.db, we can show trends ("belief health over last 30 days"). How
   important is this for v2 vs a future enhancement?
