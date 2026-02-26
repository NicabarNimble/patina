---
type: refactor
id: data-architecture-v2
status: draft
created: 2026-02-26
sessions:
  origin: 20260226-065302
  audit: 20260226-094014
  vision: 20260226-102315
beliefs:
  - measure-reads-tables-not-events
  - seq-order-is-not-timestamp-order
  - check-existing-emissions-before-adding
exit_criteria:
  - "events.db is append-only — no DELETE, no UPDATE, never touched by rebuild"
  - "patina.db is fully rebuildable from git + layer/ + events.db"
  - "every tool execution emits an event — no silent operations"
  - "belief grounding chain is traceable: event → evidence → belief"
  - "mother federates 1000s of projects with clean, aligned schemas"
  - "measure answers 'is this project healthy?' with data, not opinion"
  - "sub-specs created and scoped for each implementation area"
---

# refactor: Data Architecture v2

> Build a local-first data architecture — event-sourced, belief-grounded,
> federation-ready — that scales from one project to thousands.

## Vision: A Planned Community

We are building a planned community, not patching a prototype.

A planned community builds infrastructure for who is coming — streets, water,
power — not just for who is here today. The 96 events in our eventlog today
are the first family moving in. The architecture must serve the thousands that
follow.

Modern software is cheap to create. One developer and a few LLMs built all of
patina. That means we can afford to build smart: adopt what the big companies
do with data — event sourcing, CQRS, data mesh, observability — but adapted
for the edge. Not a datacenter with Kafka and Flink. SQLite files on a
developer's laptop. Local-first, sovereign by default, federated when useful.

**What we're building:**

- A **project-level knowledge system** where every meaningful action becomes an
  immutable fact, beliefs evolve with evidence, and an LLM can ask "is this
  project healthy?" and get a data-backed answer.

- A **federation layer** where thousands of projects coordinate through mother
  — each sovereign, each independent, but able to ask "who else has solved this
  problem?" and get real answers from real project data.

- A **belief network** deeply anchored in how and why we've come to our beliefs
  — not static config, but a living epistemic system that evolves as the
  project's reality changes.

## Architectural Principles

### 1. Separate by lifecycle, not by size

Immutable runtime history and derived cache have different integrity
requirements. A rebuild that destroys measurement history is a data design
error regardless of how many events exist. Separate files make data integrity
a construction property — you can't accidentally delete history even with a
bug in rebuild logic.

### 2. Event sourcing at the edge

Every meaningful action produces an immutable event. Current state is always
derivable from replaying events against source files. This is the Netflix /
Confluent pattern — append-only log as ground truth — but with SQLite as the
event store instead of Kafka. At the edge, one file is the event bus.

### 3. CQRS via file separation

The write model (events.db: what happened) is physically separated from the
read model (patina.db: what do we know). Writers append facts. Readers query
materialized views. This is Command Query Responsibility Segregation without
microservices — just two SQLite files with different lifecycle rules.

### 4. Projections are disposable

`rm patina.db && patina scrape` must always produce a correct, complete read
model. If it can't, the architecture is broken. Projections (materialized
views, FTS5 indices, computed tables) are acceleration structures, not truth.
The truth lives in source files (git, layer/) and events.db.

### 5. Capture everything, enforce it

No silent operations. Every scrape, every search, every session, every tool
invocation produces an event. This isn't logging — it's building the project's
memory. The event stream is the raw material that beliefs, health metrics, and
cross-project insights are built from. You can always choose not to query an
event. You can never recover an event you didn't capture.

A principle without enforcement is aspirational. Emission coverage is auditable
— `patina doctor` verifies that every command with side effects has a
corresponding emit call. New scrapers that ship without emissions fail the
health check. This is the mechanism that prevents "capture everything" from
quietly degrading into "capture most things."

### 6. Beliefs need evidence chains

A belief without evidence is an opinion. Every belief should trace back through
a grounding chain: events and code provide evidence, evidence grounds beliefs,
beliefs inform decisions. The chain must be queryable — "why do we believe X?"
should return concrete references, not vibes.

### 7. Federation without authority

Mother coordinates but doesn't control. Each project is sovereign — owns its
data, runs its own belief system, makes its own decisions. Mother provides
discovery ("these 3 projects share this pattern"), cross-pollination ("here's
how project X solved that"), and health aggregation ("your portfolio has 12
contested beliefs"). No project depends on mother to function. Mother depends
on projects to have data.

### 8. LLMs are first-class consumers

The data model is designed for machine queries, not just human CLI output. An
LLM should be able to sit down with a project and ask structured questions:
"What changed since last session?" "Which beliefs are contested?" "Is the test
infrastructure healthy?" "What knowledge is drifting?" Measure is this query
surface — the LLM's API into project health.

### 9. Token-aware data serving

Every byte served to an LLM costs tokens. A 10k-token dump where a 200-token
summary would suffice is not LLM-friendly — it's the equivalent of `SELECT *`
when the caller needed a row count. Every MCP tool and query surface must
practice progressive disclosure: summary first, detail on request.

This means:
- **Orient before dump.** Return structure, counts, and pointers by default.
  Let the LLM decide what to drill into. Scry already does this (`find` →
  `detail` modes). Every tool should.
- **Budget-aware responses.** Tools should know roughly how much context they're
  returning and offer truncation or summarization when the payload is large.
  A spec with 500 lines of body shouldn't arrive as 500 lines — it should
  arrive as frontmatter + section headings + "read section X for detail."
- **Pointers over payloads.** Return file paths, line ranges, and section
  references instead of inlining content. The LLM has a Read tool — let it
  use it when it needs depth.

This isn't just a UX concern — it's an architectural constraint. As the data
grows (thousands of events, hundreds of beliefs, dozens of specs), the tools
that serve this data must scale their responses to the consumer's context
budget, not to the data's volume.

## The Three Systems

### System 1: Events — The Immutable Record

```
┌─────────────────────────────────────────────────────┐
│  events.db — .patina/local/data/events.db           │
│                                                     │
│  WHAT:  Append-only log of runtime facts            │
│  ROLE:  CQRS write model / event store              │
│  RULE:  INSERT only. No DELETE. No UPDATE. Ever.    │
│  SCOPE: Things that happened at runtime that        │
│         cannot be reconstructed from source files   │
└─────────────────────────────────────────────────────┘
```

**What lives here:**

| Domain | Event Types | Source |
|--------|-------------|--------|
| Tool metrics | `measure.capture`, `measure.search`, `measure.index`, `measure.believe`, `measure.evolve` | Tool execution timing, counts, outcomes |
| Search feedback | `scry.query`, `scry.use`, `scry.feedback` | User search behavior, result selection |
| External cache | `forge.issue`, `forge.pr` | GitHub API responses (rate-limited, expensive) |
| Session lifecycle | `session.start`, `session.end` | When work happened, what was accomplished |
| Future: audit trail | `audit.*` | Verification results over time |

**What does NOT live here:**

Source-derived facts. Anything that can be reconstructed by running a scraper
against .git/ or layer/ files belongs in projections, not events. `code.*`,
`git.*`, `pattern.*`, `session.*` (the scraped versions), `belief.surface` —
all derived, all disposable.

**Schema:**

Same eventlog shape as today: `(seq, event_type, session_id, timestamp, data)`.
Events are self-describing — the `data` JSON blob carries all context. Event
types are namespaced (`domain.action`) for filtering and schema evolution.

**Event governance:**

Events are the foundation — if the event catalog drifts, everything built on
it drifts. Three rules govern the event system:

1. **Registry is the spec.** The event type table above IS the canonical
   registry. New event types are added to the spec before they're added to
   code. No ad-hoc event types in production — if it's not in the registry,
   it doesn't ship.

2. **Forward-compatible JSON.** Event `data` blobs evolve additively. New
   fields are added; existing fields are never removed or renamed. Readers
   ignore unknown fields and default missing fields. This means any reader
   can process any historical event without version negotiation.

3. **Doctor-audited coverage.** `patina doctor` checks that every command
   with side effects has a corresponding emit call. This isn't compile-time
   enforcement (too rigid for a tool that evolves fast) or CI gating (patina
   is a dev tool, not a pipeline). It's an always-available audit that makes
   gaps visible and actionable.

**Growth model:**

At steady state for one active project:
- ~360 measure.capture/year (scrapes)
- ~300 measure.search/year (eval/bench sessions)
- ~1000+ scry.query/year (search usage — could dominate)
- ~500+ forge.*/year (API cache updates)
- ~200 session.*/year (session lifecycle)

Conservative: ~2,400 events/year per project. Over years: tens of thousands.
Across a portfolio: multiply accordingly. SQLite handles millions of rows
without issue. This is not a scaling concern — it's a data model concern.

**Invariants:**

- `scrape --rebuild` never opens events.db
- No scraper writes to events.db
- seq is monotonic within events.db
- Events are never modified after insertion
- events.db can be backed up by copying one file

### System 2: Projections — The Queryable Cache

```
┌─────────────────────────────────────────────────────┐
│  patina.db — .patina/local/data/patina.db           │
│                                                     │
│  WHAT:  Materialized views of source + events       │
│  ROLE:  CQRS read model / query acceleration        │
│  RULE:  DELETE + INSERT is correct (cache refresh)  │
│  SCOPE: Anything derivable from .git/ + layer/ +    │
│         events.db — optimized for fast queries      │
└─────────────────────────────────────────────────────┘
```

**What lives here:**

| Domain | Tables | Source |
|--------|--------|--------|
| Code intelligence | function_facts, type_vocabulary, import_facts, call_graph, module_signals, ... (15 tables) | Parsed from source files via tree-sitter |
| Git history | commits, commit_files, co_changes, git_tracked_files, git_tags | Parsed from .git/ |
| Layer content | patterns, milestones, spec_deps | Parsed from layer/*.md |
| Sessions | sessions, observations, goals | Parsed from layer/sessions/*.md |
| Beliefs | beliefs, belief_supports, belief_attacks, belief_code_reach, belief_verifications | Parsed from layer/surface/epistemic/ |
| Search indices | code_fts, commits_fts, pattern_fts, belief_fts | FTS5 over above tables |
| Source-derived eventlog | code.*, git.*, pattern.*, session.*, belief.surface events | Cache of scraper output |

**The key property: full rebuildability.**

```
rm patina.db
patina scrape        # rebuilds everything from .git/ + layer/
                     # events.db is untouched — immutable history preserved
```

This is the test. If deleting patina.db and running scrape doesn't produce a
functionally identical database, the architecture has a bug. Projections are
acceleration structures — they make queries fast, but they're not truth.

**Relationship to events.db:**

Some queries need both. Measure asks "how has scrape performance trended?"
(events.db) while also checking "are beliefs grounded in code?" (patina.db).
These cross-system queries use SQLite ATTACH — events.db mounted read-only
into the patina.db connection when needed.

**Also derived and rebuildable:**

```
┌─────────────────────────────────────────────────────┐
│  embeddings/ — .patina/local/data/embeddings/       │
│                                                     │
│  WHAT:  Vector indices for semantic search          │
│  ROLE:  Acceleration for scry                       │
│  RULE:  Rebuildable via `patina oxidize`            │
└─────────────────────────────────────────────────────┘
```

### System 3: Federation — Mother's Knowledge Network

```
┌─────────────────────────────────────────────────────┐
│  graph.db — ~/.patina/mother/graph.db               │
│                                                     │
│  WHAT:  Cross-project knowledge index               │
│  ROLE:  Federation layer for discovery + insight    │
│  RULE:  Rebuilt from project sources on sync        │
│  SCOPE: Beliefs, patterns, and relationships        │
│         across all registered projects              │
└─────────────────────────────────────────────────────┘
```

**Mother's job:**

A project asks: "Has anyone solved this problem before?"
Mother answers: "Projects X and Y share this belief. Here's how project Z
approached it. Three projects contest this pattern — here's the evidence."

Mother is a data mesh coordinator. She doesn't own the data — projects do.
She indexes it, finds connections, and serves cross-project queries. If mother
disappears, every project still works. If a project disappears, mother loses
that project's contribution but everything else continues.

**What lives in graph.db:**

| Domain | Tables | Source |
|--------|--------|--------|
| Knowledge graph | nodes, edges, edge_usage | Cross-project relationships |
| Belief federation | beliefs, belief_supports, belief_attacks | Synced from project patina.dbs |
| Application tracking | belief_applied_in | Which projects hold which beliefs |
| Search | belief_search (FTS5) | Full-text over federated beliefs |

**Scale target: thousands of projects.**

Each project contributes ~100-300 beliefs, their support/attack relationships,
grounding scores, and health metrics. At 1000 projects:
- ~200K beliefs in graph.db
- SQLite handles this without issue (it's designed for billions of rows)
- Sync is per-project, failure-safe (one project's sync failure doesn't affect others)
- Cross-project queries are standard SQL joins

**What mother provides to projects:**

- **Discovery:** "Which projects share belief X?"
- **Cross-pollination:** "Project Y has a well-grounded belief about this problem"
- **Health aggregation:** "Across your portfolio, 12 beliefs are contested"
- **Edge weight learning:** G2.5 feedback loop learns which cross-project
  connections are actually useful

**What mother needs from projects:**

- Clean, aligned schemas (beliefs table columns must match)
- Grounding and verification data (health_score, grounding_score, verification results)
- Consistent identifiers (belief IDs, project names)

**Identity model:**

A belief's global identity is the composite key `(source, id)` — where
`source` is the project name and `id` is the belief's kebab-case identifier.
`patina:cache-first` and `dojo:cache-first` are different beliefs that happen
to share a local ID. Mother never assumes same-ID means same-belief.

Cross-project discovery uses semantic similarity, not ID matching. When a
project asks "who else has thought about caching?", mother searches by content
(FTS5 + vector similarity), not by ID. This means beliefs don't need
coordinated naming across projects — they need clear, descriptive content.

**Conflict resolution: sovereignty, not reconciliation.**

If project A evolves a belief one way and project B evolves it another way,
both versions coexist in mother. Mother doesn't reconcile — she reports:
"These projects have divergent positions on this topic. Here's the evidence
each one cites." An LLM or human decides what to do with that information.

This is deliberate. Federation without authority means no project can be
overruled by another project's belief evolution. Mother is a mirror, not a
judge. The value is visibility ("you disagree and here's why"), not
enforcement ("project A is right").

## The Belief Network — Connective Tissue

Beliefs are not configuration. They are a living epistemic system — decisions
captured with evidence, evolving as the project's reality changes.

**The grounding chain:**

```
  Events (what happened)
    ↓
  Evidence (code, commits, sessions that reference the belief)
    ↓
  Grounding Score (how deeply anchored is this belief?)
    ↓
  Belief (a decision with evidence weight)
    ↓
  Verification (does the code still match the belief?)
    ↓
  Health Score (is this belief alive, drifting, or dead?)
    ↓
  Evolution (update, contest, or retire the belief)
```

**Beliefs flow across all three systems:**

1. **Events** capture the moments that beliefs are created, applied, contested,
   or verified. These are immutable facts in events.db.

2. **Projections** materialize the belief graph — supports, attacks, code reach,
   grounding scores — for fast querying. An LLM asks "which beliefs are
   contested?" and gets an instant answer from patina.db.

3. **Federation** cross-pollinates beliefs across projects. Mother knows that
   patina and three other projects share a belief about error handling — and
   that one project has stronger evidence for it.

**The belief loop:**

A project makes a decision → captures it as a belief → evidence accumulates
(or doesn't) → verification checks if code matches → health score reflects
reality → LLM notices a drift → project evolves the belief. This is the
core feedback loop of a patina-managed project.

## Measure — The Health Query Surface

Measure is not just a health check. It's the LLM's interface for understanding
whether a project is working. Patina on patina — but also patina on any project.

**Two questions measure answers:**

1. **Does the data help my belief system?** — Are beliefs grounded? Is evidence
   accumulating? Are verifications passing? Which beliefs are drifting?

2. **Are the supporting tools working?** — Are scrapers running? Is the event
   stream flowing? Are search indices fresh? Is mother syncing?

**What measure reads:**

| Source | Questions Answered |
|--------|-------------------|
| events.db | Tool performance trends, scrape frequency, search usage patterns |
| patina.db beliefs | Grounding scores, verification results, health distribution |
| patina.db sessions | Activity recency, session classification breakdown |
| patina.db code | Module coverage, function count, code staleness |
| patina.db git | Commit frequency, active contributors, file churn |
| graph.db | Federation health, cross-project belief alignment, dangling edges |

**Measure as LLM API:**

An LLM sits down with a project and calls `patina measure --full`. It gets
back a structured JSON snapshot — organized by domain, not by table — that
answers: "Here's the state of this project's knowledge system. Here's what's
healthy, here's what's drifting, here's what needs attention."

This is the observability pattern from SRE applied to knowledge management.
Events flow in, health metrics flow out, an LLM reasons about what's working.

## Data Flow

```
 SOURCES (immutable, external)          RUNTIME (tool execution)
 ┌──────────────────────────┐           ┌──────────────────────┐
 │ .git/    — history       │           │ scrape   → measure.* │
 │ layer/   — knowledge     │           │ eval     → measure.* │
 │ GitHub   — forge         │           │ scry     → scry.*    │
 │ src/     — code          │           │ session  → session.* │
 └────────────┬─────────────┘           │ forge    → forge.*   │
              │                         │ oxidize  → measure.* │
              │                         │ audit    → audit.*   │
              │                         └──────────┬───────────┘
              │                                    │
              ▼                                    ▼
 ┌──────────────────────┐           ┌──────────────────────────┐
 │   patina scrape      │           │      events.db           │
 │   (source → cache)   │           │   (append-only facts)    │
 └──────────┬───────────┘           └──────────┬───────────────┘
            │                                  │
            ▼                                  │
 ┌──────────────────────┐                      │
 │     patina.db        │◄─── ATTACH (read) ───┘
 │  (queryable cache)   │
 │  + embeddings/       │
 └──────────┬───────────┘
            │
            ▼
 ┌──────────────────────┐        ┌──────────────────────┐
 │     measure          │        │     mother            │
 │  (health surface)    │        │  (federation layer)   │
 │  "is it working?"    │        │  "who else knows?"    │
 └──────────────────────┘        └──────────────────────┘
```

## Current State vs Target

### What exists and works

- **eventlog table** in patina.db — holds both runtime and source-derived events
- **60+ tables** of projections — code, git, layer, sessions, beliefs, forge
- **5 FTS5 indices** — code, commits, patterns, beliefs, eventlog
- **measure command** — reads from tables, computes health across 5 protocol verbs
- **mother graph sync** — federates beliefs across projects
- **Scrape pipeline** — source-derived projections rebuild correctly
- **Belief system** — 164 beliefs with supports/attacks/grounding/verification

### What's broken or missing

**Architecture:**
- events.db doesn't exist — immutable and derived data share one file
- `execute_rebuild()` destroys runtime history along with cache
- No structural guarantee against accidental event deletion

**Event capture (8 gaps):**

| Missing Emission | Impact |
|-----------------|--------|
| scrape git | No record of git scrape runs — can't trend performance |
| scrape layer | No record of layer scrape runs |
| scrape beliefs | No record of belief scrape runs |
| scrape forge | No record of forge scrape runs |
| session start/end | No lifecycle events — can't measure session patterns |
| context command | No record of context queries |
| assay command | No record of structural queries |
| scry without session | scry.query requires session_id — drops events outside sessions |

**Mother schema drift:**

| Column | patina.db | graph.db | Gap |
|--------|-----------|----------|-----|
| grounding_score | Yes | No | Cannot rank by evidence quality |
| grounding_*_count | Yes | No | Cannot show evidence breakdown |
| verification_* | Yes | No | Cannot show verification health |
| last_activity | Yes | No | Cannot detect stale beliefs |

Plus: 71 dangling edges (warning-only, not cleaned), belief_applied_in
populated but never queried, FTS5 doesn't rank by health.

**Measure query surface:**
- No `--full` JSON output for LLM consumption
- measure reads from patina.db tables, not from events.db (because events.db
  doesn't exist yet)
- Some health computations use table state, which conflates "what we know now"
  with "what happened over time"

## Implementation Areas (Sub-Specs)

This vision spec establishes the framing. Implementation details go into
focused sub-specs, each independently shippable:

### Area 1: Database Split

**Scope:** Create events.db, separate runtime event writers from source-derived
scrapers, implement ATTACH for cross-system queries, migrate existing runtime
events, update rebuild to leave events.db untouched.

**Why first:** Everything else depends on the fundamental data separation
being in place. You can't fix emission gaps into a database that gets deleted
on rebuild.

**Estimated touch:** ~7 files (runtime event writers + measure reader)

### Area 2: Emission Completeness

**Scope:** Wire measure.capture into all scrapers (git, layer, beliefs, forge).
Add session lifecycle emissions. Fix scry to log without session_id. Add
context and assay emissions.

**Why second:** Once events.db exists, we need to fill it. Every silent
operation is lost project memory.

**Estimated touch:** ~8 files (scrapers + session + scry + context + assay)

### Area 3: Mother Schema Alignment

**Scope:** Add grounding and verification columns to graph.db beliefs table.
Sync these during graph sync. Clean dangling edges automatically. Wire
belief_applied_in into search results. FTS5 ranking by health.

**Why parallel-safe:** Mother schema work is independent of the events.db
split. Can proceed in parallel with Areas 1-2.

**Estimated touch:** ~3 files (mother/graph.rs + sync pipeline)

### Area 4: Measure as LLM Query Surface

**Scope:** `patina measure --full` returns domain-organized JSON. Measure reads
from events.db (tool trends) + patina.db (belief health, code coverage,
session activity). Designed for LLM consumption — structured, comprehensive,
machine-parseable.

**Why after split:** measure needs to ATTACH events.db. The query surface
design depends on knowing what data is available in each system.

### Area 5: Fast Incremental + Hooks

**Scope:** Optimize incremental scrape (especially co-change O(n²) for git).
Git hook integration for automatic scrape on commit. Target: <2s for
incremental scrape after a commit.

**Why last:** Performance optimization after the architecture is correct.
Hooks depend on scrape being fast enough to not annoy developers.

## Design Decisions

### Why event sourcing at the edge?

Event sourcing is typically associated with distributed systems (Kafka, EventStore).
But the core insight — immutable event log as ground truth, derived state from
replay — is orthogonal to scale. SQLite gives us ACID, single-writer simplicity,
and zero operational overhead. One file is the event bus. One file is the
read model. `cp events.db events.db.backup` is disaster recovery.

### Why not a single protected table?

The alternative: keep one patina.db, mark eventlog rows as "protected" during
rebuild. This fails for two reasons:

1. **Ongoing maintenance cost.** Every new event type needs a WHERE clause in
   rebuild. Miss one and history is gone. Separate files make protection
   structural — rebuild literally cannot touch events.db because it's a
   different file.

2. **Wrong abstraction.** "Protected rows in a shared table" treats the
   problem as a query filtering issue. It's actually a lifecycle issue —
   these data have different lifetimes, different write patterns, and
   different integrity requirements.

### Why capture everything?

In the age of LLMs, data you didn't capture is data your AI can't reason
about. The cost of emitting an event is near zero (one SQLite INSERT). The
cost of not having the event when an LLM asks "how has scrape performance
changed over the last month?" is the difference between a data-backed answer
and a guess. Capture is cheap. Hindsight is expensive.

### Why local-first federation?

Each project must work without mother. Mother must work without any individual
project. This isn't a philosophical preference — it's a reliability requirement.
A developer on a plane with no internet should have full patina functionality.
Federation adds value (cross-pollination, discovery) without creating
dependency.

### Why LLMs as first-class consumers?

Every `patina measure` call, every `patina context` call, every `patina scry`
call is an LLM asking a question about the project. The data model should be
designed for these questions. JSON output, domain organization, structured
health metrics — not human-formatted tables that an LLM has to parse.

## Schema Reference (Current State)

### patina.db (60+ tables, 83MB)

```
CORE:           eventlog, scrape_meta, moments
CODE (15):      function_facts, type_vocabulary, import_facts, constant_facts,
                member_facts, call_graph, code_search, index_state,
                module_signals, skipped_files
GIT (5):        commits, commit_files, co_changes, git_tracked_files, git_tags
LAYER (3):      patterns, milestones, spec_deps
SESSIONS (3):   sessions, observations, goals
BELIEFS (5):    beliefs, belief_supports, belief_attacks,
                belief_code_reach, belief_verifications
FORGE (3):      forge_issues, forge_prs, forge_refs
FTS5 (5):       code_fts, commits_fts, pattern_fts, belief_fts, eventlog_fts
VIEWS (6):      feedback_session_queries, feedback_commit_files,
                feedback_query_hits, feedback_usage, feedback_ratings,
                feedback_query_analysis
```

### mother/graph.db (10 tables, 463KB)

```
GRAPH:          nodes, edges, edge_usage
BELIEFS:        beliefs, belief_supports, belief_attacks,
                belief_applied_in, belief_search (FTS5)
```

## References

- Session 20260225-221415 — Measure edge-finding, 4 bugs fixed
- Session 20260226-065302 — Rebuild resilience, 3 fixes, spec drafted
- Session 20260226-094014 — Deep audit, spec revised with corrected framing
- Session 20260226-102315 — Architectural vision rewrite
- Belief: [[measure-reads-tables-not-events]]
- Belief: [[seq-order-is-not-timestamp-order]]
- Belief: [[check-existing-emissions-before-adding]]
