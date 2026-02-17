---
type: feat
id: belief-graph
status: ready
created: 2026-02-16
sessions:
  origin: 20260216-155323
related:
- layer/surface/build/feat/cross-project-beliefs/SPEC.md
- layer/surface/build/feat/mother-design/SPEC.md
- layer/core/dependable-rust.md
beliefs:
- beliefs-are-the-product
- design-gaps-are-specs-not-bugs
- mother-is-the-daemon
- mcp-is-shim-cli-is-product
---

# feat: Belief Graph — Cross-Project Belief Relationships via Mother

> Mother's graph maps beliefs as first-class nodes with supports/attacks edges
> across projects. Humans discover, import, and promote beliefs. Graph routes,
> assay/scry search. Each layer keeps its job.

## TLDR

You work on project B. You ask "what do other projects know about error
handling?" Mother shows you beliefs from project A with supports/attacks
context, evidence counts, health scores. You pick one:
`patina belief import --from patina explicit-error-types`. It lands in your
project at entrenchment=low with full provenance. You build local evidence.
It grows. Your projects learn from each other through you, not through magic.

## Problem

[[cross-project-beliefs]] (Phase 2) built a flat FTS5 keyword index in
graph.db. Real-world testing revealed three issues:

1. **FTS5 duplicates assay's mechanism.** Assay already does FTS5 for
   single-project search. Mother doing FTS5 at cross-project scope is the
   wrong layer doing the wrong job. Mother's value is the **graph** — routing
   based on relationships, not keyword matching.

2. **The graph ignores beliefs.** Graph nodes are projects. Graph edges are
   LEARNS_FROM, TESTS_WITH. Beliefs exist as flat rows in a `knowledge` table
   with no relationship to each other. The supports/attacks structure that the
   scraper already parses is invisible to mother.

3. **No adoption pathway.** A user can see beliefs from other projects but
   can't bring one into their own project. No import, no provenance tracking,
   no entrenchment reset.

## What Exists Today (Reuse Map)

| Need | Already Have | Where |
|------|-------------|-------|
| Belief metadata (33 columns) | `beliefs` table with metrics, health, grounding | patina.db per project |
| Attacks/Attacked-By parsing | Scraper parses `## Attacks`, `## Attacked-By` into `attacks_ids`/`attacked_by_ids` Vec<String> | `src/commands/scrape/beliefs/mod.rs:extract_file_metrics()` |
| Supports parsing | **Not yet parsed** — scraper skips `## Supports` sections (only counts `cited_by_beliefs` via cross-reference) | Phase B must add parsing |
| Cross-project sync | `mother graph sync` reads beliefs from each project | `src/commands/mother/graph.rs` |
| Graph nodes + edges | `nodes` + `edges` tables with typed edges and weight learning | graph.db |
| Edge usage + learning | `edge_usage` table, `mother graph learn` with EMA weights | graph.db |
| Belief inspection | `patina belief audit` with rich metrics and warnings | `src/commands/belief/mod.rs` |
| Graph management CLI | `mother graph link/unlink/show/stats/learn` | `src/commands/mother/` |
| Belief markdown format | `## Supports`, `## Attacks`, `## Evidence`, `## Applied-In` | `layer/surface/epistemic/beliefs/` |
| FTS5 query sanitization | `sanitize_fts5_query()` quotes tokens | `src/mother/graph.rs` |
| Per-source rebuild | `sync_knowledge()` preserves unsynced sources | `src/mother/graph.rs` |

**Most infrastructure exists.** Scraper parses `## Attacks`/`## Attacked-By`
into ID vectors; `## Supports` parsing is new but follows the same pattern
(~10 lines). Graph has node/edge infrastructure with weight learning. The
work is: supports parsing, relationship output tables, richer sync,
belief-aware edges, import command, query subcommands.

## What To Build

### Phase A: Belief Nodes in graph.db

Replace the flat `knowledge` table with a proper `beliefs` table that carries
metrics from the scraper. The existing `knowledge_search` FTS5 table stays
(renamed to `belief_search`).

```sql
-- Replace knowledge table
CREATE TABLE IF NOT EXISTS beliefs (
    id TEXT NOT NULL,
    source TEXT NOT NULL,          -- project name or 'persona'
    kind TEXT NOT NULL,            -- 'belief' or 'value'
    statement TEXT NOT NULL,
    entrenchment TEXT DEFAULT 'medium',
    status TEXT DEFAULT 'active',
    facets TEXT,                   -- JSON array
    -- Metrics (synced from patina.db, not computed here)
    cited_by_beliefs INTEGER DEFAULT 0,
    cited_by_sessions INTEGER DEFAULT 0,
    applied_in INTEGER DEFAULT 0,
    evidence_count INTEGER DEFAULT 0,
    evidence_verified INTEGER DEFAULT 0,
    health_score REAL DEFAULT 0.0,
    contested_by TEXT DEFAULT '',
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (id, source)
);

-- Renamed from knowledge_search
CREATE VIRTUAL TABLE IF NOT EXISTS belief_search USING fts5(
    id, source, kind, statement, facets,
    tokenize='porter unicode61'
);
```

**Schema migration:** `knowledge` → `beliefs`, `knowledge_search` →
`belief_search`. Since these are rebuildable caches, migration is:
drop old tables if exist, create new tables, next sync repopulates.

**Code paths:**
- `src/mother/graph.rs` — update `init_schema()`, rename `sync_knowledge()`
  → `sync_beliefs()`, update `search_knowledge()` → `search_beliefs()`

### Phase B: Belief Relationship Edges

Add edge tables for belief-to-belief relationships. These edges come from
the `## Supports` and `## Attacks` sections that the scraper already parses.

```sql
-- Belief-to-belief edges (cross-project relationships)
-- source_project = the project whose belief markdown declared this relationship
CREATE TABLE IF NOT EXISTS belief_supports (
    from_belief TEXT NOT NULL,
    to_belief TEXT NOT NULL,
    source_project TEXT NOT NULL,
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (from_belief, to_belief, source_project)
);

CREATE TABLE IF NOT EXISTS belief_attacks (
    from_belief TEXT NOT NULL,
    to_belief TEXT NOT NULL,
    source_project TEXT NOT NULL,
    defeated INTEGER DEFAULT 0,   -- from ## Attacked-By status
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (from_belief, to_belief, source_project)
);

-- Belief provenance (which projects have this belief)
-- Note: before Phase E, this is derivable from beliefs.source.
-- After Phase E imports, a belief can exist in multiple projects
-- with different sources, making this table necessary.
-- Can be deferred from Phase B to Phase E if desired.
CREATE TABLE IF NOT EXISTS belief_applied_in (
    belief_id TEXT NOT NULL,
    project TEXT NOT NULL,
    originated INTEGER DEFAULT 0, -- 1 if this is where the belief was created
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (belief_id, project)
);
```

**Where edge data comes from:** The scraper parses `## Attacks` and
`## Attacked-By` sections into `attacks_ids` and `attacked_by_ids` Vec<String>
in memory (`extract_file_metrics()` lines 402-419), but only writes aggregated
counts (`cited_by_beliefs`, `contested_by` CSV) to patina.db. `## Supports`
sections are **not currently parsed** — `cited_by_beliefs` is computed by
cross-referencing all belief file content for ID mentions, not by parsing
`## Supports` specifically.

Two options for getting the raw relationship data into mother:

**Option 1: Read markdown directly.** `mother graph sync` reads each project's
`layer/surface/epistemic/beliefs/*.md` and parses `## Supports`/`## Attacks`
sections to extract `[[wikilink]]` targets. Duplicates scraper parsing but
keeps mother independent of patina.db schema details.

**Option 2: Add relationship tables to patina.db.** Extend scraper to:
(a) parse `## Supports` sections the same way `## Attacks` is parsed (~10
lines in `extract_file_metrics()`), and (b) write `belief_supports` and
`belief_attacks` tables in patina.db from the parsed ID vectors. Mother reads
structured data, no markdown parsing.

**Recommendation:** Option 2. The scraper already parses `## Attacks` and
`## Attacked-By` — adding `## Supports` is the same pattern. Writing parsed
results to tables keeps mother's interface to patina.db clean. Per
[[dependable-rust]]: keep each module's interface small and stable.

**Code paths:**
- `src/commands/scrape/beliefs/mod.rs` — add `## Supports` parsing to
  `extract_file_metrics()` (new `supports_ids: Vec<String>` field on
  `BeliefMetrics`), add `belief_supports` and `belief_attacks` tables to
  `create_materialized_views()`, write edges in `insert_belief()`
- `src/mother/graph.rs` — add edge tables to `init_schema()`, add
  `sync_belief_edges()` method
- `src/commands/mother/graph.rs` — extend `sync_from_registry()` to read
  relationship tables and call `sync_belief_edges()`

### Phase C: Richer Sync

Extend `mother graph sync` to pull more columns from each project's patina.db
and to sync belief edges.

**Current sync reads 5 columns:**
`id, statement, entrenchment, status, facets`

**New sync reads 12 columns:**
`id, statement, entrenchment, status, facets, cited_by_beliefs,
cited_by_sessions, applied_in, evidence_count, evidence_verified,
health_score, contested_by`

**Edge sync:** After syncing belief rows, read `belief_supports` and
`belief_attacks` from each project's patina.db and insert into graph.db's
edge tables. Per-source rebuild: delete edges where `source_project = ?`
for successfully synced sources only, per [[commit-44b1b338]] pattern.
Similarly, `belief_applied_in` deletes where `project = ?` for synced sources.

**Code paths:**
- `src/commands/mother/graph.rs` — update `collect_project_beliefs()` to
  read 12 columns, add `collect_belief_edges()` function
- `src/mother/graph.rs` — update `KnowledgeEntry` → `BeliefEntry` struct
  with additional fields, update `sync_beliefs()` signature

### Phase D: `mother graph query` — Belief-Aware Graph Traversal

Add subcommands to query the belief graph. These are SQL joins over the
tables from Phase A + B.

```
patina mother graph query belief "error handling"
  → FTS5 search over belief_search, returns beliefs with metrics

patina mother graph query supports <belief-id>
  → SELECT from belief_supports WHERE to_belief = ?
  → Shows which beliefs support this one, across all projects

patina mother graph query attacks <belief-id>
  → SELECT from belief_attacks WHERE to_belief = ?
  → Shows which beliefs attack this one, with defeated status

patina mother graph query projects <belief-id>
  → SELECT from belief_applied_in WHERE belief_id = ?
  → Shows which projects have this belief
```

**MCP:** Update the existing `mother` MCP tool to accept a `mode` parameter:
`search` (default, current behavior), `supports`, `attacks`, `projects`.

**Code paths:**
- `src/commands/mother/graph.rs` — add `query_beliefs_cli()` with subcommands
- `src/commands/mother/mod.rs` — add `Query` variant to graph subcommands
- `src/mcp/server.rs` — extend `mother` tool with `mode` parameter
- `src/mother/graph.rs` — add query methods: `query_supports()`,
  `query_attacks()`, `query_projects()`

### Phase E: `patina belief import` — Human-Driven Adoption

New subcommand on the existing `patina belief` command.

```
patina belief import --from <project> <belief-id>
```

Workflow:
1. Query mother's graph for the belief (must exist in graph.db)
2. Fetch the source belief markdown from the source project's
   `layer/surface/epistemic/beliefs/<belief-id>.md`
3. Write to local `layer/surface/epistemic/beliefs/<belief-id>.md`
4. Reset entrenchment to `low` (must earn local evidence)
5. Append `## Origin` section with provenance:
   ```markdown
   ## Origin
   - Imported from: <source-project>
   - Original entrenchment: <original-entrenchment>
   - Import date: <date>
   - Import session: [[session-YYYYMMDD-HHMMSS]]
   ```
6. Add `belief_applied_in` record in graph.db on next sync
7. Print confirmation with belief statement

**Guards:**
- Refuse if belief already exists locally (use `--force` to overwrite)
- Refuse if source project not found in mother's graph
- Refuse if belief-id not found in source project

**Code paths:**
- `src/commands/belief/mod.rs` — add `Import` subcommand
- `src/mother/graph.rs` — add `get_belief()` method to fetch single belief
  with source project path from nodes table

## What Doesn't Change

- **Scraper parsing** — `## Attacks`/`## Attacked-By` parsing unchanged
  (Phase B adds `## Supports` parsing + 2 output tables)
- **Belief markdown format** — no structural changes
- **Assay/Scry** — stay project-scoped, no `--projects` flags
- **`mother graph link/unlink/learn/stats`** — project-level edges unchanged
- **Persona beliefs** — stay as markdown in `~/.patina/layer/surface/beliefs/`
- **`patina belief audit`** — unchanged, still reads local patina.db

## Layer Responsibilities

| Layer | Does | Doesn't |
|-------|------|---------|
| **Scraper** | Parse beliefs (incl. new `## Supports` parsing), compute metrics, write patina.db + relationship tables | Push to mother |
| **Mother graph sync** | Read patina.db + edges, populate graph.db | Create synthetic edges |
| **Mother graph query** | Join belief + edge tables, surface candidates | Auto-export or auto-promote |
| **belief import** | Human-triggered fetch, local write, entrenchment reset | Skip human decision |
| **Assay/Scry** | Project-scoped search, unchanged | Grow cross-project flags |

## Academic Grounding

The belief system draws from three established fields:

- **Dung's Argumentation Framework** (1995) — supports/attacks graph structure
- **Gärdenfors' Epistemic Entrenchment** (1988) — belief ranking by centrality
- **IBIS** (Kunz & Rittel, 1970) — design rationale capture during work

See [[session-20260216-155323]] for detailed mapping.

## Exit Criteria

1. `patina scrape` writes `belief_supports` and `belief_attacks` tables to
   patina.db alongside existing `beliefs` table
2. `mother graph sync` populates graph.db `beliefs` table with 12 columns
   (metrics included) + belief edge tables from relationship data
3. `mother graph query belief "error handling"` returns beliefs with
   metrics (health_score, evidence_count, etc.)
4. `mother graph query supports <belief-id>` returns supporting beliefs
   across projects
5. `patina belief import --from patina <belief-id>` writes belief locally
   with entrenchment=low + `## Origin` provenance
6. Imported belief appears in local `patina belief audit` after `patina scrape`
7. Per-source rebuild preserved: syncing from project B doesn't wipe
   project A's beliefs or edges in graph.db

## Non-Goals

- Automatic promotion to core (human decides)
- Automatic cross-project export (no `portable` tags)
- Nightly sync automation (explicit `mother graph sync`)
- Semantic/vector search in graph.db (FTS5 for discovery, scry for precision)
- NDJSON serialization layer between scraper and mother
- Extending `context` MCP tool with cross-project beliefs
- `EVOLVES_FROM` edge type (future: belief lineage tracking)
- Core layer changes (existing core/*.md files stay as-is)

## Implementability Review Notes

*Session: [[session-20260216-211931]]*

### Issues Found and Fixed

1. **Scraper `## Supports` parsing gap** — Reuse Map claimed scraper already
   parses `## Supports`. Verified against `extract_file_metrics()` (line
   373-425): it only parses `## Attacks` and `## Attacked-By`. `## Supports`
   is not parsed; `cited_by_beliefs` is computed by cross-referencing all
   belief content, not by reading `## Supports` sections. Fixed: Reuse Map,
   Phase B description, code paths, "What Doesn't Change", and Layer
   Responsibilities all updated to reflect this.

2. **Redundant `from_source` column in edge tables** — `belief_supports`
   and `belief_attacks` had both `from_source` and `source_project`.
   `from_source` (the project that owns `from_belief`) is always the same
   project that declared the relationship (`source_project`), since you can
   only declare supports/attacks in your own belief files. Removed
   `from_source`; simplified PK to `(from_belief, to_belief, source_project)`.

3. **`belief_applied_in` deferral** — Before Phase E (import), this table
   is derivable from `beliefs.source` since each belief exists in exactly one
   project. Added note that it can be deferred from Phase B to Phase E.

4. **Per-source rebuild for edges** — SPEC referenced the per-source pattern
   but didn't specify which column to use for edge deletion. Added: delete
   edges where `source_project = ?` for supports/attacks, `project = ?` for
   applied_in.

### Verified OK

- **Schema correctness**: graph.db `beliefs` table columns align with
  patina.db fields available for sync. FTS5 table rename is clean.
- **Code path accuracy**: All referenced files, function names, and line
  ranges verified against current code (post-fix commits).
- **Phase D/E CLI integration**: `Query` variant in `GraphCommands` and
  `Import` variant in `BeliefCommands` add cleanly, no naming conflicts.
  MCP `mode` parameter is backward-compatible (default = `search` = current).
- **`KnowledgeEntry` → `BeliefEntry` rename**: struct at `graph.rs:148`,
  re-exported at `commands/mother/graph.rs:8`. Clean rename path.
- **Exit criteria**: all 7 are testable with concrete commands.
- **Per-source rebuild**: works for belief rows and edge tables via
  `source_project`/`project` column filtering.
- **Phase B feasibility (Option 2)**: `## Attacks` parsing pattern at
  lines 411-419 of scraper provides exact template for `## Supports`.
  Writing to new tables follows `insert_belief()` pattern.
