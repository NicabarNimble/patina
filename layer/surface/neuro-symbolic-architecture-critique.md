---
id: neuro-symbolic-architecture-critique
version: 1
status: active
created_date: 2025-10-29
updated_date: 2025-10-29
oxidizer: nicabar
tags: [architecture, neuro-symbolic, ml, critique, embeddings, prolog, sqlite]
---

# Neuro-Symbolic Architecture: Critical Analysis

**Context:** Evaluation of Patina's persona belief system from an ML/Neuro-Symbolic AI expert perspective.

**Current Design:**
- SQLite for structured storage
- Scryer Prolog for logical inference
- LLM (Claude) for pattern extraction and dialogue
- Confidence scoring (0.0-1.0)
- Evidence linking (beliefs ← observations)

---

## What's Right ✅

### 1. Correct Neuro-Symbolic Split

```
LLM (Neural)     → Pattern extraction, semantic understanding, dialogue
Prolog (Symbolic) → Rule enforcement, logical inference, consistency
SQLite (Storage)  → Efficient structured queries, relations
```

**Why this works:** Each component does what it's good at. Not forcing LLM to do logic (unreliable) or Prolog to do NLP (impossible).

### 2. Confidence as Continuous Value

Using `confidence REAL` (0.0-1.0) instead of binary true/false enables:
- Bayesian-ish belief revision
- Gradual strengthening/weakening
- Uncertainty representation

**Example:**
```
Initial: 0.5 (uncertain)
+2 evidence: 0.7 (confident)
+contradiction: 0.55 (weakened)
```

### 3. Evidence Provenance

`belief_observations` table linking beliefs to supporting evidence:
```sql
belief_observations (belief_id, session_id, observation_type, observation_id, validates)
```

**Critical for:**
- Explainability ("Why do I believe X?" → "Evidence from sessions Y, Z")
- Confidence adjustment (new evidence → update confidence)
- Debugging beliefs (trace back to source)

---

## Critical Gaps ❌

### 1. **No Semantic Search / Embeddings**

**Current design:**
```sql
SELECT * FROM patterns WHERE pattern_name = 'security-review'
```

**Problem:** Keyword matching only.
- Session says "code audit" → Missed
- Session says "credential protection" → Missed
- Only exact string matches work

**What's missing:**
```python
# Generate embeddings for all observations
embedding = embed("tmpfs for secrets")  # → [0.123, 0.456, ...]

# Vector similarity search
find_similar("credential security")
# Returns: "tmpfs for secrets", "1password", "vault solutions"
```

**Why this matters for multi-domain persona:**
```
Film domain: "character-driven narratives"
Session text: "psychological depth", "complex protagonists", "internal conflict"

Without embeddings: No match (different words)
With embeddings: Semantic match (same concept)

Cross-domain: "depth" in films ≈ "depth" in code architecture
```

**Solutions:**

**Option A: sqlite-vss extension**
```sql
-- Add vector similarity search to SQLite
CREATE VIRTUAL TABLE observation_vectors USING vss0(
    observation_id INTEGER,
    embedding(768)  -- embedding dimension
);

-- Semantic search
SELECT observation_id, distance
FROM observation_vectors
WHERE vss_search(embedding, query_vector)
LIMIT 10;
```

**Option B: Separate vector DB**
```
SQLite (structured data)    Qdrant/Chroma (vectors)
     ↓                             ↓
Beliefs, evidence          ←→  Embeddings
Prolog queries                  Semantic search
```

**Option C: Skip vectors (NOT recommended)**
```sql
-- Full-text search only
CREATE VIRTUAL TABLE beliefs_fts USING fts5(statement, description);
-- Fast keyword search, but not semantic
```

**Recommendation:** Use sqlite-vss extension. Keeps everything in SQLite, adds semantic search capability.

---

### 2. **No Belief Graph / Relationships**

**Current design:**
```sql
beliefs: flat list
parent_belief_id: linear refinement only
```

**Problem:** Beliefs aren't a tree, they're a **graph**.

**Example relationships:**
```
"prefers_rust" ─────┬───→ "prefers_compiled_languages"
                    │
"values_safety" ────┴───→ "prefers_compiled_languages"

"character_driven_films" ──→ "values_depth_over_action"
                             ↓
"prefers_rust" ─────────────┘
    (Both value depth/substance over surface/performance)
```

**Cross-domain belief influence:**
- Film preferences (depth, complexity) correlate with code preferences (expressiveness, abstractions)
- Parenting beliefs (consistency, structure) correlate with architecture beliefs (patterns, discipline)
- Life philosophy (pragmatism) influences all domains

**What's missing:**
```sql
CREATE TABLE belief_relationships (
    belief_1 INTEGER NOT NULL,
    belief_2 INTEGER NOT NULL,
    relationship_type TEXT NOT NULL,  -- 'supports', 'contradicts', 'refines', 'correlates'
    strength REAL DEFAULT 0.5,        -- 0.0-1.0 relationship strength
    discovered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    evidence TEXT,                    -- Why this relationship exists
    FOREIGN KEY (belief_1) REFERENCES beliefs(id),
    FOREIGN KEY (belief_2) REFERENCES beliefs(id)
);

CREATE INDEX idx_belief_relationships ON belief_relationships(belief_1, belief_2);
```

**Prolog queries enabled by this:**
```prolog
% Transitive support
supports(B1, B3) :-
    supports(B1, B2),
    supports(B2, B3).

% Find all beliefs supporting a decision
supporting_beliefs(Decision, Supports) :-
    findall(B, supports(B, Decision), Supports).

% Detect contradictions
conflict(B1, B2) :-
    belief(B1, true, C1), C1 > 0.6,
    belief(B2, true, C2), C2 > 0.6,
    contradicts(B1, B2).

% Cross-domain patterns
cross_domain_correlation(Domain1, Domain2, SharedBelief) :-
    belief_in_domain(B1, Domain1),
    belief_in_domain(B2, Domain2),
    correlates(B1, B2),
    abstraction(B1, SharedBelief),
    abstraction(B2, SharedBelief).
```

**Why this matters:**
- Explain decisions: "You prefer Rust because: values_safety → prefers_compiled_languages → prefers_rust"
- Detect conflicts: "You value simplicity (0.9) but also completeness (0.85) - these conflict here"
- Cross-domain insights: "Your film preferences predict you'll like functional programming"

---

### 3. **No Temporal Dynamics**

**Current schema:**
```sql
confidence REAL,           -- Current confidence
last_validated TIMESTAMP   -- When last checked
```

**What's missing:**
```sql
-- Belief history (how beliefs evolve)
CREATE TABLE belief_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    belief_id INTEGER NOT NULL,
    confidence REAL NOT NULL,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    reason TEXT,  -- Why it changed
    trigger_session_id TEXT,  -- What caused the change
    FOREIGN KEY (belief_id) REFERENCES beliefs(id)
);

-- Context windows (when belief is valid)
CREATE TABLE belief_contexts (
    belief_id INTEGER NOT NULL,
    valid_from TIMESTAMP,
    valid_until TIMESTAMP,
    context_description TEXT,  -- e.g., "while using React", "during startup phase"
    FOREIGN KEY (belief_id) REFERENCES beliefs(id)
);
```

**Example evolution:**
```
2023-03: "prefers_react_for_frontends" (0.90)
         Evidence: 5 React projects

2024-06: "prefers_react_for_frontends" (0.70)
         Evidence: Tried Svelte, found simpler

2024-06: "prefers_svelte_for_frontends" (0.65)
         Evidence: 2 Svelte projects

2024-12: "prefers_react_for_frontends" (0.40)
         "prefers_svelte_for_frontends" (0.85)
         Belief shifted
```

**Why this matters:**
- Show belief evolution over time
- Understand what changed your mind
- Detect trends (moving toward/away from something)
- Context-aware beliefs ("preferred React in 2023, not now")

**Temporal decay (from original design):**
```prolog
% Beliefs not validated recently lose confidence
temporal_relevance(Belief, DecayedConfidence) :-
    belief_confidence(Belief, BaseConfidence),
    last_validated(Belief, Date),
    days_ago(Date, Days),
    Days > 180,
    DecayFactor is min(0.2, (Days - 180) / 365 * 0.1),
    DecayedConfidence is max(0.4, BaseConfidence - DecayFactor).
```

**Not implemented yet.**

---

### 4. **No Retrieval Layer Design**

**Critical question:** How will LLM actually USE the beliefs?

**Current state:** Undefined. No design for how beliefs get into LLM context.

**Three approaches:**

#### **A. Dump All Beliefs (Naive)**
```python
beliefs = db.query("SELECT * FROM beliefs WHERE active=1")
prompt = f"User beliefs:\n{beliefs}\n\nAnswer question: {question}"
```

**Problem:**
- Context limit (can't fit 1000 beliefs)
- Irrelevant beliefs add noise
- Poor signal-to-noise ratio

---

#### **B. Retrieve Relevant Beliefs (Better)**
```python
# Semantic search for relevant beliefs
relevant = semantic_search(question, top_k=10)
prompt = f"Relevant beliefs:\n{relevant}\n\nAnswer: {question}"
```

**Requires:** Embeddings + vector search

**Problem:**
- Misses transitive relationships
- No reasoning about belief interactions

---

#### **C. Hybrid Retrieval (Recommended)**
```python
def retrieve_beliefs_for_query(question, top_k=10):
    # 1. Semantic search for relevant beliefs
    candidates = semantic_search(question, top_k=20)

    # 2. Prolog reasoning to find related beliefs
    related = []
    for belief in candidates:
        supporting = prolog.query(f"supports(X, {belief.id})")
        contradicting = prolog.query(f"contradicts(X, {belief.id})")
        related.extend(supporting + contradicting)

    # 3. Deduplicate and expand
    all_beliefs = candidates + related
    all_beliefs = deduplicate(all_beliefs)

    # 4. Rank by: semantic similarity × confidence × recency
    ranked = rank_beliefs(all_beliefs,
                         similarity=embeddings,
                         confidence=db.confidence,
                         recency=db.last_validated)

    # 5. Return top-k with provenance
    return ranked[:top_k]
```

**Example query:** "Should I use ECS architecture for this game?"

**Retrieval flow:**
```
1. Semantic search (embeddings):
   → "prefers_ecs_for_games" (0.90)
   → "values_data_oriented_design" (0.85)
   → "avoids_oop_for_performance" (0.80)

2. Prolog expansion (relationships):
   supports(values_performance, prefers_ecs_for_games)
   supports(prefers_composition, prefers_ecs_for_games)
   contradicts(prefers_simple_oop_for_small_projects, prefers_ecs_for_games)

3. Ranked results:
   ┌─────────────────────────────────────────────┬────────┬─────────┐
   │ Belief                                      │ Conf.  │ Reason  │
   ├─────────────────────────────────────────────┼────────┼─────────┤
   │ prefers_ecs_for_games                       │ 0.90   │ Direct  │
   │ values_performance                          │ 0.95   │ Supports│
   │ prefers_composition_over_inheritance        │ 0.85   │ Supports│
   │ prefers_simple_oop_for_small_projects       │ 0.75   │ Conflict│
   └─────────────────────────────────────────────┴────────┴─────────┘

4. Construct LLM prompt:
   "Given these beliefs (with evidence):
    - You prefer ECS for games (0.90) - Evidence: 3 game projects
    - You value performance (0.95) - Supports ECS
    - Potential conflict: You prefer simple OOP for small projects (0.75)
      → Check: Is this a small project?

    Recommendation: ..."
```

**This requires building:**
- Embedding generation pipeline
- Vector similarity search
- Prolog query interface
- Ranking algorithm
- Prompt construction

**Not designed yet.**

---

### 5. **Domain Organization Unclear**

**You said:** "organized in domains and links to those domains"

**Current schema:**
```sql
CREATE TABLE domains (
    name TEXT PRIMARY KEY,
    description TEXT
);

CREATE TABLE belief_domains (
    belief_id INTEGER,
    domain_name TEXT,
    domain_statement TEXT  -- How this domain expresses the belief
);
```

**Unanswered questions:**

**Q1: Are domains tags or ontology?**

**Option A: Flat tags**
```sql
-- Simple many-to-many
belief_domains(belief_id, domain_name)

-- Belief in multiple domains
"values_depth_over_surface" → [films, software, parenting]
```

**Pros:** Simple, flexible
**Cons:** No hierarchy, no inheritance

**Option B: Hierarchical ontology**
```prolog
% Domain hierarchy
domain(software_development).
domain(rust_programming, parent: software_development).
domain(cli_tools, parent: software_development).
domain(web_development, parent: software_development).

% Belief inherits to parent domains
belief_in_domain(prefers_rust, rust_programming).

in_domain(Belief, ParentDomain) :-
    belief_in_domain(Belief, ChildDomain),
    domain(ChildDomain, parent: ParentDomain).

% Query: all software_development beliefs includes rust_programming beliefs
```

**Pros:** Inheritance, hierarchical queries
**Cons:** More complex, need ontology design

**Q2: How do cross-domain beliefs work?**

```
Belief: "values_depth_over_surface"
Domains: [films, software_architecture, parenting]

How is this represented?
- One belief, multiple domain tags?
- Multiple domain-specific beliefs that correlate?
- Universal belief that manifests per-domain?
```

**Q3: Domain-specific vs Universal beliefs?**

```
Project-specific: "uses_sqlite_for_patina" (only in Patina project)
Domain-specific: "prefers_rust_for_cli" (software domain)
Universal: "values_simplicity" (all domains)

How are these distinguished?
```

**Not defined yet.**

---

## Architectural Decisions Needed

### **Decision 1: Is Prolog Actually Doing Work?**

**Current Prolog usage:**
- ✅ Confidence rules (working, tested)
- ❓ Pattern inference (defined but not used)
- ❌ Cross-domain reasoning (not designed)
- ❌ Belief relationships (not implemented)
- ❌ Conflict detection (schema exists, no rules)

**Question:** Is Prolog earning its keep?

**Alternative:** Confidence rules in Python:
```python
def initial_confidence(evidence_count):
    if evidence_count == 0: return 0.5
    if evidence_count <= 2: return 0.5 + (evidence_count * 0.15)
    return min(0.85, 0.5 + (evidence_count * 0.1))
```

**When Prolog shines:**
- Transitive relationships: `ancestor(X, Z) :- parent(X, Y), ancestor(Y, Z)`
- Constraint satisfaction: "Find beliefs consistent with X but not contradicting Y"
- Complex queries: "Show architecture beliefs that influenced this decision"

**If you're not doing those queries, Prolog is ceremony.**

**Recommendation:** Keep Prolog IF you implement:
1. Belief relationship graph (supports/contradicts)
2. Cross-domain reasoning
3. Transitive queries

Otherwise, Python functions suffice.

---

### **Decision 2: Global Persona vs Project Beliefs**

**You said:** "main patina will hold everything... projects have their own db"

**Two architectures:**

#### **Architecture A: Global Persona + Project Views**

```
~/.patina/persona.db (EVERYTHING - single source of truth)
  ├─ Universal beliefs
  ├─ Domain-specific beliefs
  └─ Project-specific beliefs (tagged with project_id)

~/projects/patina/.patina/beliefs.db (materialized view)
  ├─ Cached copy of relevant beliefs
  ├─ Plus project-local beliefs (not in global yet)
  └─ Links back to global persona.db
```

**Data flow:**
```
1. Session ends in project
2. Extract observations → project beliefs.db
3. Persona session in project → create/update beliefs
4. Beliefs sync to global persona.db
5. Other projects pull relevant beliefs from global
```

**Pros:**
- Single source of truth
- Beliefs available across projects
- Lessons learned transfer

**Cons:**
- Need sync mechanism
- Cache invalidation complexity
- Global persona grows unbounded

---

#### **Architecture B: Federated Beliefs**

```
~/.patina/persona.db (universal beliefs only)
  └─ "values_simplicity", "prefers_depth", etc.

~/projects/patina/.patina/beliefs.db
  ├─ Software development beliefs
  ├─ Patina-specific beliefs
  └─ References to global beliefs (foreign key? URI?)

~/projects/film-reviews/.patina/beliefs.db
  └─ Film domain beliefs
```

**Data flow:**
```
1. Each project maintains own beliefs
2. Universal beliefs extracted to global
3. Queries join global + project beliefs
```

**Pros:**
- Clear separation
- Projects independent
- No sync needed

**Cons:**
- Query complexity (join across DBs)
- Belief relationships span databases
- How does global learn from project?

---

**Recommendation:** Architecture A (Global + Views)

**Why:**
- Persona should learn from ALL experiences
- Beliefs transfer across projects
- Simpler mental model (one belief system)

**Implementation:**
```sql
-- In global persona.db
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT,
    path TEXT
);

ALTER TABLE beliefs ADD COLUMN project_id TEXT;
ALTER TABLE beliefs ADD COLUMN scope TEXT;  -- 'universal', 'domain', 'project'

-- Project-specific view
CREATE VIEW project_beliefs AS
SELECT * FROM beliefs
WHERE scope = 'universal'
   OR (scope = 'domain' AND domain IN (SELECT domain FROM project_domains))
   OR (scope = 'project' AND project_id = :project_id);
```

---

### **Decision 3: Embedding Strategy**

**Required for:** Semantic search, cross-domain concept matching, retrieval layer

**Options:**

**A. sqlite-vss extension**
```bash
pip install sqlite-vss
```

**Pros:**
- Everything in SQLite
- Simple architecture
- Fast vector search

**Cons:**
- Extension dependency
- Limited to SQLite features

**B. Separate vector DB (Qdrant/Chroma)**
```
SQLite ←→ Qdrant
   ↓          ↓
Beliefs   Embeddings
```

**Pros:**
- Specialized vector search
- Better scaling
- More features (filtering, hybrid search)

**Cons:**
- Two databases to manage
- Sync complexity
- Additional dependency

**C. pgvector (PostgreSQL extension)**

Switch from SQLite to PostgreSQL + pgvector.

**Pros:**
- Production-grade
- Native vector support
- Better concurrency

**Cons:**
- Lose SQLite simplicity
- PostgreSQL dependency
- Overkill for single-user

---

**Recommendation:** sqlite-vss extension

**Why:**
- Maintains single-database simplicity
- Good enough for single-user persona
- Minimal dependencies
- Easy to swap later if needed

**Implementation:**
```python
import sqlite_vss

# Initialize
conn = sqlite3.connect('persona.db')
conn.enable_load_extension(True)
sqlite_vss.load(conn)

# Create vector table
conn.execute("""
    CREATE VIRTUAL TABLE belief_vectors USING vss0(
        belief_id INTEGER PRIMARY KEY,
        embedding(768)  -- BERT/sentence-transformers dimension
    )
""")

# Embed beliefs
from sentence_transformers import SentenceTransformer
model = SentenceTransformer('all-MiniLM-L6-v2')

for belief in beliefs:
    embedding = model.encode(belief.statement)
    conn.execute("INSERT INTO belief_vectors VALUES (?, ?)",
                 (belief.id, embedding))

# Search
query_embedding = model.encode("prefer typed languages")
results = conn.execute("""
    SELECT belief_id, distance
    FROM belief_vectors
    WHERE vss_search(embedding, ?)
    LIMIT 10
""", (query_embedding,))
```

---

## Complete Architecture Proposal

### **Component Stack**

```
┌─────────────────────────────────────────────────────────────┐
│  LLM Layer (Claude Code / API)                               │
│  - Pattern extraction from sessions                          │
│  - Persona dialogue                                          │
│  - Belief explanation                                        │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────┴────────────────────────────────────────┐
│  Retrieval Layer (NEW)                                       │
│  - Semantic search (embeddings)                              │
│  - Prolog reasoning (relationships)                          │
│  - Ranking (similarity × confidence × recency)               │
│  - Context construction for LLM                              │
└────────────────────┬────────────────────────────────────────┘
                     │
        ┌────────────┼────────────┐
        │            │            │
┌───────▼──────┐ ┌──▼────────┐ ┌─▼──────────────┐
│ SQLite       │ │ sqlite-vss │ │ Scryer Prolog  │
│ (structured) │ │ (vectors)  │ │ (reasoning)    │
│              │ │            │ │                │
│ - Beliefs    │ │ - Belief   │ │ - Confidence   │
│ - Evidence   │ │   embeddings│ │   rules        │
│ - Sessions   │ │ - Session  │ │ - Relationship │
│ - Domains    │ │   embeddings│ │   graph        │
│              │ │            │ │ - Conflict     │
│              │ │            │ │   detection    │
└──────────────┘ └────────────┘ └────────────────┘
```

### **Data Model (Enhanced)**

```sql
-- ============================================================================
-- EXISTING (Keep)
-- ============================================================================
beliefs (id, statement, value, confidence, created_at, last_validated, ...)
belief_observations (belief_id, session_id, observation_type, observation_id, validates)
sessions (id, started_at, work_type, git_branch, ...)
patterns (id, session_id, pattern_name, category, ...)
domains (name, description)
belief_domains (belief_id, domain_name, domain_statement)

-- ============================================================================
-- NEW (Add)
-- ============================================================================

-- Belief relationships (graph)
CREATE TABLE belief_relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    belief_1 INTEGER NOT NULL,
    belief_2 INTEGER NOT NULL,
    relationship_type TEXT NOT NULL,  -- 'supports', 'contradicts', 'refines', 'correlates'
    strength REAL DEFAULT 0.5,
    discovered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    evidence TEXT,
    FOREIGN KEY (belief_1) REFERENCES beliefs(id),
    FOREIGN KEY (belief_2) REFERENCES beliefs(id),
    UNIQUE(belief_1, belief_2, relationship_type)
);

-- Belief history (temporal dynamics)
CREATE TABLE belief_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    belief_id INTEGER NOT NULL,
    confidence REAL NOT NULL,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    reason TEXT,
    trigger_session_id TEXT,
    FOREIGN KEY (belief_id) REFERENCES beliefs(id)
);

-- Vector embeddings (semantic search)
CREATE VIRTUAL TABLE belief_vectors USING vss0(
    belief_id INTEGER PRIMARY KEY,
    embedding(768)
);

CREATE VIRTUAL TABLE observation_vectors USING vss0(
    observation_id INTEGER PRIMARY KEY,
    observation_type TEXT,  -- 'pattern', 'technology', 'decision'
    embedding(768)
);

-- Projects (for global persona)
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    domains TEXT,  -- JSON array of active domains
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Add to beliefs table
ALTER TABLE beliefs ADD COLUMN project_id TEXT REFERENCES projects(id);
ALTER TABLE beliefs ADD COLUMN scope TEXT DEFAULT 'universal';  -- 'universal', 'domain', 'project'
```

### **Prolog Rules (Enhanced)**

```prolog
% ============================================================================
% EXISTING (Keep)
% ============================================================================
query_initial_confidence(EvidenceCount, Confidence).
strengthen_confidence(Current, Supporting, New).
weaken_confidence(Current, Contradicting, New).

% ============================================================================
% NEW (Add)
% ============================================================================

% Belief relationships
supports(B1, B2) :- belief_relationship(B1, B2, supports, Strength), Strength > 0.5.
contradicts(B1, B2) :- belief_relationship(B1, B2, contradicts, Strength), Strength > 0.5.
correlates(B1, B2) :- belief_relationship(B1, B2, correlates, Strength), Strength > 0.7.

% Transitive support
supports_transitively(B1, B3) :-
    supports(B1, B2),
    supports(B2, B3).

% Find all supporting beliefs
supporting_beliefs(Belief, Supports) :-
    findall(B, supports(B, Belief), Supports).

% Conflict detection
has_conflict(B1, B2) :-
    belief(B1, true, C1), C1 > 0.6,
    belief(B2, true, C2), C2 > 0.6,
    contradicts(B1, B2).

% Cross-domain patterns
cross_domain_belief(B1, B2, SharedConcept) :-
    belief_in_domain(B1, Domain1),
    belief_in_domain(B2, Domain2),
    Domain1 \= Domain2,
    correlates(B1, B2),
    shared_abstraction(B1, B2, SharedConcept).

% Belief scope queries
universal_beliefs(Beliefs) :-
    findall(B, (belief(B, _, C), C > 0.7, belief_scope(B, universal)), Beliefs).

project_beliefs(ProjectId, Beliefs) :-
    findall(B, belief_in_project(B, ProjectId), Beliefs).
```

### **Retrieval Layer API**

```python
class BeliefRetriever:
    """Hybrid retrieval: embeddings + Prolog reasoning"""

    def __init__(self, db_path, prolog_path):
        self.db = sqlite3.connect(db_path)
        self.prolog = PrologEngine(prolog_path)
        self.embedder = SentenceTransformer('all-MiniLM-L6-v2')

    def retrieve_for_query(self, query: str, top_k: int = 10) -> List[Belief]:
        """Retrieve most relevant beliefs for a query"""

        # 1. Semantic search (vector similarity)
        query_embedding = self.embedder.encode(query)
        semantic_results = self.db.execute("""
            SELECT belief_id, distance
            FROM belief_vectors
            WHERE vss_search(embedding, ?)
            LIMIT ?
        """, (query_embedding, top_k * 2))

        candidates = [r[0] for r in semantic_results]

        # 2. Prolog expansion (related beliefs)
        related = []
        for belief_id in candidates:
            # Find supporting beliefs
            supporting = self.prolog.query(f"supporting_beliefs({belief_id}, S)")
            # Find contradicting beliefs
            contradicting = self.prolog.query(f"has_conflict({belief_id}, C)")
            related.extend(supporting + contradicting)

        # 3. Deduplicate and fetch full belief objects
        all_belief_ids = list(set(candidates + related))
        beliefs = self.fetch_beliefs(all_belief_ids)

        # 4. Rank by composite score
        ranked = self.rank_beliefs(
            beliefs,
            query_embedding=query_embedding,
            weights={'similarity': 0.5, 'confidence': 0.3, 'recency': 0.2}
        )

        # 5. Return top-k with provenance
        return ranked[:top_k]

    def rank_beliefs(self, beliefs, query_embedding, weights):
        """Composite ranking: similarity × confidence × recency"""
        scored = []
        for belief in beliefs:
            similarity = cosine_similarity(belief.embedding, query_embedding)
            confidence = belief.confidence
            recency = self.compute_recency_score(belief.last_validated)

            score = (
                weights['similarity'] * similarity +
                weights['confidence'] * confidence +
                weights['recency'] * recency
            )
            scored.append((score, belief))

        return [b for (s, b) in sorted(scored, reverse=True)]
```

---

## Implementation Roadmap

### **Phase 1: Core Fixes (1-2 weeks)**
1. Add `belief_relationships` table
2. Add `belief_history` table
3. Implement Prolog relationship queries
4. Build confidence update mechanism (uses Prolog rules)

### **Phase 2: Semantic Search (1-2 weeks)**
1. Install sqlite-vss extension
2. Generate embeddings for existing beliefs/observations
3. Build embedding pipeline for new content
4. Test semantic search queries

### **Phase 3: Retrieval Layer (2-3 weeks)**
1. Design retrieval API
2. Implement hybrid retrieval (embeddings + Prolog)
3. Build ranking algorithm
4. Create prompt construction pipeline

### **Phase 4: Multi-Domain (2-3 weeks)**
1. Design domain ontology (flat tags or hierarchy?)
2. Build cross-domain belief detection
3. Implement project scoping
4. Test global persona + project views

### **Phase 5: Temporal Dynamics (1-2 weeks)**
1. Implement belief history tracking
2. Build temporal decay rules
3. Add belief evolution queries
4. Create timeline visualizations

---

## Open Questions

1. **Domain structure:** Flat tags or hierarchical ontology?
2. **Prolog scope:** Just confidence rules or full reasoning engine?
3. **Project architecture:** Global + views or federated?
4. **Embedding model:** Which sentence transformer? Fine-tune?
5. **Relationship discovery:** Manual or automatic (LLM-detected)?
6. **Temporal decay:** Linear, exponential, or threshold-based?
7. **Conflict resolution:** How to handle contradicting beliefs?
8. **Belief versioning:** Keep old versions or just history table?

---

## Conclusion

**What's working:**
- ✅ Neuro-symbolic split (LLM + Prolog + SQLite)
- ✅ Evidence linking
- ✅ Confidence scoring with Prolog rules

**Critical gaps:**
- ❌ No semantic search (need embeddings)
- ❌ No belief graph (need relationships table)
- ❌ No retrieval layer (how LLM uses beliefs)
- ❌ No temporal dynamics (belief evolution)
- ❌ Domain organization unclear

**Bottom line:** The foundation is solid, but the system can't function without:
1. Embeddings (semantic search)
2. Belief relationships (graph reasoning)
3. Retrieval layer (LLM integration)

**Recommendation:** Focus next sprint on Phase 1 + Phase 2 (relationships + embeddings). These unblock everything else.
