---
type: feat
id: cross-project-beliefs
status: active
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
with FTS5 index (`belief_fts`), evidence table (`belief_evidence`), code reach table
(`belief_code_reach`).

Mother's index doesn't need per-project metrics — statement, entrenchment,
facets, and status are sufficient for cross-project discovery.

### Mother graph (`src/mother/graph.rs`)

`graph.db` at `~/.patina/mother/graph.db`. Schema: `nodes` (projects/refs with
path, domains, importance), `edges` (USES, LEARNS_FROM, TESTS_WITH, SIBLING, DOMAIN
with weight learning), `edge_usage` (feedback loop). No belief-related tables.

### Scry federation (`src/retrieval/engine.rs`)

`query_all_repos()` iterates registered repos and does per-repo semantic search.
Persona is NOT included in `--all-repos` — it's a separate command entirely.

## What To Build

### Phase A: Knowledge Index in graph.db

Add a `knowledge` table to Mother's `graph.db` that caches both project beliefs
and persona values in one searchable index.

```sql
-- In ~/.patina/mother/graph.db (alongside existing nodes/edges tables)
CREATE TABLE IF NOT EXISTS knowledge (
    id TEXT NOT NULL,
    source TEXT NOT NULL,          -- 'persona', project UID, or ref repo name
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

**Why graph.db, not a separate beliefs.db:** Knowledge entries are nodes in
the graph — they connect projects via shared principles, and future phases will
add edges (adopted-from, contradicts) between entries and project nodes. A single
DB means these joins are local, not cross-database. The tradeoff: you can't wipe
knowledge without touching graph edges. Acceptable — `mother sync` is idempotent
and rebuilds the knowledge table from project sources.

**Why no embedding column:** Cross-project search is exploratory, not
precision-critical. Per-project `scry --belief` handles vector precision. FTS5
is sufficient for cross-project discovery. Embeddings would require a
cross-project oxidize pipeline — future scope per [[mother-owns-ref-repo-indexing]].

### Phase B: Mother Sync — Pull From Project Islands

Mother indexes knowledge by **pulling** from project databases and persona value
files. Projects never write into Mother. `patina scrape` stays project-pure.
See [[mother-design]] for the principle behind this boundary.

**New command:** `patina mother sync`

1. Walk registered projects from `~/.patina/registry.yaml`
2. For each project, open its `.patina/local/data/patina.db`, read the `beliefs`
   table (id, statement, entrenchment, status, facets). Insert into `knowledge`
   with source = project UID (from `.patina/uid`), kind = `'belief'`.
3. Read `~/.patina/layer/surface/beliefs/*.md` — parse YAML frontmatter for
   id, statement, entrenchment, status, facets. Insert into `knowledge` with
   source = `'persona'`, kind = `'value'`.
4. Clear and rebuild graph.db's `knowledge` + `knowledge_search` tables (idempotent).

**Code paths:**
- `src/commands/mother/mod.rs` — add `sync` subcommand
- `src/mother/graph.rs` — add `sync_knowledge()` method on `Graph`
- `src/paths.rs` — add `user_layer::beliefs_dir()` for `~/.patina/layer/surface/beliefs/`

### Phase C: Cross-Project Knowledge Search

Mother exposes knowledge search as a CLI command and MCP tool.

**CLI:** `patina mother search "error handling"`

1. FTS5 search on `knowledge_search` in graph.db
2. Results tagged with source and kind:
   `[project-B] belief`, `[persona] value`, `[ref:beads] belief`
3. Human output shows: ID, statement, source, kind, entrenchment
4. JSON output for MCP consumption

**MCP:** New mode on existing `scry` tool. The LLM calls this during
conversation when it notices a topic that might have cross-project knowledge.

```json
{
  "tool": "scry",
  "arguments": {
    "query": "error handling patterns",
    "mode": "mother-knowledge"
  }
}
```

**Code paths:**
- `src/commands/mother/mod.rs` — add `search` subcommand
- `src/mother/graph.rs` — add `search_knowledge()` method on `Graph`
- `src/mcp/server.rs` — add `mother-knowledge` mode to scry tool handler

## Exit Criteria

1. `patina mother sync` reads project beliefs + persona values, populates
   graph.db's `knowledge` and `knowledge_search` tables
2. `patina mother search "error handling"` returns results tagged with source
   and kind: `[project-name] belief`, `[persona] value`
3. MCP tool `scry` with `mode: "mother-knowledge"` returns cross-project
   knowledge during LLM conversation
4. `patina scrape` is unchanged — no graph.db writes, project stays island
5. Running `mother sync` twice produces identical graph.db state (idempotent)

**Verification context:** Run exit criteria post-`patina scrape` on at least one
registered project. Expected: project beliefs (130+) + 5 persona values.
Each result shows ID, statement, source, kind, entrenchment.

## Non-Goals

- Embedding/semantic search in graph.db — FTS5 is sufficient for discovery
- Belief adoption workflow — follow-up SPEC
- Mother daemon auto-sync — manual `mother sync` for now
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
| Mother v2 Phase 2 schema proposed | `git show spec/mother-v2` (archived) |
| `paths.rs` has no user-layer module | `src/paths.rs` (entire file, no `user_layer`) |
| Project UID exists at `.patina/uid` | `.patina/uid` (value: `2bdc808e`) |
| No `~/.patina/uid` exists | filesystem check — literal `'persona'` as source |
