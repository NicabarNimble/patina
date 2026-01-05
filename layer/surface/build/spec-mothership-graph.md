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

From RAG best practices (video):
- Node size = document importance
- Edge color = relationship type (Citation, Topic Overlap, Temporal)
- Query traversal follows edges, not just vector similarity

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

| Type | Meaning | Example | Query Pattern |
|------|---------|---------|---------------|
| `USES` | Project depends on/uses reference | patina USES dojo | "how does dojo do X" from patina |
| `LEARNS_FROM` | Project learns patterns from reference | patina LEARNS_FROM SDL | "what can I learn from SDL about Y" |
| `TESTS_WITH` | Project uses reference as test subject | patina TESTS_WITH dojo | "test this on dojo" |
| `SIBLING` | Projects share domains | cairo-game SIBLING starknet-foundry | "how did sibling solve Z" |
| `DOMAIN` | Node belongs to domain | dojo DOMAIN cairo | Domain-based routing |

**Direction matters:**
- `patina USES dojo` means queries FROM patina can route TO dojo
- Reverse: `dojo` doesn't automatically route to `patina`

---

## Phase G1: Graph Foundation

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

- [ ] `~/.patina/mother/graph.db` created on first use
- [ ] `patina mother graph` shows nodes from registry
- [ ] `patina mother link` creates edges
- [ ] `patina mother sync` rebuilds from registry

---

## Phase G2: Smart Routing

**Goal**: Use graph to route queries intelligently.

### Tasks

| Task | Effort | Deliverable |
|------|--------|-------------|
| Integrate graph into routing.rs | ~80 lines | Query graph before federation |
| Domain-based routing | ~50 lines | Use domain edges for initial filter |
| Relationship-based weights | ~40 lines | Weight results by edge strength |
| Cross-project measurement | ~100 lines | Queryset + benchmark |

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
   ├─4. Execute federated search on [patina, dojo]
   │
   └─5. Weight results by relationship
        → dojo results get 1.5x boost (USES relationship)
```

### Measurement

New queryset format for cross-project:

```json
{
  "queries": [
    {
      "query": "how does dojo handle world storage",
      "source_project": "patina",
      "expected_sources": ["dojo"],
      "expected_docs": ["crates/dojo-world/src/storage.rs"],
      "relationship": "TESTS_WITH"
    }
  ]
}
```

Metrics:
- **Source Recall@K**: Did we route to the right repos?
- **Doc Recall@K**: Did we find the right files?
- **Routing Precision**: How many irrelevant repos did we search?

### Exit Criteria

- [ ] Queries use graph for routing decisions
- [ ] Cross-project queryset with 10+ queries
- [ ] Source Recall@10 measured
- [ ] Baseline established for smart vs dumb routing

---

## Phase G3: Graph Intelligence (Future)

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

| File | Action | Purpose |
|------|--------|---------|
| `src/mother/mod.rs` | Create | Mother module entry |
| `src/mother/graph.rs` | Create | Graph implementation |
| `src/commands/mother/mod.rs` | Create | CLI commands |
| `src/commands/scry/internal/routing.rs` | Modify | Use graph for routing |
| `eval/cross-project-queryset.json` | Create | Measurement |

---

## References

- [spec-mothership.md](./spec-mothership.md) - Parent spec, phases 0-3
- [spec-three-layers.md](./spec-three-layers.md) - Mother layer definition
- [concept-rag-network.md](../concept-rag-network.md) - Projects as RAG nodes
- Video insight: Graph RAG preserves relationships, enables traversal
