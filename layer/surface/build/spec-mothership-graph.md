# Spec: Mothership Graph

**Status**: Active
**Created**: 2026-01-05
**Purpose**: Enable cross-project awareness through explicit relationship graph

---

## The Problem

Patina has federation but no intelligence:

```
TODAY:
  Query → enumerate ALL 13 repos → merge results → return

NEEDED:
  Query → graph lookup → "this relates to dojo, SDL" → route smartly → merge with provenance
```

The video insight: **Graph RAG preserves entity relationships. Traditional RAG is isolated chunks.**

We have isolated chunks. We need the graph.

---

## Design Principles

From layer/core:

| Principle | Application |
|-----------|-------------|
| **unix-philosophy** | Graph is one tool: store edges, traverse edges |
| **dependable-rust** | Small interface: `add_edge()`, `get_related()`, `traverse()` |
| **local-first** | SQLite graph.db, no external dependencies |
| **git as memory** | Edges can be rebuilt from project configs |
| **measure-first** | Baseline before building (Andrew Ng) |

From RAG best practices (video):
- Node size = document importance
- Edge color = relationship type (Citation, Topic Overlap, Temporal)
- Query traversal follows edges, not just vector similarity

**Andrew Ng Principle:** "Don't build infrastructure before proving the problem exists with data."

---

## Graph Schema

```sql
-- ~/.patina/mother/graph.db

-- Nodes: Projects and reference repos
CREATE TABLE nodes (
    id TEXT PRIMARY KEY,           -- 'patina', 'dojo', 'SDL'
    node_type TEXT NOT NULL,       -- 'project' | 'reference'
    path TEXT NOT NULL,            -- Absolute path to repo
    domains TEXT,                  -- JSON array: ["rust", "cairo"]
    summary TEXT,                  -- One-line description
    last_indexed TEXT,             -- Timestamp
    importance REAL DEFAULT 1.0    -- Node weight (from usage)
);

-- Edges: Relationships between nodes
CREATE TABLE edges (
    id INTEGER PRIMARY KEY,
    from_node TEXT NOT NULL,       -- Source node id
    to_node TEXT NOT NULL,         -- Target node id
    edge_type TEXT NOT NULL,       -- Relationship type (see below)
    weight REAL DEFAULT 1.0,       -- Edge strength
    created TEXT NOT NULL,         -- When relationship was added
    evidence TEXT,                 -- Why this relationship exists
    FOREIGN KEY (from_node) REFERENCES nodes(id),
    FOREIGN KEY (to_node) REFERENCES nodes(id),
    UNIQUE(from_node, to_node, edge_type)
);

-- Index for traversal queries
CREATE INDEX idx_edges_from ON edges(from_node);
CREATE INDEX idx_edges_to ON edges(to_node);
CREATE INDEX idx_edges_type ON edges(edge_type);
```

---

## Relationship Types

**Note**: These are hypothesized types. Phase G0 error analysis will reveal what relationships actually matter for routing. Don't over-engineer until data confirms.

### Hypothesized Types (to be validated)

| Type | Meaning | Example | Query Pattern |
|------|---------|---------|---------------|
| `USES` | Project depends on/uses reference | patina USES dojo | "how does dojo do X" from patina |
| `LEARNS_FROM` | Project learns patterns from reference | patina LEARNS_FROM SDL | "what can I learn from SDL about Y" |
| `TESTS_WITH` | Project uses reference as test subject | patina TESTS_WITH dojo | "test this on dojo" |
| `SIBLING` | Projects share domains | cairo-game SIBLING starknet-foundry | "how did sibling solve Z" |
| `DOMAIN` | Node belongs to domain | dojo DOMAIN cairo | Domain-based routing |

### Data-Driven Discovery

**Don't invent relationship types. Let the data reveal them.**

Sources for discovering real relationships:
1. **Session mentions**: "looked at dojo", "borrowed from SDL" → actual usage
2. **Query logs**: Which repos get queried together? → implicit relationship
3. **Import graph**: Which projects import which? → code-level dependency
4. **Error analysis**: Which wrong-routing cases would be fixed by which relationship type?

**Start simple**: G1 may only need `DOMAIN` (from registry.yaml) and `TESTS_WITH` (from bench commands). Add types only when error analysis proves they're needed.

**Direction matters:**
- `patina USES dojo` means queries FROM patina can route TO dojo
- Reverse: `dojo` doesn't automatically route to `patina`

---

## Phase G0: Cross-Project Measurement (Ng-Style)

**Goal**: Prove the problem exists. Measure dumb routing. Establish baseline.

**Principle**: "If you can't show me the failure cases, you don't understand the problem." — Andrew Ng

### The Question We Must Answer

> "What's the MRR of `--all-repos` on cross-project queries right now?"

If we don't know this, we can't prove graph routing helps.

### Tasks

| Task | Effort | Deliverable |
|------|--------|-------------|
| Create cross-project queryset | ~2 hours | `eval/cross-project-queryset.json` |
| Measure dumb routing baseline | ~30 min | MRR, Recall@K, Routing Waste |
| Error analysis | ~1 hour | Categorized failure modes |
| Simulate smart routing | ~30 min | Upper bound on improvement |

### Cross-Project Queryset

**Source**: Real queries that SHOULD route cross-project. Options:
1. Sessions that mention other projects ("looked at dojo for this")
2. Import graph (code that uses external repos)
3. Manual curation from actual usage

**Format**:

```json
{
  "name": "cross-project-v1",
  "source": "session-derived + manual",
  "queries": [
    {
      "id": "xp_001",
      "query": "how does dojo handle ECS components",
      "source_project": "patina",
      "expected_repos": ["dojo"],
      "expected_docs": ["crates/dojo-core/src/world.cairo"],
      "derivation": "session 20251203 mentioned dojo ECS"
    },
    {
      "id": "xp_002",
      "query": "cairo felt252 type system",
      "source_project": "patina",
      "expected_repos": ["dojo", "starknet-foundry"],
      "expected_docs": ["..."],
      "derivation": "domain overlap: cairo"
    }
  ]
}
```

**Key insight**: `expected_repos` is the ground truth for routing. This is what we're measuring.

### Metrics

| Metric | Definition | Baseline Target |
|--------|------------|-----------------|
| **Source Recall@K** | Did we search the right repos? | Measure current |
| **Doc Recall@K** | Did we find the right files? | Measure current |
| **Routing Waste** | How many irrelevant repos searched? | 13 (all) → ? |
| **MRR** | Where does first relevant result appear? | Measure current |

### Baseline Measurement

```bash
# Dumb routing (current)
patina scry "how does dojo handle ECS" --all-repos
# → Searches 13 repos
# → Returns N results
# → X% from relevant repos, Y% noise

# Simulate smart routing (manual --repo)
patina scry "how does dojo handle ECS" --repo dojo
# → Searches 1 repo
# → Compare quality
```

### Error Analysis

Categorize failures from `--all-repos`:

| Error Type | Example | Root Cause |
|------------|---------|------------|
| **Noise drowning signal** | dojo result at rank 15, SDL at rank 1 | No relevance weighting |
| **Wrong domain** | cairo query returns javascript | Domain not considered |
| **Missing repo** | Relevant repo not searched | N/A (searches all) |
| **Relevant but scattered** | Results from 8 repos when 2 are relevant | No relationship awareness |

### Simulated Smart Routing

Before building graph, simulate what perfect routing would achieve:

```bash
# For each query in cross-project-queryset:
# 1. Run with --all-repos (dumb)
# 2. Run with --repo <expected_repos> (smart simulation)
# 3. Compare MRR, Recall

# This gives us the CEILING - maximum possible improvement
```

### Baseline Results (2026-01-05)

**Dumb Routing (`--all-repos`) - 3 sample queries:**

| Query | Expected Repo | In Top 5? | Top Result (Actual) |
|-------|---------------|-----------|---------------------|
| "dojo ECS world storage" | dojo | ❌ No | LIVESTORE qr.ts |
| "vector similarity search" | USearch | ❌ No | SDL power.c |
| "opencode MCP server" | opencode | ❌ No | SDL server.py |

**Simulated Smart Routing (`--repo <expected>`):**

| Query | Expected Repo | Top Result | Relevant? |
|-------|---------------|------------|-----------|
| "dojo ECS world storage" | dojo | dojo_store::process | ✅ Yes |

**Error Pattern**: Noise drowning signal. Generic term matches (e.g., "server", "world", "storage") from irrelevant repos rank higher than specific matches from relevant repos.

**Root Cause**: No domain/relationship awareness. FTS5 `OR` query treats all repos equally.

**Gap Assessment**: **Large** - expected repos not appearing in top 5 at all. Smart routing dramatically improves relevance.

**Decision**: ✅ Proceed to G1. The gap is severe enough to justify building graph infrastructure.

### Exit Criteria

- [x] `eval/cross-project-queryset.json` with 10+ queries (12 created)
- [x] Dumb routing baseline: expected repos missing from top 5
- [x] Error analysis: "noise drowning signal" identified
- [x] Simulated smart routing: dramatically better relevance
- [x] **Decision point**: Gap is large → proceed to G1

### Why This Matters

If dumb routing MRR is 0.6 and simulated smart routing is 0.65, building graph.db is over-engineering.

If dumb routing MRR is 0.2 and simulated smart routing is 0.7, graph is clearly needed.

**The data decides.**

---

## Phase G1: Graph Foundation

**Prerequisite**: Phase G0 complete. Data shows graph routing will meaningfully improve metrics.

**Goal**: Create graph.db, populate from registry, enable manual edges.

### Tasks

| Task | Effort | Deliverable |
|------|--------|-------------|
| Create `src/mother/graph.rs` | ~150 lines | Graph struct with SQLite backend |
| Schema migration | ~30 lines | Create tables on first access |
| Populate from registry | ~50 lines | Convert registry.yaml → nodes |
| CLI: `patina mother graph` | ~100 lines | Show graph state |
| CLI: `patina mother link` | ~80 lines | Add/remove edges manually |

### Interface

```rust
// src/mother/graph.rs
pub struct MotherGraph {
    db: Connection,
}

impl MotherGraph {
    pub fn open() -> Result<Self>;

    // Node operations
    pub fn add_node(&self, id: &str, node_type: NodeType, path: &Path) -> Result<()>;
    pub fn get_node(&self, id: &str) -> Result<Option<Node>>;
    pub fn list_nodes(&self) -> Result<Vec<Node>>;

    // Edge operations
    pub fn add_edge(&self, from: &str, to: &str, edge_type: EdgeType) -> Result<()>;
    pub fn remove_edge(&self, from: &str, to: &str, edge_type: EdgeType) -> Result<()>;
    pub fn get_edges(&self, node: &str) -> Result<Vec<Edge>>;

    // Traversal
    pub fn get_related(&self, node: &str, edge_types: &[EdgeType]) -> Result<Vec<Node>>;
    pub fn traverse(&self, start: &str, depth: usize) -> Result<Vec<(Node, Vec<Edge>)>>;
}
```

### CLI Commands

```bash
# View graph
patina mother graph                    # Show all nodes and edges
patina mother graph --nodes            # List nodes only
patina mother graph --edges            # List edges only

# Add relationships
patina mother link patina USES dojo    # patina uses dojo as reference
patina mother link patina TESTS_WITH dojo --evidence "benchmark subject"

# Remove relationships
patina mother unlink patina USES dojo

# Sync from registry
patina mother sync                     # Rebuild nodes from registry.yaml
```

### Exit Criteria

**Functional (does it work):**
- [ ] `~/.patina/mother/graph.db` created on first use
- [ ] `patina mother graph` shows nodes from registry
- [ ] `patina mother link` creates edges
- [ ] `patina mother sync` rebuilds from registry

**Ng checkpoint (does it represent the problem):**
- [ ] Graph can encode the relationships identified in G0 error analysis
- [ ] Manual edges added for top 3 failure cases from G0
- [ ] Verify: `patina mother graph` shows edges that WOULD have fixed G0 failures

**Note**: G1 is infrastructure. It doesn't improve metrics yet. But it MUST be able to represent what G0 revealed as the fix.

---

## Phase G2: Smart Routing

**Prerequisite**: G1 complete. Graph populated with edges for G0 failure cases.

**Goal**: Use graph to route queries. Measure improvement against G0 baseline.

### Tasks

| Task | Effort | Deliverable |
|------|--------|-------------|
| Integrate graph into routing.rs | ~80 lines | Query graph before federation |
| Domain-based routing | ~50 lines | Use domain edges for initial filter |
| Relationship-based weights | ~40 lines | Weight results by edge strength |
| Benchmark comparison | ~50 lines | Compare to G0 baseline |

### Query Flow

```
Query: "how does dojo handle ECS components"
   │
   ├─1. Detect current project: patina
   │
   ├─2. Query graph: get_related("patina", [USES, TESTS_WITH])
   │    → Returns: [dojo, opencode, SDL]
   │
   ├─3. Filter by query relevance (domain match)
   │    → "ECS" matches dojo (cairo, rust)
   │    → Returns: [dojo]
   │
   ├─4. Execute federated search on [patina, dojo] (not all 13)
   │
   └─5. Weight results by relationship
        → Weight TBD from G0 error analysis (not arbitrary 1.5x)
```

### Measurement (Against G0 Baseline)

**Use the same queryset from G0.** No new queryset needed.

```bash
# G0 established baseline:
#   Dumb routing: MRR X, Recall@10 Y%, searched 13 repos
#   Simulated smart: MRR X', Recall@10 Y'%

# G2 measures actual improvement:
patina bench retrieval -q eval/cross-project-queryset.json --routing graph
# → Smart routing: MRR Z, Recall@10 W%, searched N repos (N < 13)
```

**Metrics (same as G0, now comparing):**

| Metric | G0 Dumb | G0 Simulated | G2 Actual | Target |
|--------|---------|--------------|-----------|--------|
| MRR | ? | ? | ? | ≥ 80% of simulated |
| Recall@10 | ? | ? | ? | ≥ 80% of simulated |
| Repos searched | 13 | manual | auto | < 5 avg |

### Exit Criteria

**Functional:**
- [ ] `--routing graph` flag added to scry
- [ ] Graph consulted before federation
- [ ] Results tagged with routing source

**Ng checkpoint (did it help):**
- [ ] MRR improved over G0 dumb baseline
- [ ] Recall@10 improved over G0 dumb baseline
- [ ] Routing Waste reduced (fewer irrelevant repos searched)
- [ ] **Gap closed**: Actual approaches simulated upper bound from G0

**Anti-pattern**: If G2 metrics don't approach G0 simulated ceiling, the graph isn't the right fix. Revisit error analysis.

---

## Phase G3: Graph Intelligence (Future)

**Prerequisite**: G2 proves graph routing helps. Metrics justify automation investment.

**Goal**: Auto-populate edges, learn from usage.

### Auto-Detection

| Signal | Edge Type | How |
|--------|-----------|-----|
| Import statements | USES | `use dojo::*` in code |
| Session mentions | LEARNS_FROM | "looked at SDL for this" |
| Benchmark runs | TESTS_WITH | `--repo dojo` in bench commands |
| Domain overlap | SIBLING | Shared tags in registry |

### Usage Learning

```sql
-- Track which edges actually help
CREATE TABLE edge_usage (
    edge_id INTEGER,
    query TEXT,
    result_rank INTEGER,
    was_useful INTEGER,        -- 1 if result was used
    timestamp TEXT,
    FOREIGN KEY (edge_id) REFERENCES edges(id)
);
```

Edge weights updated based on usage: edges that lead to useful results get higher weight.

### Exit Criteria

- [ ] Auto-detection populates edges from code analysis
- [ ] Usage tracking informs edge weights
- [ ] Graph improves over time

---

## What This Enables

With graph in place:

| Capability | Before | After |
|------------|--------|-------|
| Cross-project query | Search ALL 13 repos | Search 2-3 relevant repos |
| Relationship awareness | None | "patina uses dojo for testing" |
| Query routing | Dumb enumeration | Graph traversal |
| Result provenance | Source tags | Relationship context |
| Token efficiency | Many irrelevant results | Focused results |

---

## Files to Create/Modify

**Phase G0 (Measurement):**
| File | Action | Purpose |
|------|--------|---------|
| `eval/cross-project-queryset.json` | Create | Ground truth for routing |

**Phase G1 (Infrastructure):**
| File | Action | Purpose |
|------|--------|---------|
| `src/mother/mod.rs` | Create | Mother module entry |
| `src/mother/graph.rs` | Create | Graph implementation |
| `src/commands/mother/mod.rs` | Create | CLI commands |

**Phase G2 (Integration):**
| File | Action | Purpose |
|------|--------|---------|
| `src/commands/scry/internal/routing.rs` | Modify | Use graph for routing |
| `src/commands/bench/mod.rs` | Modify | Add `--routing` flag |

---

## References

- [spec-mothership.md](./spec-mothership.md) - Parent spec, phases 0-3
- [spec-three-layers.md](./spec-three-layers.md) - Mother layer definition
- [concept-rag-network.md](../concept-rag-network.md) - Projects as RAG nodes
- Video insight: Graph RAG preserves relationships, enables traversal
