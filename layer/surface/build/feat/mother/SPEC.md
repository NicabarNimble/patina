---
type: feat
id: mother
status: in_progress
created: 2026-01-01
updated: 2026-02-02
related:
  - layer/surface/build/feat/v1-release/SPEC.md
  - layer/surface/build/feat/ref-repo-semantic/SPEC.md
---

# feat: Mother Architecture

> Define Mother as the nervous system connecting all project islands

## Spec Hierarchy

```
spec-mother.md (this file)
│
├── Phase 0-0.5: Foundation ✅ COMPLETE
│   Git narrative, measurement, intent detection, persona surfaces
│   Details archived in git (see Completed Phases below)
│
├── Phase 1: Delivery + Federation ← NEXT (v0.11.0)
│   Beliefs as search channel, two-step retrieval, context briefing
│   See: layer/surface/build/feat/mother-delivery/SPEC.md
│
├── Child Specs (complete, tagged):
│   ├── spec/mothership-graph - Graph routing (~1000 lines)
│   ├── spec/ref-repo-semantic - 13/13 repos indexed (now 19)
│   └── spec/vocabulary-gap - LLM query expansion (COMPLETE)
│
├── Child Specs (active):
│   └── mother-delivery - Delivery layer + federation (v0.11.0)
│
└── Deferred:
    ├── G3: Auto-detection of edges
    └── Phase 3: Session → Mother extraction
```

**Previous blocker (resolved):** Vocabulary gap solved via `expanded_terms` MCP parameter. See `spec/vocabulary-gap` tag.

---

## Vision

Mother is not a database. Mother is a **federation layer** with a **knowledge graph** that connects all your projects.

> "README is marketing, git is truth." — Commit history tells the real story of a project.

```
┌─────────────────────────────────────────────────────────────┐
│                    MOTHER (Central)                         │
│                                                             │
│  Graph: relationships (projects, patterns, domains)         │
│  Semantic: distilled knowledge (beliefs, pattern summaries) │
│  Registry: catalog of all projects                          │
│                                                             │
│  Materialized from events - rebuildable                     │
└─────────────────────────────────────────────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         ▼                    ▼                    ▼
    [patina/]           [cairo-game/]         [dojo/]
    patina.db           patina.db             patina.db
    (full content)      (full content)        (reference)
```

---

## Core Principles

### 1. Mother = Graph + Distilled Semantic

**Graph contains relationships:**
- `project USES project` (cairo-game USES dojo)
- `project HAS pattern` (patina HAS Result<T,E>)
- `pattern SIMILAR pattern` (patina-errors ≈ rust-lib-errors)
- `user BELIEVES belief` (prefer explicit error handling)

**Semantic contains distilled knowledge:**
- Persona beliefs (embedded)
- Pattern summaries (embedded)
- Project summaries (embedded)
- Domain concepts (embedded)

**NOT content duplication** - just the shape of knowledge.

### 2. Two-Tier Semantic

| Layer | Contains | Size |
|-------|----------|------|
| **Mother** | Beliefs, pattern summaries, project summaries | 1000s of entries |
| **Local** | Full sessions, code, all content | 10000s+ per project |

Mother semantic answers: "What's relevant to this query?"
Local semantic answers: "What exactly did I say/do about this?"

### 3. Events Are Source of Truth

```
EVENTS (immutable, git-tracked)
        │
        ├──► Local DB (materialize full content)
        │
        └──► Mother (materialize distilled + graph)
```

Both are materialized views. Both rebuildable from events.

### 4. Federation Not Duplication

Mother doesn't store content. Mother knows WHERE content lives.

Query flow:
1. Local project search
2. Mother semantic (distilled) - find relevant patterns/beliefs
3. Graph traversal - find related projects
4. Route to relevant local DBs
5. Combine results with provenance tags

---

## Query Flow Example

```
Query in cairo-game: "How should I handle errors?"
              │
              ▼
┌─────────────────────────────────────────┐
│ cairo-game local: "some patterns"       │
└─────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│ MOTHER Semantic (distilled)             │
│                                         │
│ Matches:                                │
│   • Belief: "explicit error handling"   │
│   • Pattern: "Result<T,E>"              │
└─────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│ Graph Traversal                         │
│                                         │
│ "Result<T,E>" → patina, rust-lib        │
│ cairo-game USES dojo                    │
└─────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│ Route to Local DBs                      │
│                                         │
│ patina/patina.db → full semantic search │
│ dojo/patina.db → reference examples     │
└─────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│ Federated Results                       │
│                                         │
│ [BELIEF] Prefer explicit error handling │
│ [PATINA] src/error.rs uses thiserror    │
│ [DOJO] Reference: felt252 error codes   │
└─────────────────────────────────────────┘
```

---

## Event Architecture

Events flow up to Mother:

```
Project X session ends
        │
        ▼
Events Generated:
  • session.end (full content)
  • pattern.detected (if found)
  • belief.captured (if explicit)
  • relationship.uses (if dependency)
        │
        ├──► Local DB: session.end → full embedding
        │
        └──► Mother:
               pattern.detected → graph + semantic
               belief.captured → belief semantic
               relationship.uses → graph edge
```

### Event Types by Destination

| Event Type | Local DB | Mother |
|------------|----------|--------|
| `session.content` | Full embedding | Summary only |
| `code.function` | Full embedding | Pattern extraction |
| `git.commit` | Temporal index + FTS5 | — |
| `moment.detected` | Moments table | Moment semantic |
| `belief.captured` | — | Belief semantic |
| `pattern.detected` | — | Pattern semantic + graph |
| `project.uses` | — | Graph edge |

---

## Mother Storage

```
~/.patina/
  events/                 # Aggregated cross-project events

  mother/
    graph.db              # Relationships (SQLite or graph DB)

    semantic/
      beliefs.usearch     # Embedded persona beliefs
      patterns.usearch    # Embedded pattern summaries
      projects.usearch    # Embedded project summaries
      domains.usearch     # Embedded domain concepts
      moments.usearch     # Embedded temporal moments (git narrative)

  registry.yaml           # Catalog of all known projects

  personas/default/       # User persona
    events/               # Persona events
    persona.db            # Materialized persona
```

---

## Temporal Narrative: Moments

**Moments** are significant points in a project's git history. They tell the story of how a project evolved.

### Moment Types

| Type | Detection | Example |
|------|-----------|---------|
| `genesis` | First commit | "dojo init" |
| `big_bang` | >50 files changed | "feat: namespaces (#2148)" - 265 files |
| `breaking` | Message contains "breaking", "BREAKING" | "Felt type migration + breaking" |
| `migration` | Message contains "migrate", "migration" | "Migrate to Cairo 2.1.0" |
| `rewrite` | Message contains "rewrite", "refactor" | "ERC1155/ERC721 rewrite" |
| `release` | Tags or "v1.0", "v2.0" in message | "dojo 1.0.0-rc.0" |

### Moment Detection Query

```sql
WITH file_counts AS (
    SELECT sha, COUNT(*) as files FROM commit_files GROUP BY sha
),
moments AS (
    SELECT c.sha, c.message, c.timestamp, fc.files,
        CASE
            WHEN c.timestamp = (SELECT MIN(timestamp) FROM commits) THEN 'genesis'
            WHEN fc.files > 100 THEN 'big_bang'
            WHEN fc.files > 50 THEN 'major'
            WHEN c.message LIKE '%breaking%' THEN 'breaking'
            WHEN c.message LIKE '%rewrite%' THEN 'rewrite'
            WHEN c.message LIKE '%migrate%' THEN 'migration'
            ELSE NULL
        END as moment_type
    FROM commits c LEFT JOIN file_counts fc ON c.sha = fc.sha
)
SELECT * FROM moments WHERE moment_type IS NOT NULL;
```

### Moments in Pipeline

Moments are **temporal signals** computed by `assay derive`:

```
scrape (git)     → commits table (raw facts)
                        │
                        ▼
assay derive     → moments table (signals)
                        │
                        ▼
repo update      → mother/semantic/moments.usearch (cross-project)
```

### Querying Temporal Narrative

```bash
# Local: when did this project make breaking changes?
patina scry "breaking changes" --moments

# Cross-project: how do other projects handle Cairo migrations?
patina scry "cairo migration" --all --moments
```

---

## What Mother Learns

Mother accumulates knowledge in two ways:

### 1. Explicit (User declares)

```bash
patina persona note "I prefer Result<T,E> over panics"
```

→ Event: `belief.captured`
→ Mother: adds to belief semantic

### 2. Implicit (Mother observes)

Session extraction detects patterns:
- "Used Result<T,E> in 5 projects" → pattern.detected
- "Project X imports dojo" → relationship.uses

→ Events flow to Mother
→ Graph + semantic updated

**Reference:** `architecture-persona-belief.md` for full extraction/refinement vision.

---

## Completed Phases (0 - 0.5)

All foundation phases complete. Details preserved in git history.

| Phase | Summary | Key Deliverables |
|-------|---------|------------------|
| **0: Git Narrative** | Commit messages searchable via FTS5 | commits_fts, moments table, LexicalOracle |
| **0.25: Measurement** | Temporal queryset, baseline MRR 0.133 | eval/temporal-queryset.json, bench infrastructure |
| **0.25b: Intent-Aware** | Auto-detect query intent, weighted RRF | QueryIntent, IntentWeights, detect_intent() |
| **0.25c: Commit Measurement** | Deterministic ground truth from git | bench generate --from-commits, commit→file expansion |
| **0.5: Persona Surfaces** | [PERSONA] tags in CLI/MCP output | Fixed PersonaOracle table bug (2026-01-08) |

**Key metrics:**
- Code retrieval MRR: 0.624 (baseline)
- Temporal MRR: 0.100 (target: 0.4) ← vocabulary gap addressed via `expanded_terms`
- Ref repo semantic: 13/13 indexed

**Previous blocker (resolved):** Vocabulary gap addressed via `expanded_terms` MCP parameter (spec/vocabulary-gap).

---

## Phased Implementation

### Phase 1: Delivery + Federation (v0.11.0)

**Goal:** Get knowledge to the LLM at the right moment, across projects.

**Redefined:** Original Phase 1 focused on federated query routing. A/B eval (session [[20260202-151214]]) revealed the real gap is **delivery** — beliefs exist but don't reach the LLM during task work (delta -0.05). Federation routing already works (Phase 2). Ref repo research ([[openclaw/openclaw]], [[steveyegge/gastown]]) informed the delivery design.

**Full spec:** `layer/surface/build/feat/mother-delivery/SPEC.md`

**Summary:**
- [ ] D1: Beliefs as default search channel (BeliefOracle in every query)
- [ ] D2: Context as dynamic briefing (beliefs + recall directive)
- [ ] D3: Two-step retrieval (snippets → detail on demand)
- [ ] D4: Routing simplified to graph-only
- [ ] D5: Mother naming cleanup (mothership → mother)
- [ ] Cross-project belief search via graph routing
- [ ] Task-oriented A/B eval re-run: target delta ≥ 0.0

**Exit:** Task-oriented queries benefit from beliefs. Results tagged with provenance. Token-efficient retrieval.

### Phase 2: Knowledge Graph ✅ COMPLETE

**Goal:** Graph captures relationships.

**Implementation:** See archived git tag `spec/mothership-graph`

**Delivered:**
- [x] Graph schema (nodes, edges, edge_usage in graph.db)
- [x] Graph populated from registry + manual edges
- [x] Graph traversal in query routing (`--routing graph`)
- [x] Weight learning from usage (`patina mother learn`)

**Result:** 100% repo recall vs 0% dumb routing. ~1000 lines implementation.

**Exit:** ✅ "Project X uses dojo" influences query routing.

### Phase 3: Extraction Loop

**Goal:** Sessions automatically feed Mother.

**Tasks:**
- [ ] Session end → extract patterns
- [ ] Pattern events flow to Mother
- [ ] Mother semantic updated

**Exit:** New session pattern appears in Mother without manual capture.

---

## Related Documents

- [architecture-persona-belief.md](../architecture-persona-belief.md) - Full extraction/refinement vision
- [concept-rag-network.md](../concept-rag-network.md) - Projects as RAG nodes
- [spec-three-layers.md](./spec-three-layers.md) - mother/patina/awaken separation
- [spec-observability.md](./spec-observability.md) - Logging infrastructure

---

## Insight: Relationship-Weighted Context (Future Phase)

*Captured 2026-01-02 during Phase 0.25 design session*

**The Core Realization:**

Not all project-to-project connections are equal. The relationship type determines what moments matter:

```
patina → dojo (ref repo)
├── Relationship: "test subject" / "example corpus"
├── Query pattern: "how does dojo structure X" (learning)
└── Relevant moments: structural changes (more test surface)

cairo-game → dojo (framework)
├── Relationship: "dependency" / "foundation"
├── Query pattern: "why did dojo change X" (survival)
└── Relevant moments: breaking changes, migrations (critical)
```

**Sessions Reveal Relationship Type:**

The queries in sessions teach us *how* a project relates to another:
- "how does X work" → reference relationship (learning)
- "why did X change" → dependency relationship (survival)
- "test retrieval on X" → test_subject relationship (tooling)

**Context Grows Outward:**

```
        Session Queries (anchor)
               │
               ▼
        Project Knowledge
               │
               ▼
    Relationship-Weighted Edges
               │
               ▼
        Cross-Project Context
```

The measurement starts from sessions, radiates outward through learned relationships.

**The Patina Value Proposition:**

Why Patina exists (vs. LLM just crawling code):

| Dimension | Raw Code Crawling | Patina Layer |
|-----------|-------------------|--------------|
| **Speed** | Re-crawl each query | Pre-indexed, relationship-weighted |
| **Cost** | Full embedding per query | Incremental, cached |
| **Accuracy** | Text matching | Learned relevance from sessions |
| **Context** | File-level | **Interconnected** across projects |

The interconnectedness is the key differentiator. Without Patina:
- Each project is an island
- LLM must rediscover patterns each time
- No relationship context

With Patina:
- Projects form a graph weighted by usage
- Sessions teach what matters for each relationship
- Context accumulates and transfers

**Future Schema (Phase 2+):**

```sql
CREATE TABLE project_relationships (
    from_project TEXT,
    to_project TEXT,
    relationship_type TEXT,     -- learned: 'framework', 'reference', 'test_subject'
    coupling_strength REAL,     -- learned from query frequency
    moment_weights JSON,        -- {"breaking": 0.9, "rewrite": 0.3}
    learned_from TEXT,          -- 'session_queries', 'import_graph'
    PRIMARY KEY (from_project, to_project)
);
```

**Anti-Pattern:** Don't predefine relationship types. Let session queries reveal them.

---

## Open Questions

1. **Graph DB choice** - SQLite with JSON? Dedicated graph DB? → **Answered: SQLite** (see git tag: spec/mothership-graph)
2. **Cross-project measurement** - How to prove graph helps? → **Answered: Phase G0** (measure dumb routing first)
3. **Relationship types** - Which types matter? → **Answered: Data-driven** (let error analysis reveal)
4. **Pattern extraction** - LLM-based? Rule-based? Hybrid?
5. **Distillation frequency** - Real-time? Batch? On session end?
6. **Cross-project privacy** - All projects visible to Mother? Opt-in?

---

## Related Specs

**Child Specs (implementations of this vision):**
- spec/mothership-graph (archived git tag) - **✅ COMPLETE** Phase 2 implementation
- [spec-ref-repo-semantic.md](./spec-ref-repo-semantic.md) - **CURRENT** Content layer for ref repos

**Architecture:**
- [spec-three-layers.md](./spec-three-layers.md) - Mother layer architecture
- [spec-pipeline.md](./spec-pipeline.md) - Data pipeline (scrape→oxidize/assay→scry)
- [concept-rag-network.md](../concept-rag-network.md) - Projects as RAG nodes vision

**How the specs connect:**
```
User Query → Graph Routing (mother-graph) → Semantic Search (ref-repo-semantic)
             "route to dojo"                    "find relevant code in dojo"
```
