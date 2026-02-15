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
beliefs:
- beliefs-are-the-product
- belief-identity-is-slug-not-hash
- mother-is-the-daemon
- mother-owns-ref-repo-indexing
---

# feat: Cross-Project Beliefs — Federated Belief Search

> Make `patina scry` find beliefs from user persona AND project, unified.
> Mother v2 Phase 2: the central belief index that links knowledge across projects.

## Problem

120+ project beliefs live in `layer/surface/epistemic/beliefs/`. User persona
beliefs live in `~/.patina/layer/surface/beliefs/`. These two pools are invisible
to each other:

- `patina scry "error handling"` searches project semantic index only
- `patina persona query "error handling"` searches persona JSONL events only
- Neither knows the other exists. No unified belief search.

Per [[beliefs-are-the-product]]: if the belief system can't link knowledge
across projects, it's not doing its job.

## What Exists Today

### Persona write path (`src/commands/persona/mod.rs`)

`note()` appends `PersonaEvent` to `~/.patina/personas/default/events/YYYYMMDD.jsonl`.
Events have UUID-based IDs (`evt_{uuid}`), content, domains, and optional supersedes.
`materialize()` builds `~/.patina/cache/personas/default/persona.db` + `.usearch`
with 768-dim E5 embeddings. `query()` does cosine similarity search on the usearch index.

**Key observation:** Persona uses a completely separate database, separate embedding
pipeline, separate search path. It's a parallel universe to project beliefs.

### Project belief scrape (`src/commands/scrape/beliefs/mod.rs`)

Parses `layer/surface/epistemic/beliefs/*.md` YAML frontmatter + markdown. Computes
rich metrics (citations, evidence, verification, grounding). Stores in `beliefs` table
with FTS5 index (`belief_fts`), evidence table (`belief_evidence`), code reach table
(`belief_code_reach`). Full verification engine runs SQL/assay/temporal queries.

**Key observation:** Project beliefs have rich infrastructure (metrics, verification,
grounding). Persona beliefs have none of this — just raw text + embedding.

### Mother graph (`src/mother/graph.rs`)

`graph.db` at `~/.patina/mother/graph.db`. Schema: `nodes` (projects/refs with
path, domains, importance), `edges` (USES, LEARNS_FROM, TESTS_WITH, SIBLING, DOMAIN
with weight learning), `edge_usage` (feedback loop). No belief-related tables.

### Scry federation (`src/retrieval/engine.rs`)

`query_all_repos()` iterates registered repos and does per-repo semantic search.
Each repo gets its own `query_in_context()` call with temporary cwd change.
Persona is NOT included in `--all-repos` — it's a separate command entirely.

## What To Build

### Phase A: Belief Index in graph.db

Add a `beliefs` table to Mother's `graph.db` that indexes beliefs from all sources.

```sql
-- In ~/.patina/mother/graph.db (alongside existing nodes/edges tables)
CREATE TABLE IF NOT EXISTS beliefs (
    id TEXT NOT NULL,
    source TEXT NOT NULL,          -- 'user', project UID, or ref repo name
    source_path TEXT NOT NULL,     -- file path relative to source root
    statement TEXT NOT NULL,
    entrenchment TEXT DEFAULT 'medium',
    status TEXT DEFAULT 'active',
    facets TEXT,                   -- JSON array of domain tags
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (id, source)
);

CREATE VIRTUAL TABLE IF NOT EXISTS belief_search USING fts5(
    id, statement, facets,
    content='beliefs'
);
```

**Why graph.db, not a new beliefs.db:** graph.db already has the cross-project
infrastructure (nodes, edges). Beliefs are edges in the knowledge graph — they
connect projects via shared principles. Adding a table is simpler than a new DB.

### Phase B: Index Beliefs During Scrape

Modify `patina scrape` to also write to graph.db's beliefs table:

1. After project belief scrape completes, write each belief to graph.db
   with source = project UID (from `.patina/uid`)
2. Add `patina persona materialize` step that writes persona beliefs
   to graph.db with source = 'user'
3. Use existing `~/.patina/uid` or project `.patina/uid` as source identifier

**Code paths to modify:**
- `src/commands/scrape/beliefs/mod.rs` — after inserting into project `beliefs`
  table, also insert into graph.db `beliefs` table
- `src/commands/persona/mod.rs` → `materialize()` — after building persona.db,
  also insert into graph.db

### Phase C: Unified Belief Search in Scry

Add `--content-type beliefs` to scry that queries graph.db instead of per-project DB:

1. FTS5 search on `belief_search` in graph.db
2. Results tagged with source (user vs project name)
3. Works with `--all-repos` flag naturally

**Code path:** `src/retrieval/engine.rs` — add a belief-specific query path
that reads from graph.db when `content_type == "beliefs"`.

## Exit Criteria

1. `patina scrape` indexes project beliefs into graph.db
2. `patina persona materialize` indexes persona beliefs into graph.db
3. `patina scry "error handling" --content-type beliefs --all-repos` returns
   beliefs from both user and project, tagged by source
4. MCP tool `scry` with `content_type: "beliefs"` works identically

## Non-Goals

- Belief embedding/semantic search in graph.db (FTS5 is sufficient for Phase 2)
- Belief promotion (user → project or project → user) — future phase
- Ref repo belief extraction — future phase
- Values/rules system — future phase

## Evidence

| Claim | Source |
|-------|--------|
| Persona writes to JSONL events | `src/commands/persona/mod.rs:76-87` |
| Persona uses separate persona.db + usearch | `src/commands/persona/mod.rs:96-227` |
| Project beliefs have rich metrics | `src/commands/scrape/beliefs/mod.rs:23-67` |
| graph.db has nodes + edges, no beliefs | `src/mother/graph.rs:186-234` |
| Scry all-repos doesn't include persona | `src/retrieval/engine.rs:224-261` |
| Mother v2 Phase 2 schema proposed | `git show spec/mother-v2` (archived) |
