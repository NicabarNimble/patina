---
type: feat
id: belief-graph
status: active
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
argument structure, regardless of which project holds each belief.

This is safe because:
- Before Phase E, each belief ID exists in exactly one project. No ambiguity.
- After Phase E, an imported belief is the *same concept* in a new project
  (same ID, same statement, `## Origin` links back). The support/attack
  relationship applies equally to the original and the imported copy.
- `source_project` identifies who *declared* the relationship, not which
  copy of the belief is involved. This is sufficient for per-source rebuild
  and provenance.

If two unrelated projects independently create beliefs with the same ID
but different meanings, the edges become ambiguous. This is a naming
collision, not an architecture bug — belief IDs are human-chosen slugs
per [[belief-identity-is-slug-not-hash]], and collisions indicate the
beliefs should be renamed. `mother graph query supports <id>` would
surface the collision by showing contradictory relationships.

```sql
-- Belief-to-belief edges (cross-project relationships)
-- source_project = the project whose belief markdown declared this relationship
-- from_belief/to_belief reference belief concepts by ID (not project-scoped)
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

-- Belief provenance (which projects have this belief)
-- DEFERRED to Phase E. Until imports exist, `query projects` derives
-- from: SELECT DISTINCT source FROM beliefs WHERE id = ?
-- Phase E write path: during sync, for each project's beliefs, insert
-- (belief_id, project, originated). originated = 1 when the belief's
-- frontmatter has no `imported_from` field (native belief). originated = 0
-- when `imported_from` exists (imported via `patina belief import`).
-- The scraper reads this from YAML frontmatter in parse_belief_file()
-- and stores an `imported` boolean in patina.db beliefs table (new
-- column, Phase E only). Detection is by frontmatter key, NOT by
-- presence of a `## Origin` section — hand-authored Origin sections
-- must not trigger the imported flag.
CREATE TABLE IF NOT EXISTS belief_applied_in (
    belief_id TEXT NOT NULL,
    project TEXT NOT NULL,
    originated INTEGER DEFAULT 0, -- 1 = created here, 0 = imported
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (belief_id, project)
);
```

**`defeated` flag semantics:** The `defeated INTEGER` column maps the
`(status: defeated)` annotation in belief markdown. In practice:

| Section | Entry | Meaning | `defeated` value |
|---------|-------|---------|------------------|
| `## Attacks` | `- [[B]] (status: defeated, ...)` | A attacked B, B won | 1 |
| `## Attacks` | `- [[B]]` or `(status: active)` | A attacks B, unresolved | 0 |
| `## Attacked-By` | `- [[A]] (status: defeated)` | A attacked me, I won | 1 (same edge) |
| `## Attacked-By` | `- [[A]] (status: active)` | A attacks me, unresolved | 0 (same edge) |

Both `## Attacks` and `## Attacked-By` describe the same edge from
opposite perspectives. The scraper writes edges from **both** sections:
- `## Attacks` on belief A: `(from_belief=A, to_belief=B)`
- `## Attacked-By` on belief B: `(from_belief=A, to_belief=B)` (reverse lookup)

Deduplication: the PK `(from_belief, to_belief, source_project)` handles
this. When both A's `## Attacks` and B's `## Attacked-By` describe the
same edge within one project, the `defeated` flag must agree. If they
disagree, the merge rule is: **`defeated=1` wins** (conservative — if
either side claims the attack was resolved, honor the resolution). The
scraper implements this as:

1. First write: `INSERT OR IGNORE` — creates the row with whichever
   `defeated` value comes first (alphabetical processing order).
2. Upgrade only: `UPDATE belief_attacks SET defeated = 1 WHERE
   from_belief = ? AND to_belief = ? AND defeated = 0` — if any
   subsequent mention claims `defeated=1`, upgrade. Never downgrade.
3. Diagnostic: when a mismatch is detected (one section says defeated,
   the other doesn't), emit to stderr:
   `⚠ <belief-id>: defeated status conflict for attack <from>→<to>
   (## Attacks says <X>, ## Attacked-By says <Y>) — using defeated=1`

This makes the flag deterministic regardless of processing order.

**Current scraper behavior** (`extract_file_metrics()` lines 402-419):
- `## Attacks` with `status: defeated` → silently skipped (not in
  `attacks_ids`, not counted). **Change needed:** also emit edge row with
  `defeated=1`.
- `## Attacked-By` with `status: defeated` → `defeated_attacks` count,
  NOT added to `attacked_by_ids`. **Change needed:** also emit edge row
  with `defeated=1`, reversing to `(from_belief=attacker, to_belief=self)`.
- Non-defeated entries → already in `attacks_ids` / `attacked_by_ids`.
  Write as edge rows with `defeated=0`.

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

**`## Supports` parsing semantics:** Audit of 131 belief files shows 234
supports entries: 228 (97.4%) use `[[wikilink]]` format, 6 (2.6%) use bare
`belief-name: explanation` format without wikilinks. The parser should:
1. Extract `[[id]]` targets using the existing `wikilink_re` regex (same
   as `## Attacks` parsing)
2. For entries with no `[[wikilink]]`, attempt to extract a bare belief ID
   from the first token before `:` or ` ` (same `belief-name: explanation`
   pattern seen in the 6 outliers)
3. Emit a `⚠ <belief-id>: ## Supports entry without [[wikilink]]: "<line>"`
   diagnostic to stderr so users know an edge was skipped or inferred

**patina.db edge tables** (created in `create_materialized_views()`):

```sql
-- Per-project belief relationship tables (scraper output)
-- Simpler than graph.db: no source_project (always current project),
-- no last_indexed (rebuilt each scrape)
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

Both tables are rebuilt each scrape run (full or incremental). Since
`cross_reference_beliefs()` always processes all beliefs and computes fresh
relationship data, edge tables are always rebuilt from scratch — this is
cheap (O(beliefs × edges_per_belief)) and avoids stale edge data after
incremental runs. Edge writes happen in a **separate pass** after the
Phase 3 insert loop, iterating ALL beliefs (not just newly inserted ones).
This ensures edges are correct after both full and incremental scrapes.
Dedup uses the same `defeated=1` wins rule as graph.db: `INSERT OR IGNORE`
\+ `UPDATE ... SET defeated = 1 WHERE defeated = 0`. See Phase B defeated
flag semantics for the full merge rule and diagnostic.

**Code paths:**
- `src/commands/scrape/beliefs/mod.rs` — add `## Supports` parsing to
  `extract_file_metrics()` (new `supports_ids: Vec<String>` field on
  `BeliefMetrics`; also capture defeated entry IDs for edge rows), add
  `belief_supports` and `belief_attacks` tables to
  `create_materialized_views()`, write edges in a separate pass after
  Phase 3 (not inside `insert_belief()`)
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

**Schema version guard:** A project's patina.db may not have the
`belief_supports`/`belief_attacks` tables yet (scraper hasn't been updated,
or hasn't re-run since update). `collect_belief_edges()` must check for
table existence before querying, using the same pattern as
`collect_project_beliefs()` line 188-196:
```rust
let table_exists: bool = conn.query_row(
    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='belief_supports'",
    [], |_| Ok(true),
).unwrap_or(false);
```
If missing, skip edge sync for that project (log warning), sync belief rows
only. No version pragma needed — table existence is the version signal.

**Dangling edge detection:** After all projects have been synced (both
belief rows and edges), run a validation pass:
```sql
-- Find edges whose endpoints don't exist in graph.db beliefs table
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
**Action:** Log warnings to stderr, do NOT auto-delete. The target belief
may exist in a project that hasn't been synced yet, or whose `patina scrape`
hasn't run. Dangling edges are stale data, not corrupted data — they'll
resolve on next full sync or get overwritten by per-source rebuild.
Only log if count > 0: `⚠ {N} dangling edge(s) — target beliefs not
in graph.db (run 'mother graph sync' after 'patina scrape' in all projects)`.

**Dedup: current project vs registry.** If the current project (auto-detected
via `find_project_root()`) is also in the registry, beliefs would be collected
twice with the same `(id, source)` pair, causing a PK violation in
`sync_beliefs()`. Fix: skip the registry entry if its resolved path matches
`project_root`. This is a pre-existing bug in `sync_from_registry()` — fix
during Phase C since this function is already being extended.

**Code paths:**
- `src/commands/mother/graph.rs` — update `collect_project_beliefs()` to
  read 12 columns, add `collect_belief_edges()` function, add dedup guard
  (skip registry entry matching current project root)
- `src/mother/graph.rs` — update `BeliefEntry` struct (renamed from
  `KnowledgeEntry` in Phase A) with additional metric fields, update
  `sync_beliefs()` signature

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
- `mode` parameter is optional; omitting it defaults to `search` (existing
  behavior). Existing clients that don't send `mode` see no change.
- Unknown `mode` values return JSON-RPC error -32602 (Invalid params) with
  message: `"unknown mode '{value}' — valid: search, supports, attacks,
  projects"`. This matches the existing validation pattern for the `query`
  parameter (`server.rs:601-606`).
- `supports` and `attacks` modes require a `belief_id` parameter (not
  `query`). If `belief_id` is missing, return -32602 with
  `"mode '{mode}' requires 'belief_id' parameter"`.
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

Workflow:
1. Query mother's graph for the belief (must exist in graph.db)
2. Resolve the source project's filesystem path via `Graph::get_node(project)`
   → `node.path` (stored in `nodes` table during `mother graph sync`).
   The source file is at `{node.path}/layer/surface/epistemic/beliefs/{belief-id}.md`.
   If `get_node()` returns None or the file doesn't exist at the resolved
   path, fail with actionable error: "project not in graph — run
   `mother graph sync`" or "belief file not found at {path}".
3. Write to local `layer/surface/epistemic/beliefs/<belief-id>.md`
4. Reset entrenchment to `low` (must earn local evidence)
5. Add `imported_from` to YAML frontmatter (machine-readable import marker):
   ```yaml
   imported_from: <source-project>
   import_date: <YYYY-MM-DD>
   ```
   This frontmatter field is the authoritative import signal — the scraper
   checks `imported_from` (not `## Origin`) to set the `imported` flag in
   patina.db. Hand-authored `## Origin` sections are narrative, not markers.
6. Append `## Origin` section with human-readable provenance:
   ```markdown
   ## Origin
   - Imported from: <source-project>
   - Original entrenchment: <original-entrenchment>
   - Import date: <date>
   ```
   If an active session exists (`.patina/local/active-session.md` has a
   valid `id:` field), append `- Import session: [[session-<id>]]`.
   If no session is active, omit the session line — the `import_date`
   frontmatter field provides the timestamp. The import command reads
   the active session via `get_active_session_id()` (same function used
   by `mcp/server.rs` for query logging). It does NOT create a session.
7. Add `belief_applied_in` record in graph.db on next sync
8. Print confirmation with belief statement and reminder:
   `Run 'patina scrape' to index the imported belief for local audit.`
   Import does NOT auto-scrape — it only writes the markdown file.
   The belief won't appear in `patina belief audit` or local patina.db
   until the user runs `patina scrape`. This is intentional: import is
   a write-to-layer operation, scrape is a separate pipeline step.
   Per [[unix-philosophy]]: one tool, one job.

**Guards:**
- Refuse if belief already exists locally (use `--force` to overwrite)
- Refuse if source project not found in mother's graph
- Refuse if belief-id not found in source project

**Path portability:** Import requires the source project to be on the
local filesystem at the path stored in graph.db's `nodes` table. This
is a deliberate local-first constraint — Patina doesn't fetch from
remote sources. If the path doesn't exist (project on another machine,
moved directory, etc.), the error is:
```
Error: source project 'foo' path does not exist: /old/path/foo
  The registry path may be stale. Options:
  - If the project moved: update registry with `patina repo register`
  - If the project is remote: clone it locally first, then register
```
No git clone fallback. No network access during import. The registry
(`~/.patina/registry.yaml`) stores local paths; if a path goes stale,
the human updates it. This matches how `mother graph sync` already
works — it reads from local paths and skips projects it can't access.

**Code paths:**
- `src/commands/belief/mod.rs` — add `Import` subcommand
- `src/mother/graph.rs` — add `get_belief()` method to fetch single belief
  with source project path from nodes table

## Project Identity

Edge tables key on `source_project TEXT` — a project name that must be
stable and unique. Current identity sources:

- **Current project:** `project_root.file_name()` — the directory basename
  (`commands/mother/graph.rs:28-31`). Fragile: renaming the directory
  changes the name, orphaning edges in graph.db.
- **Registered projects:** registry key in `~/.patina/registry.yaml`
  (`HashMap<String, ProjectEntry>`). More stable: user chose the name.
- **Persona:** hardcoded `"persona"` string. Stable.

**Decision:** Use the registry key as canonical identity. For the current
project (auto-detected, may not be in registry), use `file_name()` as
today — this is the same value that becomes the registry key when the
project is registered. If a project renames its directory, `mother graph
sync` will create a new node and the old node's edges become orphaned
(acceptable: sync again to rebuild). Name collisions between projects:
the registry `HashMap` enforces uniqueness for registered projects; two
unregistered projects with the same directory name would collide, but
only the current project is auto-detected, so this can't happen in
a single sync run.

**Future:** if identity drift becomes a real problem, add a `[patina]`
`project_id` field to `.patina/config.toml` as the authoritative name.
Not needed now — directory basenames have been stable in practice.

## What Doesn't Change

- **Scraper parsing** — `## Attacks`/`## Attacked-By` section detection and
  wikilink extraction logic unchanged. Phase B extends output: adds
  `## Supports` parsing (~10 lines), captures defeated entry IDs for edge
  rows (currently counted but IDs discarded), writes 2 new output tables
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

### Review Pass 2 — Design Gaps (same session)

5. **Schema migration path** — SPEC said "drop old, create new" but didn't
   enumerate which files reference `knowledge*` names or how to coordinate
   the rename. Added: full file list (5 files), explicit DROP sequence in
   init_schema(), backward compatibility analysis (old binary creates empty
   tables, no crash), single-commit rename requirement.

6. **Schema version guard for patina.db** — Mother reads `belief_supports`
   from patina.db, but that table may not exist yet. Added: table-existence
   check pattern (matching existing `collect_project_beliefs()` guard),
   skip edge sync with warning if tables missing. No version pragma needed.

7. **`## Supports` parsing semantics** — Audited all 131 belief files:
   228/234 entries (97.4%) use `[[wikilink]]` format, 6 use bare
   `belief-name: explanation`. Added: wikilink-first extraction, bare ID
   fallback, stderr diagnostic for skipped entries.

8. **Project identity stability** — `source_project` in edge tables uses
   directory basename, which changes on rename. Added: Project Identity
   section documenting current derivation, why registry key is canonical,
   why collisions can't happen in practice, and future escape hatch
   (`project_id` in config.toml).

9. **Phase D/E timing for `belief_applied_in`** — Phase D `query projects`
   depended on `belief_applied_in` table, which was deferred to Phase E.
   Fixed: Phase D derives from `SELECT DISTINCT source FROM beliefs` until
   Phase E creates `belief_applied_in`.

10. **Import path discovery** — SPEC step 2 said "fetch from source project"
    without specifying how CLI finds the path. Added: resolve via
    `Graph::get_node(project).path`, error messages for missing node or
    missing file.

11. **Scrape-after-import visibility** — Imported belief won't appear in
    audit until `patina scrape` runs. Added: explicit reminder in import
    output, rationale (import writes layer, scrape is separate pipeline).

12. **MCP unknown mode validation** — New `mode` parameter had no error
    spec for invalid values. Added: -32602 error for unknown modes,
    `belief_id` parameter requirement for supports/attacks modes, tool
    description update.

### Review Pass 3 — Interconnected Design Gaps (same session)

13. **Belief ID ambiguity in edge tables** — Edge tables use belief IDs
    without source qualifier, but `beliefs` table PK is `(id, source)`.
    Resolved: edges model *conceptual* relationships per Dung's framework,
    not project-scoped instances. Same-ID-different-meaning collisions are
    naming bugs surfaced by `query supports`. Added: full rationale section
    before edge DDL, reference to [[belief-identity-is-slug-not-hash]].

14. **`defeated` flag semantics** — Table had `defeated INTEGER` with no
    mapping from markdown. Added: full truth table mapping
    `(status: defeated)` annotations from both `## Attacks` and
    `## Attacked-By` sections. Documented that both sections describe the
    same edge from opposite perspectives. Noted scraper changes needed:
    currently defeated entries are counted but not emitted as edge rows.

15. **Import path portability** — Phase E assumed local filesystem access
    without documenting the constraint. Added: explicit local-first
    constraint, error message template for stale paths, no git clone
    fallback, matches existing sync behavior.

16. **`belief_applied_in` write path** — No concrete data flow from import
    to table. Resolved: deferred to Phase E, with concrete write path:
    sync reads `imported` flag from patina.db (scraper detects `## Origin`
    section), populates `originated` column. Before Phase E, `query
    projects` derives from `beliefs.source`.

17. **Dangling edge validation** — Edges could reference deleted beliefs
    indefinitely. Added: post-sync validation query, warn-not-delete
    policy (target may exist in unsynced project), actionable message.

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

### Review Pass 4 — Final Review (session [[session-20260216-214628]])

**Scope:** Completeness, internal consistency, exit criteria precision,
phase ordering, scope discipline. Read all 10 referenced files (SPEC +
4 layer/core docs + 5 source files), checked 6 review criteria.

18. **"What Doesn't Change" vs defeated-entry behavior** — Claimed
    "Scraper parsing — `## Attacks`/`## Attacked-By` parsing unchanged"
    but Phase B changes `extract_file_metrics()` to capture defeated entry
    IDs for edge rows (currently counted but IDs discarded). Fixed:
    qualified the claim to "section detection and wikilink extraction
    logic unchanged; output extended."

19. **Phase C struct rename phrasing ambiguity** — Line 346 said
    "update `KnowledgeEntry` → `BeliefEntry` struct" which could be read
    as performing the rename in Phase C (already done in Phase A). Fixed:
    "update `BeliefEntry` struct (renamed from `KnowledgeEntry` in Phase A)
    with additional metric fields."

20. **Missing patina.db edge table DDL** — Phase B specified graph.db edge
    tables but not patina.db's `belief_supports`/`belief_attacks` tables.
    These differ from graph.db: no `source_project` (always current project),
    no `last_indexed` (rebuilt each scrape). Fixed: added explicit DDL,
    dedup behavior (INSERT OR REPLACE via PK), and rebuild semantics.

### Review Pass 4 — Verified OK

- **Completeness**: All 5 phases traceable from SPEC text to specific
  files and functions. No engineer stop-and-ask points remaining.
- **Internal consistency**: No contradictions between original text and
  review amendments (passes 1-3). Three minor issues found and fixed
  (findings 18-20).
- **Exit criteria**: All 7 criteria verifiable with concrete commands.
  Criteria 1 and 3 could include specific SQL verification but are
  unambiguous as written.
- **Phase ordering**: A→C, B→C, C→D, A+C→E. Correct. Phases D and E
  can be parallelized (independent command modules, independent Graph
  methods). No circular dependencies.
- **Scope discipline**: 5 phases, ~10 files, one coherent capability
  (belief relationships via mother). Phase E (import) is the most
  independent piece but too small to justify a separate SPEC. Phased
  structure provides internal scoping per [[spec-driven-design]].
- **`belief_applied_in` DDL placement**: Shown under Phase B with
  "DEFERRED to Phase E" comment. Clear enough — the DDL is documentation
  of final state, not a Phase B instruction.

### Review Pass 5 — Human-Identified Gaps (same session)

21. **Defeated race is nondeterministic** — Both `## Attacks` and
    `## Attacked-By` produce the same edge row via `INSERT OR REPLACE`,
    so the final `defeated` flag depended on alphabetical processing
    order when the two sections disagreed. Fixed: deterministic merge
    rule — `defeated=1` wins (conservative: if either side claims
    resolution, honor it). Implementation: `INSERT OR IGNORE` + upgrade-
    only `UPDATE ... SET defeated = 1 WHERE defeated = 0`. Diagnostic
    emitted to stderr on conflict. Applied to both graph.db and patina.db
    edge dedup sections.

22. **Import detection conflates `## Origin` section with machine import**
    — `belief_applied_in` write path used "presence of `## Origin` section"
    to detect imports, but hand-authored beliefs can include `## Origin`
    for narrative purposes. Fixed: machine-readable `imported_from` field
    in YAML frontmatter is the authoritative signal. The scraper checks
    frontmatter, not section headings. `## Origin` section is human-
    readable provenance only.

23. **Session backlink in import has no producer** — Import step 5 wrote
    `Import session: [[session-YYYYMMDD-HHMMSS]]` but `patina belief
    import` doesn't create sessions. The wikilink would be dead if no
    session was active. Fixed: import reads active session ID via
    `get_active_session_id()` (existing function, used by MCP query
    logging). If a session is active, appends the link. If not, omits
    the session line — `import_date` in frontmatter provides the
    timestamp regardless.

### Review Pass 6 — Code-Grounded Review (session [[session-20260216-221447]])

**Scope:** Read all 6 referenced source files + 3 session archives. Verified
code references, function names, line numbers, struct definitions, phase
ordering, and cross-referencing behavior against current codebase.

24. **`## Attacks` defeated behavior misdescribed** — SPEC claimed
    `## Attacks` with `status: defeated` increments `defeated_attacks`.
    Actual code (`extract_file_metrics()` line 414): defeated entries are
    silently skipped — no counter, no ID capture. Only `## Attacked-By`
    increments `defeated_attacks` (line 404). Fixed: corrected current-
    behavior description. Change request (emit edge row) unchanged.

25. **Edge table rebuild timing ambiguity** — SPEC said "dropped and
    rebuilt each full scrape" but `cross_reference_beliefs()` runs on
    every scrape (full and incremental), computing fresh relationship
    data for ALL beliefs. Rebuilding edges only on full scrape leaves
    them stale after incremental runs. Fixed: changed to "each scrape
    run (full or incremental)" with rationale (cheap, avoids staleness).

26. **Pre-existing duplicate collection in `sync_from_registry()`** —
    If the current project (auto-detected, named by `file_name()`) is
    also in the registry, beliefs are collected twice with the same
    `(id, source)` pair. The `sync_knowledge()` transaction hits a PK
    violation on the second INSERT and ROLLBACKs. Pre-existing bug, not
    introduced by this SPEC. Fixed: added dedup guard to Phase C code
    paths (skip registry entry matching current project root).

27. **Edge writes must be a separate pass from `insert_belief()`** —
    On incremental scrape, `insert_belief()` only runs for new beliefs.
    If edge tables are rebuilt every scrape but edges are written inside
    `insert_belief()`, existing beliefs' edges would be lost. Fixed:
    clarified that edge writes happen in a separate pass after Phase 3,
    iterating ALL beliefs. Updated code paths accordingly.

### Review Pass 6 — Verified OK

- **Phase A rename scope**: All 5 files confirmed. `KnowledgeEntry` at
  `graph.rs:148`, re-export at `mod.rs:39`, call sites at
  `commands/mother/graph.rs:8,132,148`, `mcp/server.rs:609`.
- **`get_active_session_id()`**: Exists in `mcp/server.rs` and
  `commands/scry/internal/logging.rs`. SPEC reference valid.
- **`collect_project_beliefs()` reads 5 columns**: Confirmed at line 200.
- **`extract_file_metrics()` line references**: `## Attacked-By` at
  402-410, `## Attacks` at 412-418. Correct.
- **FTS5 lifecycle**: `DROP TABLE IF EXISTS` on virtual table removes
  shadow tables. Clean rename path confirmed.
- **Phase ordering**: A,B (parallelizable) → C → D,E (parallelizable).
- **Exit criteria**: All 7 testable with concrete commands.
- **MCP backward compatibility**: `mode` default `search` preserves
  existing behavior. Error codes documented.
- **Non-goals**: Comprehensive, no scope creep risk.
