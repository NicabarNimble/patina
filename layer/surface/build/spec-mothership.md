# Spec: Mothership Architecture

**Status:** Design
**Created:** 2026-01-01
**Purpose:** Define Mother as the nervous system connecting all project islands

---

## Vision

Mother is not a database. Mother is a **federation layer** with a **knowledge graph** that connects all your projects.

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
| `git.commit` | Temporal index | — |
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

  registry.yaml           # Catalog of all known projects

  personas/default/       # User persona
    events/               # Persona events
    persona.db            # Materialized persona
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

## Phased Implementation

Only proceed to Phase N+1 when Phase N proves value.

### Phase 0: Persona Surfaces

**Problem:** PersonaOracle works but drowns in RRF fusion.

**Solution:** Display separately. Persona is context, not competition.

**Tasks:**
- [ ] Add `[PERSONA]` section to scry output
- [ ] MCP: include persona with `source: "persona"` tag
- [ ] Verify: `patina scry "error handling"` shows belief

**Exit:** Persona surfaces in scry results.

### Phase 1: Federated Query

**Goal:** Local miss → Mother routes → cross-project results.

**Tasks:**
- [ ] Mother registry knows all projects
- [ ] Query routing based on registry
- [ ] Results tagged with provenance

**Exit:** Query in Project X returns relevant results from Project Y.

### Phase 2: Knowledge Graph

**Goal:** Graph captures relationships.

**Tasks:**
- [ ] Graph schema (projects, patterns, domains, relationships)
- [ ] Graph populated from events
- [ ] Graph traversal in query routing

**Exit:** "Project X uses dojo" influences query routing.

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

## Open Questions

1. **Graph DB choice** - SQLite with JSON? Dedicated graph DB?
2. **Pattern extraction** - LLM-based? Rule-based? Hybrid?
3. **Distillation frequency** - Real-time? Batch? On session end?
4. **Cross-project privacy** - All projects visible to Mother? Opt-in?
