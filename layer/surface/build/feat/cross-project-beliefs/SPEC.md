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

# feat: Cross-Project Knowledge — Federated Belief & Value Search

> Mother knows what every project believes and what the user values.
> Projects remain islands. Mother v2 Phase 2.

## Knowledge Model

Two kinds of epistemic knowledge, distinct in scope and grounding:

**Beliefs** are project-scoped assertions with project-scoped evidence. "This
codebase prefers Result types for error handling" — grounded in commits, code
patterns, and session history within one project. Beliefs live in
`layer/surface/epistemic/beliefs/` (git-tracked, per-project).

**Values** are cross-project principles held by a persona. "I prefer explicit
error handling across all projects" — grounded across projects (which projects
apply this? what evidence from multiple codebases?). Values live in
`~/.patina/layer/surface/beliefs/` (machine-local, per-user).

**Persona** is architecturally distinct from user. A user could have multiple
personas with different values (e.g., "rust-architect" vs "quick-prototyper").
For now user = persona (1:1), but the name slot matters. Persona separation
and multi-persona support are future scope.

Values need grounding just as much as beliefs — but cross-project grounding,
not single-project evidence. That grounding infrastructure is future scope;
this SPEC makes values discoverable, not verifiable.

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

You're in project-A, talking to an LLM via adapter. The conversation surfaces
a topic — error handling, testing patterns, module boundaries. The LLM asks
Mother: "do other projects or the user's persona have knowledge about this?"

Mother searches her index and returns beliefs AND values together:

```
[project-B]  explicit-error-types     belief  (high entrenchment, 3 evidence)
[persona]    prefer-result-over-panics value   (low entrenchment)
[ref:beads]  errors-are-values         belief  (extracted)
```

The user decides: adopt, reject, or tweak. If adopted, a new belief is created
in **project-A's layer** — a native project belief, possibly with a `sourced-from`
reference. The cross-project link lives in **Mother's graph**, not in either project.

**Key principle:** Projects are islands. Mother is the overlay that sees across
islands and brokers introductions. Projects never write into Mother. Mother never
writes into projects. Per [[mother-owns-ref-repo-indexing]] and [[mother-is-the-daemon]].

## What Exists Today

### Persona values (`~/.patina/layer/surface/beliefs/`)

Phase 1 (mother-v2) created the user-layer directory structure and migrated 5
persona notes to markdown value files. These files use the same format as project
beliefs (YAML frontmatter + markdown sections) but simpler — no verification
queries, no project-scoped grounding metrics. Example:
`prefer-result-t-e-over-panics-for-error-handling.md`.

**Important:** These markdown files are the **truth** for persona values going
forward. The legacy JSONL events in `~/.patina/personas/default/events/` are the
migration source but not the ongoing write target. No code path currently reads
these markdown files for indexing — that's what this SPEC builds.

**Directory naming:** The directory is `beliefs/` (Phase 1 artifact). A future
SPEC should rename to `values/` — but that's a separate scope. For now, Mother
distinguishes by source field, not directory name.

### Legacy persona path (`src/commands/persona/mod.rs`)

`note()` appends `PersonaEvent` to `~/.patina/personas/default/events/YYYYMMDD.jsonl`.
`materialize()` builds `~/.patina/cache/personas/default/persona.db` + `.usearch`
with 768-dim E5 embeddings. `query()` does cosine similarity search on the usearch index.

**Key observation:** This is the legacy path. `persona note` still writes JSONL only —
it does NOT write markdown value files. A future SPEC should migrate `persona note`
to write `~/.patina/layer/surface/beliefs/` directly, replacing JSONL entirely.
The `persona query` and `persona materialize` commands are superseded by
`mother search` and `mother sync` once this SPEC ships.

### Project belief scrape (`src/commands/scrape/beliefs/mod.rs`)

Parses `layer/surface/epistemic/beliefs/*.md` YAML frontmatter + markdown. Computes
rich metrics (citations, evidence, verification, grounding). Stores in `beliefs` table
with FTS5 index (`belief_fts`), evidence table (`belief_evidence`), code reach table
(`belief_code_reach`). Full verification engine runs SQL/assay/temporal queries.

**Key observation:** Project beliefs have rich project-scoped infrastructure (metrics,
verification, grounding). Persona values have none of this — just markdown files on
disk. Mother's index doesn't need per-project metrics — statement, entrenchment,
facets, and status are sufficient for cross-project discovery.

### Mother graph (`src/mother/graph.rs`)

`graph.db` at `~/.patina/mother/graph.db`. Schema: `nodes` (projects/refs with
path, domains, importance), `edges` (USES, LEARNS_FROM, TESTS_WITH, SIBLING, DOMAIN
with weight learning), `edge_usage` (feedback loop). No belief-related tables.

### Scry federation (`src/retrieval/engine.rs`)

`query_all_repos()` iterates registered repos and does per-repo semantic search.
Each repo gets its own `query_in_context()` call with temporary cwd change.
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
    kind TEXT NOT NULL,            -- 'belief' (project-scoped) or 'value' (persona-scoped)
    statement TEXT NOT NULL,
    entrenchment TEXT DEFAULT 'medium',
    status TEXT DEFAULT 'active',
    facets TEXT,                   -- JSON array of domain tags
    last_indexed TEXT NOT NULL,
    PRIMARY KEY (id, source)
);

-- Regular FTS5 table (not external content) — Mother owns her own copy of the
-- searchable text. Avoids trigger complexity of content-linked FTS5.
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_search USING fts5(
    id, source, kind, statement, facets,
    tokenize='porter unicode61'
);
```

**`kind` column:** Distinguishes project beliefs from persona values. Both need
grounding but grounding is scoped differently — beliefs ground in one project's
code/commits/sessions, values ground across projects (future: which projects
apply this value? how many codebases show evidence?). The `kind` column lets
future queries filter or weight by type.

**Why graph.db, not a separate beliefs.db:** Mother-v2 originally proposed a
separate `beliefs.db`. We use graph.db because knowledge entries are nodes in
the graph — they connect projects via shared principles, and future phases will
add edges (adopted-from, contradicts) between entries and project nodes. A single
DB means these joins are local, not cross-database. The tradeoff: you can't wipe
knowledge without touching graph edges. Acceptable — `mother sync` is idempotent
and rebuilds the knowledge table from project sources.

**Why no embedding column:** Cross-project search is exploratory ("which projects
discuss error handling?"), not precision-critical. Per-project `scry --belief`
already handles vector-precision grounding. FTS5 keyword matching is sufficient
for cross-project discovery. Adding embeddings would require a cross-project
oxidize pipeline — Phase 3+ scope per [[mother-owns-ref-repo-indexing]].

### Phase B: Mother Sync — Pull From Project Islands

Mother indexes knowledge by **pulling** from project databases and persona value
files. Projects never write into Mother. `patina scrape` stays project-pure.

**New command:** `patina mother sync`

1. Walk registered projects from `~/.patina/registry.yaml`
2. For each project, open its `.patina/local/data/patina.db`, read the `beliefs`
   table (id, statement, entrenchment, status, facets). Insert into `knowledge`
   with source = project UID (from `.patina/uid`), kind = `'belief'`.
3. Read `~/.patina/layer/surface/beliefs/*.md` — parse YAML frontmatter for
   id, statement, entrenchment, status, facets. Insert into `knowledge` with
   source = `'persona'` (literal string; no `~/.patina/uid` exists or is needed),
   kind = `'value'`.
4. Clear and rebuild graph.db's `knowledge` + `knowledge_search` tables (idempotent).

**Code paths:**
- `src/commands/mother/mod.rs` — add `sync` subcommand
- `src/mother/graph.rs` — add `sync_knowledge()` method on `Graph`
- `src/paths.rs` — add `user_layer::beliefs_dir()` for `~/.patina/layer/surface/beliefs/`

**Data flow direction:** Mother reaches down to read from project islands.
Projects don't know or care this is happening. Per [[mother-owns-ref-repo-indexing]]:
"projects are the door, mother is the house."

### Phase C: Cross-Project Knowledge Search

Mother exposes knowledge search as a CLI command and MCP tool. This is a Mother
command, not a scry extension — scry searches project knowledge, Mother searches
across projects and persona.

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

- Embedding/semantic search in graph.db — FTS5 is sufficient for cross-project
  discovery; per-project `scry --belief` handles vector precision
- Belief adoption workflow (create project belief from Mother search result) —
  natural next step, follow-up SPEC
- Migrating `persona note` to write markdown value files — future SPEC;
  current JSONL path is legacy but functional
- Cross-project grounding for values (which projects apply this value?) —
  future phase; this SPEC makes values discoverable, not verifiable
- Persona separation (multiple personas per user) — future phase
- Multi-user project influence (per-user values at project level) — future phase
- Ref repo belief extraction — future phase
- Values-to-rules system — future phase
- Mother daemon auto-sync (manual `mother sync` for now) — Phase 3+

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
