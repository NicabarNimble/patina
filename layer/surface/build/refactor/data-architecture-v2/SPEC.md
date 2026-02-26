---
type: refactor
id: data-architecture-v2
status: ready
created: 2026-02-26
blocked_by:
- data-db-split
- data-emission-completeness
- data-mother-schema
sessions:
  origin: 20260226-065302
beliefs:
- measure-reads-tables-not-events
- seq-order-is-not-timestamp-order
- check-existing-emissions-before-adding
- if-its-patina-its-git
- events-are-autobiography-not-telemetry
- beliefs-are-where-machine-meets-human
exit_criteria:
- id: events-db-is-append-only-no-delete-no-update-never-touched-by-rebuild
  text: events.db is append-only — no DELETE, no UPDATE, never touched by rebuild
  checked: false
- id: patina-db-is-fully-rebuildable-from-git-layer-events-db
  text: patina.db is fully rebuildable from git + layer/ + events.db
  checked: false
- id: every-tool-execution-emits-an-event-no-silent-operations
  text: every tool execution emits an event — no silent operations
  checked: false
- id: belief-grounding-chain-is-traceable-event-evidence-belief
  text: 'belief grounding chain is traceable: event → evidence → belief'
  checked: false
- id: each-implementation-area-has-a-sub-spec-with-concrete-testable-exit-criteria
  text: each implementation area has a sub-spec with concrete, testable exit criteria
  checked: false
- id: measure-answers-is-this-project-healthy-with-data-not-opinion
  text: measure answers 'is this project healthy?' with data, not opinion
  checked: false
- id: sub-specs-created-and-scoped-for-each-implementation-area
  text: sub-specs created and scoped for each implementation area
  checked: false
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

### 7. Mother is core, projects are sovereign

There is no patina without mother. Mother is part of every patina installation
— not optional infrastructure you add later. She manages federation across
projects, the connector registry for external data sources, and reference
repos. One project or a thousand, mother is there.

But projects are sovereign. Each project owns its data, runs its own belief
system, makes its own decisions. Mother provides discovery ("these 3 projects
share this pattern"), cross-pollination ("here's how project X solved that"),
and health aggregation ("your portfolio has 12 beliefs contested"). A project
functions fully even if mother's data is stale or empty — like a computer
with a network card works offline.

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

Scry demonstrates this today: `find` mode returns summaries and pointers,
`detail` mode returns full content for a single result. This is the reference
implementation — other tools adopt progressive disclosure as they're built
or revised.

This isn't just a UX concern — it's an architectural constraint. As the data
grows (thousands of events, hundreds of beliefs, dozens of specs), the tools
that serve this data must scale their responses to the consumer's context
budget, not to the data's volume.

## Source Model

A patina project has two sources of truth with different lifecycles.

**Git is the source of record for what a project declares.** Code, prose,
knowledge files, beliefs, specs, sessions — whatever the project contains
or decides, it lives in git. Git is the declaration layer.

**events.db is the source of record for what a project experiences.** Search
behavior, belief lifecycle moments, tool execution, decisions, API cache —
whatever happens when patina runs, it lives in events.db. This is runtime
knowledge that git cannot capture: an agent searching "error handling" 200
times, a belief surviving 3 contestations, scrape performance degrading over
a month. events.db is the experience layer.

**Together they are the complete truth. Everything else is derived.**

```
  SOURCES OF TRUTH (irreplaceable):
  git repo (local)           = what the project declares
  events.db                  = what the project experiences

  DERIVED (rebuildable from the above):
  patina.db                  = structured projections for fast queries
  embeddings/                = semantic proximity for search + grounding
  graph.db                   = cross-project knowledge index
```

**External data sources are accessed through connectors** — WASM plugins that
know how to fetch from and push to external APIs. Forge (GitHub) is the first
connector. All connectors follow the same pattern: fetch → cache as events
in events.db → scrape into Layer 1 for querying.

**Mother manages all connectors.** Connector registry, credentials, rate
limits, shared access across projects — that's mother's job. A project
doesn't install connectors — it tells mother which data sources it needs,
and mother provides the pipe. Connectors add material. The data stack
doesn't change regardless of how many connectors a project uses.

## The Data Stack

Four layers of understanding, one irreplaceable, three derived. Each layer
builds on the ones below it. The belief layer at the top reaches down into
all layers for evidence, while all layers push patterns up into beliefs.
The stack is circular — not just bottom-up.

```
  Layer 0: Events        (what happened — autobiography)       IRREPLACEABLE
  Layer 1: Structured    (what we parsed — tables, graphs)     REBUILDABLE
  Layer 2: Semantic      (what things mean — vectors)          REBUILDABLE
  Layer 3: Beliefs       (what we understand — emergent        REBUILDABLE
                          + decided)
                │
                │  beliefs ground INTO layers 0-2 for evidence
                │  layers 0-2 feed UP into beliefs
                │  the loop IS the intelligence
```

### Layer 0: Events — The Project's Autobiography

```
┌─────────────────────────────────────────────────────┐
│  events.db — .patina/local/data/events.db           │
│                                                     │
│  WHAT:  Append-only log of everything meaningful    │
│  ROLE:  CQRS write model / event store              │
│  RULE:  INSERT only. No DELETE. No UPDATE. Ever.    │
│  SCOPE: Moments in time — ops, epistemic,           │
│         lifecycle, decisions, discoveries            │
└─────────────────────────────────────────────────────┘
```

events.db is the project's autobiography. Not ops telemetry — **everything
meaningful that happens to the project**, captured as immutable facts.

**What lives here:**

| Domain | Event Types | Status | Source |
|--------|-------------|--------|--------|
| Tool metrics | `measure.capture`, `measure.search`, `measure.index`, `measure.believe`, `measure.evolve` | Active | Tool execution timing, counts, outcomes |
| Search feedback | `scry.query`, `scry.use`, `scry.feedback` | Active | User search behavior, result selection |
| External cache | `forge.issue`, `forge.pr` | Active | GitHub API responses (rate-limited, expensive) |
| Session lifecycle | `session.start`, `session.end` | Planned | When work happened, what was accomplished |
| Epistemic | `belief.created`, `belief.contested`, `belief.supported`, `belief.verified`, `belief.evolved`, `belief.retired` | Planned | Belief lifecycle — the moments decisions were made, challenged, and changed |
| Spec lifecycle | `spec.promoted`, `spec.completed`, `spec.paused`, `spec.abandoned` | Planned | Spec state transitions — the journey from idea to shipped |
| Decisions | `decision.made` | Planned | Choices captured with reasoning and alternatives considered |
| Discovery | `discovery.pattern`, `discovery.cross_project` | Planned | Machine or human insight moments |
| Future: audit trail | `audit.*` | Future | Verification results over time |

**Status key:** Active = emitter exists in code today. Planned = defined and
scoped, emitter to be wired in Area 2 (data-emission-completeness). Future =
conceptual, no sub-spec yet.

**The boundary: moments vs current state.**

The event log captures *when and why*. Files and tables capture *what is now*.
A belief markdown file is the current state — what the belief says today.
The `belief.created` event is the moment — when it was proposed, what evidence
triggered it, what session context surrounded it. The file is a projection
of the belief's latest state. The events are its history.

**What does NOT live here:**

Source-derived parse output. Running a scraper against .git/ or layer/ files
produces structured data (code facts, commit tables, pattern records) — that's
Layer 1 projection work. `code.*`, `git.*`, `pattern.*` are derived,
disposable, and belong in patina.db.

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
- ~500+ belief.*/year (epistemic events — creation, verification, evolution)
- ~200+ spec.*/year (spec lifecycle transitions)
- ~100+ decision.*/year (captured choices)
- ~100+ discovery.*/year (pattern recognition moments)

Conservative: ~3,500+ events/year per active project. Over years: tens of
thousands. This is the project's memory — every event is a fact the system
can reason about. SQLite handles millions of rows without issue.

**Durability:**

events.db is the source of truth for runtime knowledge. It is not a cache,
not a materialization of something in git — it IS the thing. WAL mode,
`synchronous=FULL`, crash-safe by default.

`layer/events.jsonl` is a **replica** for disaster recovery — a git-tracked
append-only JSONL file where each line is one event. On session end and
scrape, new events since last export are appended and committed. If events.db
is lost (machine failure, accidental deletion), the replica enables recovery:
`patina events import layer/events.jsonl`. Loss window: events since last
export (typically hours).

The JSONL is a backup, not the source. events.db feeds the JSONL, not the
other way around. This follows Schickling's model: SQLite is the runtime
source of truth, the export is a replica for durability across machine
boundaries.

**Invariants:**

- `scrape --rebuild` never opens events.db
- No scraper writes to events.db
- seq is monotonic within events.db
- Events are never modified after insertion
- events.db can be backed up by copying one file
- `layer/events.jsonl` is a git-tracked replica for disaster recovery
- Doctor audits replica freshness: gap between events.db and JSONL export

### Layer 1: Structured — The Queryable Cache

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
                     # events.db is untouched — project memory preserved
```

This is the test. If deleting patina.db and running scrape doesn't produce a
functionally identical database, the architecture has a bug. Layer 1 is an
acceleration structure — it makes queries fast, but it's not truth.

**Relationship to events.db:**

Some queries need both. Measure asks "how has scrape performance trended?"
(events.db) while also checking "are beliefs grounded in code?" (patina.db).
These cross-system queries use SQLite ATTACH — events.db mounted read-only
into the patina.db connection when needed.

### Layer 2: Semantic — What Things Mean

```
┌─────────────────────────────────────────────────────┐
│  embeddings/ — .patina/local/data/embeddings/       │
│                                                     │
│  WHAT:  Dense vector representations of entities    │
│  ROLE:  Semantic similarity for search + grounding  │
│  RULE:  Rebuildable via `patina oxidize`            │
│  SCOPE: Vectors for code, commits, patterns,        │
│         beliefs — "what is near what" in meaning    │
└─────────────────────────────────────────────────────┘
```

Layer 2 takes the structured data from Layer 1 and computes what things
*mean* — not just what they contain. "This belief is semantically near that
code region" is a Layer 2 insight. "These two beliefs are 0.87 similar" is
Layer 2. It enables scry's semantic search and feeds into belief grounding.

**What lives here today:**

```
embeddings/
├── code.usearch        ← function/type embeddings
├── commits.usearch     ← commit message embeddings
├── patterns.usearch    ← pattern embeddings
├── beliefs.usearch     ← belief embeddings
└── projections/        ← dimensionality reduction matrices
```

Built by `patina oxidize` via ONNX Runtime (`ort` crate). All rebuildable —
delete the directory and re-oxidize.

**Why it's its own layer:**

Dense vectors are opaque. "0.87 similar" doesn't tell you *why*. Layer 2
provides proximity but not interpretability. That's what Layer 3 adds.

**Layer 2 feeds up:** Semantic proximity is evidence for belief grounding.
If a belief about error handling is semantically near 47 functions that do
error handling, that's grounding signal — even if no human explicitly linked
them.

### Layer 3: Beliefs — Where Understanding Lives

```
┌─────────────────────────────────────────────────────┐
│  The belief layer lives across multiple stores:     │
│                                                     │
│  layer/surface/epistemic/beliefs/*.md               │
│    → human-readable projections (git-tracked)       │
│  patina.db beliefs tables                           │
│    → queryable graph (supports, attacks, grounding) │
│  embeddings/beliefs.usearch                         │
│    → semantic proximity for discovery               │
│  events.db belief.*                                 │
│    → lifecycle history (created, evolved, retired)  │
│                                                     │
│  RULE:  Beliefs are the integration point —         │
│         not a single store but a cross-layer        │
│         concern that reaches into everything        │
└─────────────────────────────────────────────────────┘
```

The belief layer is where machine understanding meets human understanding.
It holds two kinds of knowledge at different stages of maturity:

**Named beliefs** — decisions a human or LLM has captured explicitly:
- "We use result types for error handling" — grounded in 47 functions
- Created with evidence, verified against code, evolved over time
- Have support/attack relationships, health scores, grounding chains
- Stored as markdown files (human view) + table rows (query view) + events (history)

**Proto-beliefs** — patterns the system has detected but nobody has named:
- SAE feature 47 activates on retry/fallback code across 12 modules
- Concept cluster: these 8 beliefs share an unnamed theme
- Anomaly: this code region has no beliefs but high churn
- Discovered computationally, surfaced for human/LLM recognition

**The maturity pipeline:**

```
  EMERGENT                                    NAMED
  (machine-discovered)                        (human/LLM-decided)

  SAE feature detected ─→ pattern surfaced ─→ "defensive-coding"
  concept cluster found ─→ LLM names it    ─→ belief with evidence
  anomaly flagged       ─→ human reviews   ─→ new belief or dismiss
                                               │
  proto-belief                                 belief
         └──────────── SAME LAYER ────────────┘
```

A proto-belief that gets named becomes a belief. A belief that loses all its
evidence becomes contested. A contested belief that fails verification gets
retired. The lifecycle is continuous.

Proto-belief detection is a future capability — no sub-spec exists for this
work. The architecture accommodates it: new computational inputs (SAEs, graph
analysis, anomaly detection) feed into Layer 3 without adding layers or
changing the belief lifecycle.

**Beliefs reach DOWN into all layers for evidence:**

| Layer | Evidence Type | Example |
|-------|---------------|---------|
| Layer 0: Events | Lifecycle history | "This belief was created in session X, contested in session Y" |
| Layer 1: Structured | Code reach, commit refs | "47 functions implement this pattern, referenced in 12 commits" |
| Layer 2: Semantic | Vector proximity | "Semantically near 23 code regions about error handling" |
| Layer 3: Beliefs | Support/attack network | "Supported by 3 beliefs, attacked by 1" |

The more layers provide evidence, the deeper the grounding. A belief with
only Layer 3 evidence (supported by other beliefs) is an echo chamber. A
belief with Layer 0 + 1 + 2 evidence is deeply anchored in the project's
reality.

**All layers push UP into beliefs:**

Layer 1 discovers a code pattern → surfaces as proto-belief.
Layer 2 finds a semantic cluster → surfaces as proto-belief.
Layer 0 records a decision event → becomes a named belief.
Each new computational input (SAEs, graph analysis, whatever comes next)
becomes a new **source of proto-beliefs** feeding into the same layer.

**Extensibility:**

Adding new computational methods (sparse autoencoders, graph neural networks,
anomaly detection) doesn't add new layers. It adds new inputs to Layer 3.
The belief layer is the integration point — the place where all forms of
understanding converge into actionable knowledge.

**The belief loop:**

```
  A decision is made ─→ captured as belief + event
       │
       ▼
  Evidence accumulates from Layers 0-2
       │
       ▼
  Verification checks: does code still match?
       │
       ▼
  Health score reflects reality
       │
       ▼
  LLM notices drift ─→ evolves belief + emits event
       │
       └──────── back to events.db ────────┘
```

## Federation — Cross-Cutting System

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

Mother is core to patina — every installation has her. She is not a layer in
the data stack but a cross-cutting system that primarily operates at Layer 3
(beliefs) and could evolve to sync events (Layer 0) and semantic features
(Layer 2) across projects.

**Mother's three jobs:**

1. **Federation** — cross-project knowledge coordination. A project asks:
   "Has anyone solved this problem before?" Mother answers: "Projects X and Y
   share this belief. Here's how project Z approached it."

2. **Connector management** — registry, credentials, and rate limiting for
   WASM connector plugins that access external data sources. Forge (GitHub)
   is connector #1. Future connectors follow the same pattern, managed in
   one place.

3. **Reference repos** — external repositories that projects learn from.
   Mother manages the catalog so individual projects don't duplicate effort.

Mother is a coordinator, not an authority. She doesn't own the data — projects
do. She indexes it, finds connections, manages shared infrastructure, and
serves cross-project queries. Projects function fully without mother's data.
Mother functions without any individual project.

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

**Future: operation-level federation.**

Today mother syncs snapshots (current belief state). With epistemic events
in events.db, mother could evolve to sync operations — the history of how
a belief evolved, not just its current state. This would let cross-project
queries answer "how did project X arrive at this belief?" not just "what
does project X believe?"

## Measure — The Health Query Surface

Measure is not just a health check. It's the LLM's interface for understanding
whether a project is working. Patina on patina — but also patina on any project.

**Two questions measure answers:**

1. **Does the data help my belief system?** — Are beliefs grounded? Is evidence
   accumulating? Are verifications passing? Which beliefs are drifting?

2. **Are the supporting tools working?** — Are scrapers running? Is the event
   stream flowing? Are search indices fresh? Is mother syncing?

**What measure reads (all 4 layers):**

| Layer | Questions Answered |
|-------|-------------------|
| Layer 0: events.db | Tool performance trends, belief lifecycle patterns, decision frequency, scrape cadence |
| Layer 1: patina.db | Code coverage, git activity, belief graph health, session patterns |
| Layer 2: embeddings | Index freshness, coverage gaps, semantic drift detection |
| Layer 3: beliefs | Grounding depth, proto-belief backlog, contested beliefs, verification pass rates |
| Federation: graph.db | Cross-project alignment, dangling edges, portfolio health |

**Measure as LLM API:**

An LLM sits down with a project and calls `patina measure --full`. It gets
back a structured JSON snapshot — organized by domain, not by table — that
answers: "Here's the state of this project's knowledge system. Here's what's
healthy, here's what's drifting, here's what needs attention."

This is the observability pattern from SRE applied to knowledge management.
Events flow in, health metrics flow out, an LLM reasons about what's working.

## Data Flow

```
 ┌──────────────────────────────────────────────────────────────┐
 │  MOTHER (always present)                                     │
 │  ┌────────────────────────────────────────────────────────┐  │
 │  │ federation │ connector registry │ reference repos      │  │
 │  └────────────┼───────────────────┼───────────────────────┘  │
 └───────────────┼───────────────────┼──────────────────────────┘
                 │                   │
                 │            ┌──────┴──────┐
                 │            │ connectors  │
                 │            │ (WASM)      │
                 │            │ forge, ...  │
                 │            └──────┬──────┘
                 │                   │ fetch/cache
                 │                   │
 ┌───────────────┼───────────────────┼──────────────────────────┐
 │  PROJECT      │                   │                          │
 │               │                   │                          │
 │  ┌────────────┴──────┐            │                          │
 │  │ git repo          │            │                          │
 │  │ (the one source)  │            │                          │
 │  └─────────┬─────────┘            │                          │
 │            │                      │                          │
 │            │   ┌──────────────────┘                          │
 │            │   │                                             │
 │            ▼   ▼                                             │
 │  ┌──────────────────────────────────────────────────────┐   │
 │  │  LAYER 0: events.db (autobiography — IRREPLACEABLE)  │   │
 │  │                                                      │   │
 │  │  ops: measure.*, scry.*, forge.*                     │   │
 │  │  epistemic: belief.created/contested/evolved/retired │   │
 │  │  lifecycle: spec.*, session.*, decision.*, discovery.*│   │
 │  └───────────────────────┬──────────────────────────────┘   │
 │                          │                                   │
 │         ┌────────────────┼────────────────────┐              │
 │         │                │                    │              │
 │         ▼                ▼                    ▼              │
 │  ┌──────────────┐ ┌──────────────┐    ┌──────────────────┐  │
 │  │ LAYER 1:     │ │ LAYER 2:     │    │ LAYER 3:         │  │
 │  │ patina.db    │ │ embeddings/  │    │ BELIEFS          │  │
 │  │ (structured) │ │ (semantic)   │    │ (proto + named)  │  │
 │  │              │ │              │    │                  │  │
 │  │ tables, FTS5 │ │ vectors,     │    │ grounds into 0-2 │  │
 │  │ code, git,   │ │ similarity,  │    │ all layers feed  │  │
 │  │ sessions     │ │ proximity    │◄──►│ UP into beliefs  │  │
 │  └──────┬───────┘ └──────────────┘    └────────┬─────────┘  │
 │         │                                      │             │
 │         └──────────────┬───────────────────────┘             │
 │                        │                                     │
 │           ┌────────────┴────────────┐                        │
 │           ▼                         ▼                        │
 │  ┌──────────────────┐      ┌──────────────────┐             │
 │  │   measure        │      │   → mother       │             │
 │  │   reads all 4    │      │   (Layer 3 sync) │             │
 │  │   layers         │      │                  │             │
 │  │   "is it         │      │   federation,    │             │
 │  │    working?"     │      │   discovery,     │             │
 │  └──────────────────┘      │   cross-project  │             │
 │                            └──────────────────┘             │
 └──────────────────────────────────────────────────────────────┘

 ┌──────────────────────────────────────────────────────────────┐
 │                    FEEDBACK LOOP                              │
 │                                                               │
 │  Layer 3 discovers pattern ──→ emits discovery event          │
 │  Discovery event lands in  ──→ Layer 0 (events.db)            │
 │  LLM reads events          ──→ proposes new belief            │
 │  New belief                ──→ grounds into all layers        │
 │                                                               │
 │  The stack is circular, not just bottom-up                    │
 └──────────────────────────────────────────────────────────────┘
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

**Event capture (7 gaps):**

| Missing Emission | Impact |
|-----------------|--------|
| scrape git | No `measure.capture` — can't trend scrape performance |
| scrape layer | No `measure.capture` — layer scrape runs untracked |
| scrape beliefs | No `measure.capture` — belief scrape runs untracked |
| scrape forge | No `measure.capture` — forge scrape runs untracked |
| context command | No event at all — context queries leave no trace |
| assay command | No event at all — structural queries leave no trace |
| scry without session | `scry.query` requires session_id — drops events outside sessions |

Note: session lifecycle events (session.started, session.ended) already exist
in `src/commands/session/internal.rs`. Original gap list overcounted.

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
