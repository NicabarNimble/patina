---
type: feat
id: cross-project-beliefs
status: complete
created: 2026-02-15
sessions:
  origin: 20260215-083121
related:
- layer/core/patina-identity.md
- layer/core/oxidized-knowledge.md
- layer/surface/build/feat/mother-design/SPEC.md
beliefs:
- beliefs-are-the-product
- belief-identity-is-slug-not-hash
- mother-is-the-daemon
- mother-owns-ref-repo-indexing
- mcp-is-shim-cli-is-product
---

# feat: Cross-Project Knowledge — Federated Belief & Value Search

> Mother indexes project beliefs and persona values into one searchable table.
> CLI and MCP expose cross-project knowledge search. Mother v2 Phase 2.

## Terminology

This SPEC uses "beliefs" (project-scoped) and "values" (persona-scoped) as
distinct kinds of epistemic knowledge. See [[mother-design]] for the full
knowledge model, vocabulary definitions, and design principles behind the
project/Mother boundary.

## Problem

130+ project beliefs live in `layer/surface/epistemic/beliefs/`. 5 persona
values live in `~/.patina/layer/surface/beliefs/`. Project beliefs from other
registered projects are invisible. These pools can't see each other:

- `patina scry "error handling"` searches project semantic index only
- `patina persona query "error handling"` searches legacy persona JSONL only
- No way to search beliefs across projects or values alongside beliefs

Per [[beliefs-are-the-product]]: if the belief system can't link knowledge
across projects, it's not doing its job.

## User Story

You're in project-A, talking to an LLM via adapter. The LLM asks Mother:
"do other projects or the user's persona have knowledge about error handling?"

Mother searches her index and returns beliefs AND values together:

```
[project-B]  explicit-error-types     belief  (high entrenchment, 3 evidence)
[persona]    prefer-result-over-panics value   (low entrenchment)
[ref:beads]  errors-are-values         belief  (extracted)
```

The user decides: adopt, reject, or tweak. Adoption creates a native belief
in project-A's layer — the cross-project link lives in Mother's graph.

## What Exists Today

### Persona values (`~/.patina/layer/surface/beliefs/`)

Phase 1 (mother-v2) created the user-layer directory structure and migrated 5
persona notes to markdown value files. These files use the same format as project
beliefs (YAML frontmatter + markdown sections) but simpler — no verification
queries, no project-scoped grounding metrics.

No code path currently reads these markdown files for indexing — that's what
this SPEC builds.

### Legacy persona path (`src/commands/persona/mod.rs`)

`note()` appends `PersonaEvent` to `~/.patina/personas/default/events/YYYYMMDD.jsonl`.
`materialize()` builds `~/.patina/cache/personas/default/persona.db` + `.usearch`
with 768-dim E5 embeddings. `query()` does cosine similarity search.

This is the legacy path. See [[mother-design]] for deprecation roadmap.

### Project belief scrape (`src/commands/scrape/beliefs/mod.rs`)

Parses `layer/surface/epistemic/beliefs/*.md` YAML frontmatter + markdown. Computes
rich metrics (citations, evidence, verification, grounding). Stores in `beliefs` table
with FTS5 index (`belief_fts`), verification table (`belief_verifications`), code reach
table (`belief_code_reach`).

Mother's index doesn't need per-project metrics — statement, entrenchment,
facets, and status are sufficient for cross-project discovery.

### Mother graph (`src/mother/graph.rs`)

`graph.db` at `~/.patina/mother/graph.db`. Schema: `nodes` (projects/refs with
path, domains, importance), `edges` (USES, LEARNS_FROM, TESTS_WITH, SIBLING, DOMAIN
with weight learning), `edge_usage` (feedback loop). No belief-related tables.

### Scry federation (`src/retrieval/engine.rs`)

`query_all_repos()` iterates registered repos and does per-repo semantic search.
Persona is NOT included in `--all-repos` — it's a separate command entirely.

### MCP tool architecture (`src/mcp/server.rs`)

Three MCP tools exist today. Each is a thin wrapper over library functions —
per [[mcp-is-shim-cli-is-product]], MCP wraps CLI logic, never implements its own.

| Tool | Role | Scope |
|------|------|-------|
| `scry` | Semantic vector search | Project |
| `assay` | Structural/factual queries | Project |
| `context` | Composition: layer files + scry + assay + beliefs | Project |

No cross-project knowledge tool exists — that's what this SPEC builds.

`context` is the aggregation tool — it calls `assay_search()`, `engine.query()`,
`search_beliefs_fts()`, and reads `layer/` markdown files. Currently ~14K chars
(~3K tokens) without topic filter. With topic, smaller (only matching content).

## What To Build

### Phase A: Knowledge Index in graph.db

Add a `knowledge` table to Mother's `graph.db` that caches both project beliefs
and persona values in one searchable index.

```sql
-- In ~/.patina/mother/graph.db (alongside existing nodes/edges tables)
CREATE TABLE IF NOT EXISTS knowledge (
    id TEXT NOT NULL,
    source TEXT NOT NULL,          -- 'persona', registry project name, or ref repo name
    kind TEXT NOT NULL,            -- 'belief' or 'value' (see [[mother-design]])
    statement TEXT NOT NULL,
    entrenchment TEXT DEFAULT 'medium',
    status TEXT DEFAULT 'active',
    facets TEXT,                   -- JSON array of domain tags
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (id, source)
);

-- Regular FTS5 table (not external content) — Mother owns her own copy.
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_search USING fts5(
    id, source, kind, statement, facets,
    tokenize='porter unicode61'
);
```

**Source naming:** The `source` field uses the registry key from
`~/.patina/registry.yaml` — a HashMap, so keys are unique by construction.
Persona uses the literal `'persona'`. Ref repos use their registry key (e.g.,
`"beads"`); the `[ref:beads]` display format is a CLI presentation concern,
not stored. The `(id, source)` primary key means two projects can hold a
belief with the same slug without collision.

**Why graph.db, not a separate beliefs.db:** Knowledge entries link to graph
nodes via the `source` field (which matches `nodes.id`). Future phases will
add edges (adopted-from, contradicts) between project nodes — the adoption
provenance lives in `edges.evidence`, not duplicated in knowledge. A single
DB means these joins are local, not cross-database. The tradeoff: you can't
wipe knowledge without touching graph edges. Acceptable — `mother graph sync`
is idempotent and rebuilds the knowledge table from project sources.

**Why no embedding column:** Cross-project search is exploratory, not
precision-critical. Per-project `scry --belief` handles vector precision. FTS5
is sufficient for cross-project discovery. Embeddings would require a
cross-project oxidize pipeline — future scope per [[mother-owns-ref-repo-indexing]].

**Schema migration:** `CREATE TABLE IF NOT EXISTS` in `Graph::init_schema()`
is sufficient — knowledge is a rebuildable cache, not source data. graph.db
has no `PRAGMA user_version` today and doesn't need one for this SPEC. If
columns change in a future SPEC, `mother graph sync` rebuilds from source
(same pattern as `patina rebuild` for patina.db). Older binaries that don't
know about these tables are unaffected — they never query them.

### Phase B: Mother Sync — Extend `mother graph sync`

Extend the existing `patina mother graph sync` command to sync knowledge
alongside nodes. Same registry walk, same `Graph::open()`, one more table.

**Extended command:** `patina mother graph sync`

Current behavior (unchanged):
1. Walk registered projects from `~/.patina/registry.yaml`
2. Create nodes for all projects and repos

New behavior (added):
3. For each project, try to open its `.patina/local/data/patina.db` and read
   the `beliefs` table (id, statement, entrenchment, status, facets). Insert
   into `knowledge` with source = registry project name (matches graph node
   ID), kind = `'belief'`.
   - **No patina.db** (never scraped): skip with warning, continue. Node is
     still created (step 2) — knowledge comes later when the user scrapes.
   - **No `beliefs` table** (legacy schema): skip with warning, continue.
   - **Ref repos**: no patina.db expected — skip knowledge sync. Ref repo
     belief extraction is future scope per [[mother-owns-ref-repo-indexing]].
4. Read `~/.patina/layer/surface/beliefs/*.md` — parse YAML frontmatter for
   id, statement, entrenchment, status, facets. Insert into `knowledge` with
   source = `'persona'`, kind = `'value'`.
   - **Required fields**: `id` (from filename if missing) and `statement`
     (first non-empty line after `# heading`, or id as fallback).
   - **Defaults**: entrenchment = `'medium'`, status = `'active'`, facets = `[]`.
   - **Malformed files**: warn to stderr, skip file, continue sync.
5. Wrap the knowledge rebuild in a single SQLite transaction:
   `BEGIN` → `DELETE FROM knowledge` → `DELETE FROM knowledge_search` →
   repopulate both tables → `COMMIT`. On failure the transaction rolls back
   and old data is preserved. CLI/MCP queries during sync see either the old
   complete state or the new complete state, never partial.

**Code paths:**
- `src/commands/mother/graph.rs` — extend `sync_from_registry()` with knowledge sync
- `src/mother/graph.rs` — add `sync_knowledge()` and `search_knowledge()` on `Graph`
- `src/paths.rs` — add `user_layer::beliefs_dir()` for `~/.patina/layer/surface/beliefs/`

### Phase C: Cross-Project Knowledge Search — CLI + MCP

Two interfaces to the same library function (`Graph::search_knowledge()`),
following the existing pattern where CLI and MCP are both thin wrappers over
shared Rust library code.

#### CLI: `patina mother search "query"`

Direct human interface. FTS5 search on `knowledge_search` in graph.db.
No daemon required — opens graph.db directly.

```
$ patina mother search "error handling"

[patina]          explicit-error-types     belief  high
                  "Prefer explicit error types over string errors..."

[persona]         prefer-result-over-panics value  medium
                  "I prefer Result<T,E> over panics..."

[bevy-playground] panic-in-systems-is-ok   belief  low
                  "Bevy systems can panic — the runner catches..."

3 results from 2 projects + persona
```

**Code paths:**
- `src/commands/mother/mod.rs` — add `Search` variant to `MotherCommands`
- `src/commands/mother/graph.rs` — add `search_knowledge()` CLI handler

#### MCP: New `mother` tool

New MCP tool alongside scry, assay, and context. Same pattern — thin wrapper
over `Graph::search_knowledge()`. Single-purpose: cross-project FTS5 search.

```json
{
  "tool": "mother",
  "arguments": {
    "query": "error handling patterns",
    "limit": 10
  }
}
```

Returns knowledge entries tagged with source, kind, entrenchment. JSON for
LLM consumption.

**Code paths:**
- `src/mcp/server.rs` — add `mother` tool to `handle_list_tools()` and `handle_tool_call()`

#### Search API contract

Shared by CLI and MCP — both call `Graph::search_knowledge()`.

- **Default limit**: 10 (CLI `--limit`, MCP `limit` param)
- **Ordering**: FTS5 rank (relevance to query)
- **Return fields**: id, source, kind, statement, entrenchment, status, facets
- **CLI display**: statement truncated to 200 chars, one entry per 2 lines
  (source + id + kind + entrenchment on line 1, statement on line 2)
- **MCP return**: JSON array of objects with all fields, full statement
  (no truncation — LLM manages its own token budget)
- **Empty results**: CLI prints "No results." MCP returns empty array.
- **No filters in v1**: no facet filter, no kind filter, no source filter.
  FTS5 query is the only input. Filters are future scope.

#### MCP tool architecture after this SPEC

| Tool | Role | Scope |
|------|------|-------|
| `scry` | Semantic vector search | Project |
| `assay` | Structural/factual queries | Project |
| `mother` | Cross-project knowledge search | Mother (graph.db) |
| `context` | Composition: layer files + scry + assay + beliefs | Project |

The individual tools (scry, assay, mother) provide direct access for targeted
search. `context` remains project-scoped — integrating mother into context is
a follow-up composition concern (see Non-Goals).

## Exit Criteria

1. `patina mother graph sync` syncs knowledge alongside nodes — project beliefs
   + persona values populate graph.db's `knowledge` and `knowledge_search` tables
2. `patina mother search "error handling"` returns results tagged with source
   and kind: `[project-name] belief`, `[persona] value`
3. MCP tool `mother` returns cross-project knowledge as JSON
4. `patina scrape` is unchanged — no graph.db writes, project stays island
5. Running `mother graph sync` twice produces identical graph.db state (idempotent)

**Verification context:** Run exit criteria post-`patina scrape` on at least one
registered project. Expected: project beliefs (130+) + 5 persona values.
Each result shows ID, statement, source, kind, entrenchment.

## Non-Goals

- Embedding/semantic search in graph.db — FTS5 is sufficient for discovery
- Belief adoption workflow — follow-up SPEC
- Mother daemon auto-sync — manual `mother graph sync` for now
- Extending `context` to include cross-project knowledge — composition concern,
  follow-up after mechanism proves out
- Refactoring `context` token budget — separate concern, can be tuned later
- Everything in [[mother-design]] non-goals (persona migration, multi-persona,
  value grounding, values-to-rules, etc.)

## Evidence

| Claim | Source |
|-------|--------|
| Persona values exist as markdown | `~/.patina/layer/surface/beliefs/` (5 files) |
| Persona legacy writes to JSONL events | `src/commands/persona/mod.rs:76-87` |
| Persona uses separate persona.db + usearch | `src/commands/persona/mod.rs:91-227` |
| Project beliefs have rich metrics | `src/commands/scrape/beliefs/mod.rs:23-82` |
| graph.db has nodes + edges, no knowledge table | `src/mother/graph.rs:186-234` |
| Scry all-repos doesn't include persona | `src/retrieval/engine.rs:224-261` |
| MCP tools are library wrappers, not CLI shims | `src/mcp/server.rs` (calls Rust fns directly) |
| `context` is ~14K chars (~3K tokens) no topic | `patina context \| wc -c` = 14106 |
| `context` calls assay + scry + reads layer files | `src/commands/context.rs:56-68` (not modified by this SPEC) |
| `mother graph sync` already walks registry | `src/commands/mother/graph.rs:16-68` |
| Graph node IDs are registry names, not UIDs | `src/commands/mother/graph.rs:35-46` (uses `name` from registry) |
| Mother v2 Phase 2 schema proposed | `git show spec/mother-v2` (archived) |
| `paths.rs` has no user-layer module | `src/paths.rs` (entire file, no `user_layer`) |
| No `~/.patina/uid` exists for persona | filesystem check — literal `'persona'` as source |
