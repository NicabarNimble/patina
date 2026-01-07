# Spec: Mothership Architecture

**Status:** Active (Parent Spec)
**Created:** 2026-01-01
**Purpose:** Define Mother as the nervous system connecting all project islands

## Spec Hierarchy

This is the **parent architecture spec**. Implementation is done via focused child specs:

```
spec-mothership.md (this file - vision + phases)
│
├── Phase 0-0.25c: Git Narrative + Measurement ✅ (implemented here)
│
├── Phase 2: Knowledge Graph
│   └── spec-mothership-graph.md ✅ COMPLETE
│       Tag: spec/mothership-graph
│       Delivered: graph.db, routing, weight learning (~1000 lines)
│
└── Content Layer (complements routing)
    └── spec-ref-repo-semantic.md ← CURRENT FOCUS
        Fixes: ref repos lack semantic.usearch
        Solution: commit-based training pairs
```

**Key insight:** Graph routing (WHERE to search) needs semantic content (WHAT to find). Both specs are required for cross-project queries to work well.

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

## Phased Implementation

Only proceed to Phase N+1 when Phase N proves value.

### Phase 0: Git Narrative (Current Focus)

**Problem:** Ref repos have full git history (47k commits, 540k co-change) but we can't search it effectively.

**Solution:** Add commit messages to FTS5, compute moments as temporal signals.

**Deliverables (ALL required, no deferrals):**

| # | Component | Description | Status |
|---|-----------|-------------|--------|
| 1 | `commits` table | Populated by scrape | ✅ Done |
| 2 | `commits_fts` | FTS5 index on commit messages | ✅ Done |
| 3 | scrape integration | Populate commits_fts during scrape | ✅ Done |
| 4 | LexicalOracle | Extend to search commits_fts | ✅ Done |
| 5 | `moments` table | Schema for derived temporal signals | ✅ Done |
| 6 | `assay derive-moments` | Compute genesis/breaking/migration/etc | ✅ Done |

**Exit Criteria (ALL passed 2026-01-02):**

```bash
# Test 1: Commit search works ✅
patina scry "breaking" --repo dojo --hybrid
# → Returns 6 commits with "breaking" in message

# Test 2: Moments detected ✅
patina assay --repo dojo derive-moments
# → 380 moments: 1 genesis, 12 big_bang, 27 major, 3 breaking, 37 migration, 300 rewrite

# Test 3: Cross-repo narrative query ✅
patina scry "cairo migration" --repo dojo --hybrid
# → Returns commit "Migrate to Cairo 2.1.0 (#675)"
```

**Success Metric:** Retrieval precision on "how/when/why" questions against ref repos.

### Phase 0.25: Measurement Before Tuning

**Problem:** We built git narrative infrastructure but have no ground truth to evaluate it. The existing queryset tests code retrieval, not temporal/narrative questions. Hardcoded moment keywords are brittle - patterns should emerge from data.

**Principle:** Measure first, tune second. Learn from sessions, don't invent.

**Insight:** Sessions are both the source of queries AND the ground truth:
- Session goals contain natural queries ("fix X", "understand Y", "add Z")
- Session content contains the answers (decisions, rationale, outcomes)
- Session ↔ commit linkage (via tags) provides query → document mapping

**Deliverables:**

| # | Component | Description | Status |
|---|-----------|-------------|--------|
| 1 | `eval/temporal-queryset.json` | Derived from real sessions, not invented | |
| 2 | Session query extraction | Mine session goals/content for temporal patterns | |
| 3 | Query log mining | Extract patterns from actual scry usage | |
| 4 | Moments v2 schema | Data-driven detection, not hardcoded keywords | |
| 5 | Baseline metrics | MRR/Recall on temporal queryset before tuning | |

**Session-Derived Queryset Strategy:**

```
Sessions (243 in patina)
        │
        ├── Goals → Proto-queries ("add MCP support", "fix retrieval bug")
        │
        ├── Activity Log → Context (what was done, why)
        │
        └── Git Tags → Ground truth (which commits answer this query)

Example derivation:
  Session: "20250803-081714.md"
  Goal: "Add MCP server implementation"
  Commits: [abc123, def456] (linked via session tags)

  → Query: "MCP server implementation"
  → Relevant: [abc123, def456, session content]
```

**Moments v2: Learned Patterns**

Current (hardcoded):
```sql
WHEN LOWER(c.message) LIKE '%breaking%' THEN 'breaking'  -- brittle
```

Proposed (data-driven):
```sql
-- Moment vocabulary learned from actual usage, not hardcoded
CREATE TABLE moment_vocabulary (
    term TEXT PRIMARY KEY,
    moment_type TEXT,        -- learned category (may evolve)
    frequency INTEGER,       -- how often this term appears in high-signal commits
    source TEXT,             -- 'session', 'query', 'commit_cluster'
    confidence REAL          -- learned weight
);
```

**Learning Sources:**

1. **Session-Commit Linkage**: Sessions that touch many files often describe significant moments
   ```sql
   -- Extract high-signal terms from sessions with large file deltas
   SELECT session_id, goal, file_count
   FROM sessions s
   JOIN session_tags t ON s.id = t.session_id
   WHERE file_count > 20
   -- → Terms from these goals become moment vocabulary
   ```

2. **Query Logs**: What temporal patterns do users actually search for?
   ```sql
   -- Extract terms from queries that matched moments/commits
   SELECT query, hit_type, hit_count
   FROM query_log
   WHERE hit_type IN ('git.commit', 'moment')
   -- → Frequently searched terms inform vocabulary
   ```

3. **Commit Message Clustering**: Let the data reveal natural categories
   - Embed all commit messages
   - Cluster by semantic similarity
   - Label clusters by most frequent terms
   - Clusters become moment types (not predetermined categories)

**Bootstrap Strategy:**
- Start with current hardcoded terms as seed vocabulary
- Log which terms actually get searched/matched
- Promote high-frequency terms, demote unused ones
- After N queries, rebuild vocabulary from data

**Anti-Pattern:** Don't try to predict all possible keywords. Let usage teach the system what matters.

**Exit Criteria:**
- [x] 20+ temporal queries derived from real sessions → 10 queries (start small)
- [x] Each query has ground truth (session + commits)
- [x] Baseline MRR measured on temporal queryset
- [ ] Moments detection uses learned patterns, not hardcoded keywords

**Baseline Results (2026-01-02):**

```
📊 patina-temporal-retrieval (10 queries)
   MRR:        0.133   ← First relevant at rank ~7.5
   Recall@5:   15.0%
   Recall@10:  23.3%   ← Finding only 1/4 of relevant docs
   Quality:    ❌ Needs improvement
```

**What the Baseline Reveals:**

| Query Type | Count | Performance | Gap |
|------------|-------|-------------|-----|
| when | 2 | RR=0.0, 0.2 | Commits not surfacing |
| why | 4 | RR=0.0, 0.0, 0.17, 0.0 | Sessions found but wrong ones |
| how | 2 | RR=0.5, 0.0 | Mixed - code found, rationale missing |
| what | 2 | RR=0.33, 0.0 | Patterns found, context missing |

**Root Causes Identified:**

1. **FTS5 drops temporal signal words**: "when", "why", "how" are stop-worded out
2. **Commit messages sparse**: Query terms often don't match commit wording
3. **Session semantic matching**: Finds sessions but often wrong context
4. **No temporal weighting**: Recent sessions not boosted

**Improvement Levers:**

| Lever | Current | Potential Change |
|-------|---------|------------------|
| FTS5 query prep | Drops stop words | Preserve temporal keywords |
| Commit boosting | Same weight as code | Higher weight for temporal queries |
| Session recency | Not weighted | Boost recent sessions for "when" |
| Moments oracle | Not in retrieval | Add MomentsOracle for temporal |

**Success Metric:** Temporal queryset MRR baseline established (0.133). Target: MRR > 0.4 before A/B testing.

---

### Phase 0.25b: Intent-Aware Retrieval (The Long-Term Fix)

**Problem:** Query intent determines optimal retrieval strategy, but we use uniform oracle weights for all queries.

**Insight:** We have a frontier LLM as the UI. The LLM understands query intent and can signal it to the retrieval system.

**Solution:** Add `intent` parameter to scry. Intent drives oracle weighting.

```
mode   = HOW to present results (list, orient, explain)
intent = WHAT kind of answer is needed (temporal, rationale, mechanism)
```

**Architecture:**

```
User Query: "when did we add commit message search"
                │
                ▼
┌───────────────────────────────────────────────────────┐
│  LLM (Claude/Gemini/OpenCode)                         │
│                                                       │
│  Understands: user wants temporal information         │
│  Calls: scry(query="...", intent="temporal")          │
└───────────────────────────────────────────────────────┘
                │
                ▼
┌───────────────────────────────────────────────────────┐
│  QueryEngine with Intent-Aware Weights                │
│                                                       │
│  intent=temporal → commits↑ sessions↑ code↓          │
│  intent=rationale → sessions↑ patterns↑ commits↓    │
│  intent=mechanism → code↑ patterns↑ sessions↓       │
└───────────────────────────────────────────────────────┘
                │
                ▼
Results optimized for intent
```

**MCP Schema Change:**

```json
{
  "name": "scry",
  "inputSchema": {
    "properties": {
      "query": { "type": "string" },
      "intent": {
        "type": "string",
        "enum": ["general", "temporal", "rationale", "mechanism", "definition"],
        "default": "general",
        "description": "Query intent - guides oracle weighting:
          - general: balanced search (default)
          - temporal: when/history questions (boosts commits, sessions)
          - rationale: why/decision questions (boosts sessions, patterns)
          - mechanism: how/implementation questions (boosts code, patterns)
          - definition: what-is questions (boosts patterns, code)"
      }
    }
  }
}
```

**Oracle Weight Matrix:**

```rust
pub struct IntentWeights {
    semantic: f32,   // Code embeddings
    lexical: f32,    // FTS5 (code, commits, patterns)
    temporal: f32,   // Co-change graph
    persona: f32,    // Beliefs
}

impl IntentWeights {
    pub fn for_intent(intent: QueryIntent) -> Self {
        match intent {
            General    => Self { semantic: 1.0, lexical: 1.0, temporal: 1.0, persona: 1.0 },
            Temporal   => Self { semantic: 0.5, lexical: 2.0, temporal: 1.5, persona: 0.5 },
            Rationale  => Self { semantic: 1.0, lexical: 1.5, temporal: 0.5, persona: 1.5 },
            Mechanism  => Self { semantic: 1.5, lexical: 1.0, temporal: 0.5, persona: 0.5 },
            Definition => Self { semantic: 1.0, lexical: 1.5, temporal: 0.3, persona: 1.0 },
        }
    }
}
```

**Implementation (RRF modification):**

Current RRF: `score = 1 / (k + rank)` for all oracles equally

Intent-aware RRF: `score = weight[oracle] * 1 / (k + rank)`

Oracles with higher weight for this intent contribute more to final ranking.

**Deliverables:**

| # | Component | Description | Status |
|---|-----------|-------------|--------|
| 1 | `QueryIntent` enum | temporal, rationale, mechanism, definition, general | |
| 2 | `IntentWeights` | Per-intent oracle weight matrix | |
| 3 | `rrf_fuse_weighted` | RRF with oracle-specific weights | |
| 4 | MCP `intent` param | Expose intent in scry tool | |
| 5 | Benchmark by intent | Measure MRR for each intent category | |

**Exit Criteria:**
- [ ] Temporal intent MRR > 0.4 (baseline: 0.133)
- [ ] Other intents don't regress from general baseline
- [ ] LLM naturally uses intent parameter

**Why This Is Durable:**
1. Intent categories are stable (how humans ask questions)
2. Weights tuned once from queryset, persist forever
3. MCP is universal across all LLM frontends
4. New content types just get assigned weights
5. Measurable per-intent (Andrew Ng style)

---

### Patina as Trusted Advisor (MCP Architecture)

Patina serves as the intelligence layer via MCP. The LLM frontend calls Patina tools, and Patina does the smart work in the backend.

```
User Question → LLM Frontend → MCP: scry(query) → PATINA (brain)
                                                      │
                                          ┌───────────┴───────────┐
                                          │ • Intent detection    │
                                          │ • Oracle weighting    │
                                          │ • Result ranking      │
                                          │ • All measurable      │
                                          └───────────────────────┘
```

**Intent Detection in Scry:**

```rust
fn detect_intent(query: &str) -> QueryIntent {
    let q = query.to_lowercase();

    // Temporal: when, history, added, changed
    if q.contains("when") || q.contains("added") || q.contains("history") {
        return QueryIntent::Temporal;
    }

    // Rationale: why, decided, chose, reason
    if q.contains("why") || q.contains("decided") || q.contains("reason") {
        return QueryIntent::Rationale;
    }

    // Mechanism: how X works
    if q.contains("how") && q.contains("work") {
        return QueryIntent::Mechanism;
    }

    QueryIntent::General
}
```

**Next:** Implement intent detection + weighted RRF, measure on temporal queryset.

---

### Phase 0.25c: Commit-Derived Measurement (Ref Repos)

**Problem:** Phase 0.25 uses session-derived querysets, but ref repos have no sessions. We need deterministic ground truth that works for any codebase with git history. Additionally, tools like `scry recent` and `context topic` have query model mismatches (identified in session 20251220-092623) that need measurable fixes.

**Principle:** Git commits are deterministic ground truth. Commit message = query, files changed = expected results.

**Insight (2026-01-05 session):** Every commit is a query/answer pair:
- **Query**: The commit message (what the developer said they did)
- **Ground Truth**: The files changed (what actually changed)
- **Verification**: `git show --stat` proves the mapping

This is 100% deterministic, requires no manual labeling, and scales to thousands of queries per repo.

**Architecture:**

```
Git History (any repo)
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│  patina bench generate --from-commits --repo dojo           │
│                                                             │
│  For each commit:                                           │
│    query = commit.message (cleaned)                         │
│    relevant_docs = commit.files                             │
│    relevant_commits = [commit.sha]                          │
└─────────────────────────────────────────────────────────────┘
        │
        ▼
eval/dojo-commits-queryset.json (deterministic, reproducible)
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│  patina bench retrieval -q eval/dojo-commits-queryset.json  │
│                                                             │
│  Metrics:                                                   │
│    - Commit-Recall@K: Does query find the source commit?    │
│    - File-Recall@K: Does query find the changed files?      │
│    - MRR: Where does first relevant result appear?          │
└─────────────────────────────────────────────────────────────┘
```

**Queryset Format:**

```json
{
  "name": "dojo-commits-v1",
  "source": "git commits (auto-generated)",
  "generated": "2026-01-05T12:00:00Z",
  "repo": "dojo",
  "queries": [
    {
      "id": "q_0ccbf6a8",
      "query": "add derive-moments command for temporal signals",
      "source_commit": "0ccbf6a8",
      "relevant_docs": [
        "src/commands/assay/internal/derive.rs",
        "src/commands/assay/mod.rs",
        "src/main.rs"
      ],
      "relevant_commits": ["0ccbf6a8"]
    }
  ]
}
```

**Generation Filters (quality over quantity):**

```sql
-- Good queries: meaningful message, reasonable file count
SELECT c.sha, c.message, GROUP_CONCAT(cf.file_path) as files
FROM commits c
JOIN commit_files cf ON c.sha = cf.sha
WHERE length(c.message) > 20           -- Not "fix typo"
  AND length(c.message) < 200          -- Not multi-paragraph
  AND c.message NOT LIKE 'Merge%'      -- Not merge commits
  AND c.message NOT LIKE 'WIP%'        -- Not work-in-progress
GROUP BY c.sha
HAVING COUNT(cf.file_path) BETWEEN 2 AND 15  -- Meaningful change
```

**Fixing Query Model Mismatches:**

| Tool | Current Behavior | Issue | Fix | Measured By |
|------|------------------|-------|-----|-------------|
| `scry recent` | Query = file path LIKE | Users pass NL, expects semantic | Use FTS5 on commits_fts | Commit-Recall@10 |
| `context topic` | Exact substring match | Users pass concepts, expects relevance | Semantic search on patterns | Pattern-Recall@10 |
| `scry --hybrid` | Uniform oracle weights | Different queries need different oracles | Intent-aware weighting | MRR by query type |

**Revised Understanding (2026-01-05 error analysis):**

After measuring baseline (MRR 0.211), we discovered a deeper issue:

```
Query: "add derive-moments command for temporal signals"
Found: commit 0ccbf6a8 at rank #1 ✓ (RR=1.00)
But:   File-Recall@10 = 33% (only 2 of 6 files found)
```

**The commit is found. The files are not followed.**

Current retrieval flow:
```
Query → LexicalOracle → finds commit → returns commit as result
                      → finds code symbols → returns unrelated symbols
```

Needed flow:
```
Query → LexicalOracle → finds commit → EXPAND to commit's files
                                     → boost those files in results
```

This is architectural, not tuning. The fix is **commit→file expansion** in the retrieval layer:

1. When a commit matches via commits_fts, look up its files from commit_files table
2. Inject those files as additional results with boosted score
3. RRF fusion will naturally rank them higher

This is similar to how TemporalOracle uses co-changes - we follow relationships, not just match text.

**Measurement Loop:**

```
1. GENERATE   →  patina bench generate --from-commits --repo X
                 (creates eval/X-commits-queryset.json)

2. BASELINE   →  patina bench retrieval -q eval/X-commits-queryset.json
                 (establishes: MRR 0.23, File-Recall@10 31%)

3. FIX        →  Change ONE thing (e.g., recent mode uses FTS5)

4. MEASURE    →  patina bench retrieval -q eval/X-commits-queryset.json
                 (proves: MRR 0.41 (+78%), File-Recall@10 52% (+68%))

5. REPEAT     →  Next fix, same queryset, cumulative improvement
```

**Deliverables:**

| # | Component | Description | Status |
|---|-----------|-------------|--------|
| 1 | `bench generate --from-commits` | Generate queryset from git history | ✅ Done |
| 2 | Baseline measurement | MRR/Recall on patina-commits-v1 | ✅ Done |
| 3 | Commit→file expansion | Follow commit matches to their files | ✅ Done |
| 4 | `--repo` flag for bench | Enable ref repo benchmarking | ✅ Done |
| 5 | Ref repo baselines | MRR/Recall for dojo, opencode, SDL | ✅ Done |
| 6 | `context topic` fix | Semantic search instead of substring | Pending |

**Results (2026-01-05):**

| Repo | MRR | Recall@10 | Notes |
|------|-----|-----------|-------|
| patina | 0.511 | 45.7% | +169% from baseline (0.211) |
| SDL | 0.614 | 52.7% | Best performer |
| opencode | 0.501 | 47.5% | |
| dojo | 0.351 | 24.2% | Needs investigation |

Commit→file expansion in `LexicalOracle` follows relationships (like TemporalOracle). See `src/retrieval/oracles/lexical.rs`.

**Exit Criteria:**
- [x] `bench generate --from-commits` produces valid querysets
- [x] Baseline measured for patina (MRR 0.211 → 0.511)
- [x] Commit→file expansion improves Recall@10 by >50% (achieved +169%)
- [x] Add `--repo` flag to `bench retrieval` for ref repo testing
- [x] Baseline measured for 3 ref repos (dojo, opencode, SDL)
- [ ] `context topic` fix improves Pattern-Recall@10 by >20%
- [x] No regression on existing retrieval benchmarks

**Why This Matters:**
1. **Deterministic**: Same queryset, reproducible results
2. **Scalable**: Any repo with git history works
3. **No manual labeling**: Ground truth from git itself
4. **Proves fixes work**: Numbers, not feelings
5. **Catches regressions**: Run on every change

---

### Phase 0.5: Persona Surfaces

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

### Phase 2: Knowledge Graph ✅ COMPLETE

**Goal:** Graph captures relationships.

**Implementation:** See [spec-mothership-graph.md](./spec-mothership-graph.md) (archived as `spec/mothership-graph`)

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

1. **Graph DB choice** - SQLite with JSON? Dedicated graph DB? → **Answered: SQLite** (spec-mothership-graph.md)
2. **Cross-project measurement** - How to prove graph helps? → **Answered: Phase G0** (measure dumb routing first)
3. **Relationship types** - Which types matter? → **Answered: Data-driven** (let error analysis reveal)
4. **Pattern extraction** - LLM-based? Rule-based? Hybrid?
5. **Distillation frequency** - Real-time? Batch? On session end?
6. **Cross-project privacy** - All projects visible to Mother? Opt-in?

---

## Related Specs

**Child Specs (implementations of this vision):**
- [spec-mothership-graph.md](./spec-mothership-graph.md) - **✅ COMPLETE** Phase 2 implementation (tag: `spec/mothership-graph`)
- [spec-ref-repo-semantic.md](./spec-ref-repo-semantic.md) - **CURRENT** Content layer for ref repos

**Architecture:**
- [spec-three-layers.md](./spec-three-layers.md) - Mother layer architecture
- [spec-pipeline.md](./spec-pipeline.md) - Data pipeline (scrape→oxidize/assay→scry)
- [concept-rag-network.md](../concept-rag-network.md) - Projects as RAG nodes vision

**How the specs connect:**
```
User Query → Graph Routing (mothership-graph) → Semantic Search (ref-repo-semantic)
             "route to dojo"                    "find relevant code in dojo"
```
