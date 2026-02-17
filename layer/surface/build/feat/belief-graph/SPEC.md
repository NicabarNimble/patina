---
type: feat
id: belief-graph
status: draft
created: 2026-02-16
sessions:
  origin: 20260216-155323
supersedes: cross-project-beliefs
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
| Supports/Attacks parsing | Scraper parses `## Supports`, `## Attacks`, `## Attacked-By` | `src/commands/scrape/beliefs/mod.rs` |
| Cross-project sync | `mother graph sync` reads beliefs from each project | `src/commands/mother/graph.rs` |
| Graph nodes + edges | `nodes` + `edges` tables with typed edges and weight learning | graph.db |
| Edge usage + learning | `edge_usage` table, `mother graph learn` with EMA weights | graph.db |
| Belief inspection | `patina belief audit` with rich metrics and warnings | `src/commands/belief/mod.rs` |
| Graph management CLI | `mother graph link/unlink/show/stats/learn` | `src/commands/mother/` |
| Belief markdown format | `## Supports`, `## Attacks`, `## Evidence`, `## Applied-In` | `layer/surface/epistemic/beliefs/` |
| FTS5 query sanitization | `sanitize_fts5_query()` quotes tokens | `src/mother/graph.rs` |
| Per-source rebuild | `sync_knowledge()` preserves unsynced sources | `src/mother/graph.rs` |

**The heavy lifting is done.** Scraper parses relationships. Graph has
node/edge infrastructure with weight learning. The work is: richer sync,
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
CREATE TABLE IF NOT EXISTS belief_supports (
    from_belief TEXT NOT NULL,
    from_source TEXT NOT NULL,
    to_belief TEXT NOT NULL,
    source_project TEXT NOT NULL,  -- which project declared this relationship
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (from_belief, from_source, to_belief, source_project)
);

CREATE TABLE IF NOT EXISTS belief_attacks (
    from_belief TEXT NOT NULL,
    from_source TEXT NOT NULL,
    to_belief TEXT NOT NULL,
    source_project TEXT NOT NULL,
    defeated INTEGER DEFAULT 0,   -- from ## Attacked-By status
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (from_belief, from_source, to_belief, source_project)
);

-- Belief provenance (which projects have this belief)
CREATE TABLE IF NOT EXISTS belief_applied_in (
    belief_id TEXT NOT NULL,
    project TEXT NOT NULL,
    originated INTEGER DEFAULT 0, -- 1 if this is where the belief was created
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (belief_id, project)
);
```

**Where edge data comes from:** The scraper already parses `## Supports` and
`## Attacks` sections in belief markdown files. Currently it only stores
aggregated counts (`cited_by_beliefs`) and CSV (`contested_by`) in patina.db.

Two options for getting the raw relationship data into mother:

**Option 1: Read markdown directly.** `mother graph sync` reads each project's
`layer/surface/epistemic/beliefs/*.md` and parses `## Supports`/`## Attacks`
sections to extract `[[wikilink]]` targets. Duplicates scraper parsing but
keeps mother independent of patina.db schema details.

**Option 2: Add relationship tables to patina.db.** Extend scraper to write
`belief_supports` and `belief_attacks` tables in patina.db. Mother reads
structured data, no markdown parsing. Cleaner but requires scraper change.

**Recommendation:** Option 2. The scraper already does the parsing — it should
write the structured result. Mother reads structured data. Per
[[dependable-rust]]: keep each module's interface small and stable.

**Code paths:**
- `src/commands/scrape/beliefs/mod.rs` — add `belief_supports` and
  `belief_attacks` tables to patina.db (scraper already has the parsed data)
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
edge tables. Per-source rebuild: only delete edges for successfully synced
sources, per [[commit-44b1b338]] pattern.

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

- **Scraper** — already parses everything we need (Phase B adds 2 output
  tables, no parsing changes)
- **Belief markdown format** — no structural changes
- **Assay/Scry** — stay project-scoped, no `--projects` flags
- **`mother graph link/unlink/learn/stats`** — project-level edges unchanged
- **Persona beliefs** — stay as markdown in `~/.patina/layer/surface/beliefs/`
- **`patina belief audit`** — unchanged, still reads local patina.db

## Layer Responsibilities

| Layer | Does | Doesn't |
|-------|------|---------|
| **Scraper** | Parse beliefs, compute metrics, write patina.db + relationship tables | Push to mother |
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
