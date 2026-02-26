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

**Decision: per-request ATTACH.** The MCP server already opens a new SQLite
connection for every request (scry, assay, mother — all per-request). No
persistent database connections exist. Adding ATTACH follows the same model:

```
open patina.db → ATTACH events.db READONLY → query → close both
```

Overhead: ~200μs per ATTACH (connection open + ATTACH statement) vs
10-100ms for the actual query. Negligible. Per-request ATTACH also avoids
WAL contention — events.db stays available for concurrent writers without
long-lived reader connections blocking checkpoints.

No persistent connection infrastructure needed. No connection pooling,
health monitoring, or reconnect logic. The current per-request model
scales unchanged.

### Migration strategy

Area 1 needs a one-time migration (copy runtime events from patina.db to
events.db). Conventions for future migrations:

- **Schema version tracking:** `PRAGMA user_version` in each database.
  Currently 0 in patina.db (never set). Each database tracks its own version
  independently — events.db starts at version 1 when created.
- **Forward-only:** Migrations go forward. No rollback. If something breaks,
  rebuild patina.db from sources + events. events.db is append-only so
  schema changes are additive (new columns, new indices — never column
  removal).
- **Automatic:** Migrations run on first command invocation after upgrade.
  No manual step. The initialization code checks `PRAGMA user_version` and
  applies pending migrations before proceeding.

**Decision: `PRAGMA user_version` per database, independent versioning.**
patina.db and events.db evolve at different rates. patina.db schema changes
with every scraper update; events.db schema is nearly frozen (the eventlog
table shape rarely changes — evolution happens in the JSON data blobs).
Two independent version counters, each checked on database open.

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

## Sub-Spec Readiness Assessment

| Area | Ready? | Rationale |
|------|--------|-----------|
| **1: DB Split** | **YES** | All cross-cutting decisions resolved (ATTACH, schema versioning, migration sketch). ~7 writer files, ~3 reader files identified. Gate is concrete. Create as draft spec now. |
| **2: Emission Completeness** | **YES (blocked)** | 8 emission gaps documented. session_id resolved. Event registry locked. Blocked on Area 1 — can draft now with `blocked_by: [data-db-split]`. |
| **3: Mother Schema** | **YES** | Schema drift documented (4 missing column groups, 71 dangling edges). Independent of Areas 1-2. Create as draft spec now. |
| **4: Measure Surface** | **Not yet** | Open questions #8 (what questions?), #9 (temporal?), #11 (JSON contract) unresolved. Needs more design work. Defer until Areas 1-2 are in progress. |
| **5: Fast Incremental** | **Not yet** | Scope is clear but no profiling data. Last in sequence. Defer until architecture is correct (post-Phase C). |

**Action:** Create Area 1, Area 2, and Area 3 as draft sub-specs now. Areas 4-5 get
created when their predecessors are near completion and their design questions are resolved.

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
1. ~~**Feedback views**~~ **Resolved.** `create_feedback_views()` is dead code
   — defined in eventlog.rs but never called. The live implementation is
   `execute_feedback()` in `src/commands/eval/mod.rs` which does its own temp
   table materialization, bypassing the views entirely.

   After the split, `eval --feedback` becomes an ATTACH consumer:
   - **scry.query events** → read from events.db (runtime, via ATTACH)
   - **commit/file data** → read from patina.db's `commits` + `commit_files`
     tables (structured, already there)
   - The join correlates "what did the LLM search for?" (events.db) with
     "what files actually changed?" (patina.db) — a natural cross-system query.

   This is a **simplification**: the current code parses git.commit JSON blobs
   from the eventlog; after the split it reads from structured tables.
   `create_feedback_views()` should be deleted during Area 1 cleanup.
   `execute_feedback()` gets rewritten as part of Area 4 (measure surface).

2. ~~**Event compaction**~~ **Resolved: no compaction, ever.**

   The numbers don't justify it. Current runtime events (the ones moving to
   events.db): ~96 rows. Projected growth: ~3,500/year. After 10 years:
   ~35K events at ~200 bytes each = ~7MB. SQLite handles millions of rows.

   Compaction would add complexity (which events to keep? rollup semantics?
   archive format?) to solve a problem that won't exist. The autobiography
   framing makes this clear: you don't summarize your journal entries to
   save paper. Every event is a fact the system can reason about.

   If a project ever reaches scale where events.db size matters (millions
   of events, unlikely for a local dev tool), the right answer is time-based
   partitioning (events-2026.db, events-2027.db), not compaction. But this
   is a decade-away concern not worth designing for now.

3. ~~**Backup story**~~ **Resolved: events.db is source, JSONL is replica.**

   events.db is the source of truth for runtime knowledge — not a cache,
   not a materialization of git. It IS the thing. The durability question
   is: how do you survive machine loss?

   **Mechanism:** `layer/events.jsonl` — a git-tracked append-only JSONL
   **replica**. Each line is one event. On session end and scrape, new events
   since last export are appended and committed. events.db feeds the JSONL,
   not the other way around.

   **Recovery:** `patina events import layer/events.jsonl` rebuilds events.db
   from the replica. Loss window: events since last export (typically hours).
   Full disaster recovery: clone the repo, import events, scrape — project
   is fully restored.

   **Scale:** ~3,500 events/year × ~200 bytes = ~700KB/year of JSONL. After
   10 years: ~7MB. Git handles line-oriented text well.

   **Doctor audit:** "events.db has N events, JSONL replica has M events,
   gap is K" — makes replica staleness visible and actionable.

   **Why this works:**
   - events.db is the source of truth (Schickling-aligned)
   - JSONL is a replica that travels with git for machine-loss durability
   - Respects [[if-its-patina-its-git]] as revised: git + events.db are the
     two sources of truth, JSONL replicates one into the other for safety
   - No new infrastructure — just a text file and an export command

### Schema
4. ~~**Event versioning**~~ **Resolved.** Forward-compatible JSON. New fields
   additive, existing fields never removed/renamed. Readers ignore unknown
   fields, default missing fields. See cross-cutting decisions above.

5. ~~**Session ID for events**~~ **Resolved: always optional context, never a gate.**

   Currently `log_scry_query()` uses `get_active_session_id()?` — the early
   return `?` means queries outside sessions are silently dropped. This violates
   "capture everything." The fix (Area 2): session_id is an optional JSON field,
   included when available, absent when not. Events fire regardless of session
   state.

   This applies to ALL event types:
   - **session.start/end:** session_id IS the event — always present by definition
   - **scry.query, measure.capture, etc.:** session_id as optional context
   - **belief.created, spec.promoted, etc.:** session_id as optional context
     (which session triggered it? useful but not required)

   Consistent with forward-compatible JSON conventions: readers default missing
   fields. If session_id is null, the event still happened — it just happened
   outside a tracked session.

### Deferred to sub-specs (acknowledged risks)
10. ~~**Migration cutover**~~ **Risk downgraded. Sketch below.**

    The migration is lower-risk than initially feared. Current state: 96
    runtime events (154KB) vs 97,242 source-derived events (19.6MB). The
    migration moves a tiny fraction of data.

    **Runtime event types (→ events.db):**
    - `measure.*` — tool execution metrics (3 events)
    - `scry.*` — search queries, usage, feedback (0 events currently)
    - `forge.*` — GitHub API cache (92 events)
    - Future: `session.start/end`, `belief.*`, `spec.*`, `decision.*`, `discovery.*`

    **Source-derived event types (→ stay in patina.db as eventlog rows):**
    - `code.*` — tree-sitter parse output (~80K events)
    - `git.*` — commit/tag data (~8K events)
    - `session.*` — parsed from layer/sessions/ markdown (~5K events)
    - `pattern.*` — parsed from layer/ patterns (~50 events)
    - `belief.surface` — parsed from belief markdown

    **Migration sequence (one-time, on first command after upgrade):**

    ```
    Step 1: Check — does events.db already exist with data?
            YES → skip migration (idempotent)
            NO  → proceed

    Step 2: Create events.db with eventlog schema
            Same table shape: (seq, event_type, timestamp, source_id,
            source_file, data) with AUTOINCREMENT
            Set PRAGMA user_version = 1

    Step 3: Copy runtime events from patina.db → events.db
            INSERT INTO events.eventlog (event_type, timestamp, ...)
            SELECT event_type, timestamp, ...
            FROM patina.eventlog
            WHERE event_type LIKE 'measure.%'
               OR event_type LIKE 'scry.%'
               OR event_type LIKE 'forge.%'
            ORDER BY timestamp ASC
            -- New seq values via AUTOINCREMENT (don't preserve old seqs)
            -- Timestamp ordering ensures chronological monotonicity

    Step 4: Verify
            - events.db row count matches expected
            - All runtime event types present
            - No seq gaps (AUTOINCREMENT is contiguous for fresh DBs)

    Step 5: Done. Runtime events remain in patina.db until next rebuild.
            Next `patina scrape --rebuild` deletes patina.db and recreates
            it — runtime events are now safely in events.db.
    ```

    **Why this is safe:**
    - The migration is a COPY, not a MOVE. patina.db retains the original
      data until the next rebuild. If the migration is buggy, no data is lost.
    - Idempotent: events.db existence check prevents double-migration.
    - Self-verifiable: count checks confirm completeness.
    - Tiny volume: 96 rows, <1 second to copy.

    **The real risk is the code changes**, not the data migration:
    - ~7 runtime event writers need to target events.db instead of patina.db
    - ~3 readers need ATTACH (measure, eval --feedback, scry logging)
    - `execute_rebuild()` must skip events.db (change: don't delete events.db)
    - Forge dedup must check events.db instead of patina.db eventlog

    Full implementation details belong in Area 1's sub-spec DESIGN.md.

11. **Measure JSON contract** — **Confirmed deferred to Area 4.** The contract
    depends on what data is available (Areas 1-2 must ship first) and what
    questions matter (see resolved #8 above for the initial catalog). The
    question catalog gives Area 4 a concrete target list. Specifying the
    JSON schema before the data exists would be premature — the contract
    should be derived from implementation, not guessed from vision.

### Federation
6. ~~**Mother scale testing**~~ **Resolved: defer.** 200K rows is well within
   SQLite's proven range (designed for billions of rows). No synthetic load
   test needed at this stage. If scale issues emerge during data-mother-schema
   implementation, profile and address then. The risk is low — the bottleneck
   will be sync network I/O, not SQLite query performance.

7. ~~**Sync conflict resolution**~~ **Resolved.** Projects are sovereign —
   conflicting belief evolutions coexist in mother. Mother reports divergence
   ("these projects disagree, here's the evidence"), never reconciles. Identity
   is `(source, id)` composite key. Cross-project discovery uses semantic
   similarity, not ID matching. See SPEC § Federation identity model.

### Measure
8. ~~**What questions should measure answer?**~~ **Resolved: initial catalog.**

   The measure question catalog — what an LLM should be able to answer after
   Area 4. Each question maps to the verb/layer that provides the answer:

   | # | Question | Primary Source | Verb |
   |---|----------|---------------|------|
   | 1 | "Is this project healthy?" | Aggregate across all verbs | all |
   | 2 | "Are beliefs grounded in code?" | beliefs table (grounding_score, evidence_count) | believe |
   | 3 | "Are tools running and capturing data?" | events.db (measure.capture events) | capture |
   | 4 | "What's drifting — beliefs, coverage, freshness?" | beliefs table + scrape_meta + embeddings age | believe + capture |
   | 5 | "What changed since last session?" | session.ended events, git commits | evolve |
   | 6 | "Which beliefs are contested?" | belief_attacks table, health_score | believe |
   | 7 | "Is the event stream flowing?" | events.db event counts by recency | capture (meta) |
   | 8 | "How has scrape performance trended?" | events.db measure.capture over time | capture (temporal) |
   | 9 | "What knowledge is stale?" | beliefs.last_activity, embeddings age, scrape_meta | believe + index |

   This is the initial catalog. Area 4's sub-spec refines it into a JSON
   contract. Questions 1-7 are point-in-time (v2 minimum). Questions 8-9
   require temporal queries (v2 stretch, see #9 below).

9. ~~**Temporal queries**~~ **Resolved: point-in-time is v2, trends are
   stretch.**

   The architecture supports temporal queries — events.db has the history.
   But building the trend analysis surface is Area 4 scope, not vision scope.

   - **Minimum (v2):** Point-in-time health. "Are beliefs grounded right now?"
     "Did the last scrape succeed?" Current snapshot, no time series.
   - **Stretch (v2):** 30-day trends for key metrics. "How has scrape duration
     changed?" "Are beliefs gaining or losing grounding?" Requires simple
     window queries over events.db — the data exists, it's a surface question.
   - **Future:** Anomaly detection, drift alerts, automated health regression.
     Not v2 scope.

   Area 4's sub-spec decides what ships. The vision spec's job is to ensure
   the data architecture makes temporal queries *possible* — and it does,
   because events.db is append-only with timestamps.
