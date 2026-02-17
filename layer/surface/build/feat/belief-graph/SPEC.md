---
type: feat
id: belief-graph
status: active
created: 2026-02-16
sessions:
  origin: 20260216-155323
  build-1: 20260217-055500
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
    imported INTEGER DEFAULT 0,    -- Phase E: 1 = imported via `belief import`
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
`belief_search`. graph.db is a rebuildable cache, so migration is:

1. In `init_schema()`: `DROP TABLE IF EXISTS knowledge; DROP TABLE IF
   EXISTS knowledge_search;` then create new `beliefs` + `belief_search`
   tables. Old tables are cleaned up on first open after upgrade.
2. All function renames (`sync_knowledge` → `sync_beliefs`, etc.) and
   struct renames (`KnowledgeEntry` → `BeliefEntry`) happen in the same
   commit to prevent partial-rename compilation failures.
3. **Files requiring coordinated rename** (all reference `knowledge*` or
   `KnowledgeEntry` by name):
   - `src/mother/graph.rs` — schema DDL, struct, 4 methods
   - `src/mother/mod.rs` — re-export `KnowledgeEntry`
   - `src/commands/mother/graph.rs` — calls `sync_knowledge()`,
     `KnowledgeEntry` in `collect_project_beliefs()`
   - `src/commands/mother/mod.rs` — help text
   - `src/mcp/server.rs` — tool description, `handle_mother_search()`
4. **Backward compatibility:** An old binary opening the migrated graph.db
   will execute `CREATE TABLE IF NOT EXISTS knowledge` (succeeds, empty),
   and return empty search results until re-synced. No crash. This is
   acceptable because graph.db data is always rebuildable via
   `mother graph sync`.

**Code paths:**
- `src/mother/graph.rs` — update `init_schema()` (drop old + create new),
  rename `sync_knowledge()` → `sync_beliefs()`, `search_knowledge()` →
  `search_beliefs()`, `knowledge_count()` → `belief_count()`,
  `KnowledgeEntry` → `BeliefEntry`
- `src/mother/mod.rs` — update re-export
- `src/commands/mother/graph.rs` — update all call sites
- `src/commands/mother/mod.rs` — update help text
- `src/mcp/server.rs` — update tool description and handler

### Phase B: Belief Relationship Edges

Add edge tables for belief-to-belief relationships. These edges come from
the `## Supports` and `## Attacks` sections that the scraper already parses.

**Belief ID semantics in edges:** Edge tables reference beliefs by ID only
(not by `(id, source)` pair), because edges model relationships between
belief *concepts*, not project-specific instances. Per Dung's framework,
"explicit-error-types SUPPORTS sync-first" is a statement about the
argument structure, regardless of which project holds each belief. Before
Phase E each ID exists in one project (no ambiguity); after Phase E an
imported belief is the same concept (same ID, same statement). Naming
collisions between unrelated beliefs are surfaced by `query supports`.

```sql
-- graph.db: Belief-to-belief edges (cross-project relationships)
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
    defeated INTEGER DEFAULT 0,
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (from_belief, to_belief, source_project)
);

-- graph.db: Belief provenance — DEFERRED to Phase E.
-- Until imports exist, derive from: SELECT DISTINCT source FROM beliefs WHERE id = ?
CREATE TABLE IF NOT EXISTS belief_applied_in (
    belief_id TEXT NOT NULL,
    project TEXT NOT NULL,
    originated INTEGER DEFAULT 0, -- 1 = created here, 0 = imported
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (belief_id, project)
);
```

```sql
-- patina.db: Per-project edge tables (scraper output)
-- Simpler than graph.db: no source_project, no last_indexed (rebuilt each scrape)
CREATE TABLE IF NOT EXISTS belief_supports (
    from_belief TEXT NOT NULL,
    to_belief TEXT NOT NULL,
    PRIMARY KEY (from_belief, to_belief)
);

CREATE TABLE IF NOT EXISTS belief_attacks (
    from_belief TEXT NOT NULL,
    to_belief TEXT NOT NULL,
    defeated INTEGER DEFAULT 0,
    PRIMARY KEY (from_belief, to_belief)
);
```

**`defeated` flag semantics:**

| Section | Entry | Meaning | `defeated` value |
|---------|-------|---------|------------------|
| `## Attacks` | `- [[B]] (status: defeated, ...)` | A attacked B, B won | 1 |
| `## Attacks` | `- [[B]]` or `(status: active)` | A attacks B, unresolved | 0 |
| `## Attacked-By` | `- [[A]] (status: defeated)` | A attacked me, I won | 1 (same edge) |
| `## Attacked-By` | `- [[A]] (status: active)` | A attacks me, unresolved | 0 (same edge) |

Both sections describe the same edge from opposite perspectives. Dedup
merge rule: **`defeated=1` wins** (conservative). Implementation:
`INSERT OR IGNORE` + `UPDATE ... SET defeated = 1 WHERE defeated = 0`.
Diagnostic on conflict to stderr.

**Current scraper behavior** (`extract_file_metrics()` lines 402-419):
- `## Attacks` with `status: defeated` → silently skipped (not in
  `attacks_ids`, not counted). **Change:** also emit edge row with
  `defeated=1`.
- `## Attacked-By` with `status: defeated` → `defeated_attacks` count,
  NOT added to `attacked_by_ids`. **Change:** also emit edge row with
  `defeated=1`, reversing to `(from_belief=attacker, to_belief=self)`.
- Non-defeated entries → already in `attacks_ids` / `attacked_by_ids`.
  Write as edge rows with `defeated=0`.

**`## Supports` parsing:** Parse via existing `wikilink_re` regex (same as
`## Attacks`). For entries without `[[wikilink]]`, extract bare belief ID
from first token before `:` or space. Warn on non-link entries to stderr.

**Edge table rebuild:** Both patina.db edge tables are rebuilt each scrape
run (full or incremental). Edge writes happen in a **separate pass** after
the Phase 3 insert loop, iterating ALL beliefs (not just newly inserted
ones), because `cross_reference_beliefs()` always computes fresh data for
all beliefs.

**Code paths:**
- `src/commands/scrape/beliefs/mod.rs` — add `## Supports` parsing to
  `extract_file_metrics()` (new `supports_ids: Vec<String>` on
  `BeliefMetrics`; capture defeated entry IDs for edge rows), add edge
  tables to `create_materialized_views()`, write edges in separate pass
- `src/mother/graph.rs` — add edge tables to `init_schema()`, add
  `sync_belief_edges()` method
- `src/commands/mother/graph.rs` — extend `sync_from_registry()` to read
  relationship tables and call `sync_belief_edges()`

### Phase C: Richer Sync

Extend `mother graph sync` to pull more columns from each project's patina.db
and to sync belief edges.

**Current sync reads 5 columns:**
`id, statement, entrenchment, status, facets`

**Phase C sync reads 12 columns (Phase E adds a 13th):**
`id, statement, entrenchment, status, facets, cited_by_beliefs,
cited_by_sessions, applied_in, evidence_count, evidence_verified,
health_score, contested_by`
Phase E extends this to 13 columns by adding `imported` — see Phase E
sync changes. `BeliefEntry` struct grows correspondingly in each phase.

**Edge sync:** After syncing belief rows, read `belief_supports` and
`belief_attacks` from each project's patina.db and insert into graph.db's
edge tables. Per-source rebuild: delete edges where `source_project = ?`
for successfully synced sources only, per [[commit-44b1b338]] pattern.

**Schema version guard:** A project's patina.db may not have the
`belief_supports`/`belief_attacks` tables yet. Check table existence before
querying (same pattern as `collect_project_beliefs()` line 188-196). If
missing, skip edge sync for that project (log warning). No version pragma
needed — table existence is the version signal.

**Dangling edge detection:** After all projects are synced, validate:
```sql
SELECT 'supports' AS type, s.from_belief, s.to_belief, s.source_project
FROM belief_supports s
WHERE s.from_belief NOT IN (SELECT id FROM beliefs)
   OR s.to_belief NOT IN (SELECT id FROM beliefs)
UNION ALL
SELECT 'attacks', a.from_belief, a.to_belief, a.source_project
FROM belief_attacks a
WHERE a.from_belief NOT IN (SELECT id FROM beliefs)
   OR a.to_belief NOT IN (SELECT id FROM beliefs);
```
Log warnings to stderr, do NOT auto-delete. Dangling edges are stale data
that may resolve on next full sync.

**Dedup: current project vs registry.** If the current project (auto-detected
via `find_project_root()`) is also in the registry, beliefs get collected
twice with the same `(id, source)` pair → PK violation. Fix: skip registry
entry if its path matches `project_root`. Pre-existing bug, fix here.

**Code paths:**
- `src/commands/mother/graph.rs` — update `collect_project_beliefs()` to
  read 12 columns (Phase E extends to 13), add `collect_belief_edges()`,
  add dedup guard
- `src/mother/graph.rs` — update `BeliefEntry` struct (renamed from
  `KnowledgeEntry` in Phase A) with 12 metric fields (Phase E adds
  `imported`), update `sync_beliefs()` signature

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
  → Before Phase E: SELECT DISTINCT source FROM beliefs WHERE id = ?
  → After Phase E: SELECT from belief_applied_in WHERE belief_id = ?
  → Shows which projects have this belief
```

**MCP:** Update the existing `mother` MCP tool to accept a `mode` parameter:
`search` (default, current behavior), `supports`, `attacks`, `projects`.

MCP backward compatibility:
- `mode` is optional; omitting defaults to `search` (existing behavior).
- Unknown `mode` → JSON-RPC -32602: `"unknown mode '{value}'"`.
- `supports`/`attacks` modes require `belief_id` parameter (not `query`).
  Missing → -32602: `"mode '{mode}' requires 'belief_id' parameter"`.
- Tool description in `tools/list` updated to document all modes.

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

**Workflow:**
1. Query mother's graph for the belief (must exist in graph.db)
2. Resolve source project path via `Graph::get_node(project)` → `node.path`.
   Source file at `{node.path}/layer/surface/epistemic/beliefs/{belief-id}.md`.
   Fail with actionable error if node or file missing.
3. Write to local `layer/surface/epistemic/beliefs/<belief-id>.md`
4. Reset entrenchment to `low` (must earn local evidence)
5. Add `imported_from` to YAML frontmatter (machine-readable import marker):
   ```yaml
   imported_from: <source-project>
   import_date: <YYYY-MM-DD>
   ```
6. Append `## Origin` section (human-readable provenance):
   ```markdown
   ## Origin
   - Imported from: <source-project>
   - Original entrenchment: <original-entrenchment>
   - Import date: <date>
   ```
   If active session exists, append `- Import session: [[session-<id>]]`
   via `get_active_session_id()`. Omit if no session active.
7. Print reminder: `Run 'patina scrape' to index the imported belief.`
   Import does NOT auto-scrape. Per [[unix-philosophy]]: one tool, one job.

**Guards:**
- Refuse if belief already exists locally (use `--force` to overwrite)
- Refuse if source project not found in mother's graph
- Refuse if belief-id not found in source project

**Path portability:** Import requires source project on local filesystem
at path in graph.db's `nodes` table. No git clone fallback. No network
access. Stale paths → error with `patina repo register` suggestion.

**Import detection in scraper (new in Phase E):** The `imported_from`
frontmatter field is the **sole** authoritative signal for imports.
`## Origin` sections are narrative only — hand-authored Origin sections
must NOT trigger the imported flag.

Phase E scraper changes:
- `create_materialized_views()`: add `imported INTEGER DEFAULT 0` column
  to `beliefs` table (via ALTER TABLE, same migration pattern as E4 columns)
- `parse_belief_file()`: read `imported_from` from YAML frontmatter, set
  `imported = 1` if present, `0` if absent
- `insert_belief()`: write the `imported` value to the new column

Phase E sync changes:
- `collect_project_beliefs()`: add `imported` to SELECT (13th column)
- `sync_from_registry()`: after syncing belief rows, populate
  `belief_applied_in` table: `originated = 1` when `imported = 0`
  (native belief), `originated = 0` when `imported = 1`.
  Per-source rebuild: `DELETE FROM belief_applied_in WHERE project = ?`
  for each successfully synced source before re-inserting, same pattern
  as edge tables' `source_project` cleanup. This prevents stale rows
  when a project removes or renames a belief.

**Code paths:**
- `src/commands/belief/mod.rs` — add `Import` subcommand
- `src/commands/scrape/beliefs/mod.rs` — add `imported` column + frontmatter
  detection (Phase E only, not Phase B)
- `src/commands/mother/graph.rs` — extend sync to read `imported` column,
  populate `belief_applied_in`
- `src/mother/graph.rs` — add `get_belief()` method to fetch single belief
  with source project path from nodes table

## Project Identity

Edge tables key on `source_project TEXT` — must be stable and unique.

- **Current project:** `project_root.file_name()` — directory basename.
  Fragile on rename, but matches registry key when registered.
- **Registered projects:** registry key in `~/.patina/registry.yaml`.
  More stable: user chose the name.
- **Persona:** hardcoded `"persona"`. Stable.

**Decision:** Use registry key as canonical identity. For current project
(auto-detected), use `file_name()`. Directory rename orphans old edges
(acceptable: sync again). Registry `HashMap` enforces uniqueness.

**Future:** if identity drift is a real problem, add `project_id` to
`.patina/config.toml`. Not needed now.

## What Doesn't Change

- **Scraper parsing** — `## Attacks`/`## Attacked-By` section detection and
  wikilink extraction logic unchanged. Phase B extends output: adds
  `## Supports` parsing, captures defeated entry IDs for edge rows, writes
  2 new output tables
- **Belief markdown format** — no structural changes (Phase E adds optional
  `imported_from` frontmatter key)
- **Assay/Scry** — stay project-scoped, no `--projects` flags
- **`mother graph link/unlink/learn/stats`** — project-level edges unchanged
- **Persona beliefs** — stay as markdown in `~/.patina/layer/surface/beliefs/`
- **`patina belief audit`** — unchanged, still reads local patina.db

## Layer Responsibilities

| Layer | Does | Doesn't |
|-------|------|---------|
| **Scraper** | Parse beliefs (incl. `## Supports`), compute metrics, write patina.db + edge tables | Push to mother |
| **Mother graph sync** | Read patina.db + edges, populate graph.db | Create synthetic edges |
| **Mother graph query** | Join belief + edge tables, surface candidates | Auto-export or auto-promote |
| **belief import** | Human-triggered fetch, local write, entrenchment reset | Skip human decision |
| **Assay/Scry** | Project-scoped search, unchanged | Grow cross-project flags |

## Academic Grounding

- **Dung's Argumentation Framework** (1995) — supports/attacks graph structure
- **Gärdenfors' Epistemic Entrenchment** (1988) — belief ranking by centrality
- **IBIS** (Kunz & Rittel, 1970) — design rationale capture during work

See [[session-20260216-155323]] for detailed mapping.

## Exit Criteria

1. `patina scrape` writes `belief_supports` and `belief_attacks` tables to
   patina.db alongside existing `beliefs` table
2. `mother graph sync` populates graph.db `beliefs` table with 12 columns
   after Phase C (13 after Phase E adds `imported`) + belief edge tables
   from relationship data. Phase E also populates `belief_applied_in`.
3. `mother graph query belief "error handling"` returns beliefs with
   metrics (health_score, evidence_count, etc.)
4. `mother graph query supports <belief-id>` returns supporting beliefs
   across projects
5. `patina belief import --from patina <belief-id>` writes belief locally
   with entrenchment=low + `## Origin` provenance + `imported_from` frontmatter
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

## Build Progress

| Phase | Status | Session | Commits |
|-------|--------|---------|---------|
| A: Belief nodes in graph.db | DONE | [[session-20260217-055500]] | [[commit-ff2e1f25]] |
| B: Belief relationship edges | DONE | same | [[commit-f3db3826]] |
| C: Richer sync + dedup fix | DONE | same | [[commit-17166a84]] |
| D: `mother graph query` | TODO | — | — |
| E: `patina belief import` | TODO | — | — |

**Session 1 verified exit criteria 1, 2, 7.** Criteria 3–6 require Phases D+E.

## Review History

*30 findings across 7 passes, all resolved. Details in session archives.*

| Pass | Session | Findings | Focus |
|------|---------|----------|-------|
| 1 | [[session-20260216-211931]] | 1–4 | Factual errors (supports parsing, redundant columns, deferral, rebuild) |
| 2 | same | 5–12 | Design gaps (migration, schema guard, parsing semantics, identity, timing, path discovery, visibility, MCP validation) |
| 3 | same | 13–17 | Interconnected gaps (belief ID ambiguity, defeated semantics, path portability, applied_in flow, dangling edges) |
| 4 | [[session-20260216-214628]] | 18–20 | Final review (defeated-entry behavior claim, rename phrasing, patina.db DDL) |
| 5 | same | 21–23 | Human-identified gaps (defeated race, import detection conflation, session backlink) |
| 6 | [[session-20260216-221447]] | 24–27 | Code-grounded review (defeated count misdescribed, edge rebuild timing, sync dedup, edge write ordering) |
| 7 | same | 28–30 | Regression fixes (stale review note, imported column spec, imported propagation) |
