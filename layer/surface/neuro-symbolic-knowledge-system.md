---
id: neuro-symbolic-knowledge-system
version: 1
status: active
created_date: 2025-11-03
updated_date: 2025-11-03
oxidizer: nicabar
tags: [architecture, neuro-symbolic, integration, sqlite, usearch, prolog, implementation]
---

# Neuro-Symbolic Knowledge System

**The Vision**: A hybrid AI system that combines neural (semantic search), symbolic (logical rules), and storage (relational) layers to build a persona belief system through interactive dialogue.

**Current Phase**: Phase 2 - Adding semantic search to existing persona sessions

**Timeline**:
- ✅ Phase 1: Core persona system (SQLite + Prolog orchestration)
- 🚧 Phase 2: Semantic evidence discovery (USearch integration) ← **WE ARE HERE**
- ⏳ Phase 3: Full dataset extraction (260 sessions)
- ⏳ Phase 4: Global persona (cross-project beliefs)

---

## System Overview

### The Complete Vision

**Purpose**: Build a belief system that mirrors user preferences by analyzing their development sessions, so AI assistants can make aligned suggestions without constant re-explanation.

**How it works**:
1. **Observation Layer**: Sessions are scraped for patterns, decisions, technologies used
2. **Persona Sessions**: User has relaxed dialogue with AI, one question at a time
3. **Evidence Discovery**: System searches all history (semantic + keyword) for supporting evidence
4. **Belief Codification**: Prolog calculates confidence, SQLite stores beliefs with evidence links
5. **Future AI Reference**: Build sessions query beliefs for context

**The Three Layers**:
- **Neural (USearch)**: Semantic similarity - finds "code audit" when you say "security review"
- **Symbolic (Prolog)**: Logical rules - enforces confidence scoring, can't be overridden by LLM
- **Storage (SQLite)**: Source of truth - observations, beliefs, evidence links

**Core Insight: User-Guided Persona Building FOR LLMs**

Patina combines knowledge capture (observations) and belief extraction (preferences) through interactive dialogue. LLM orchestrates, humans validate, and the symbolic layer governs.

### System Architecture (Complete)

```
User: /persona-start (kick back, dialogue)
         ↓
    Claude orchestrates:
         ↓
┌────────┴───────────┐
│                    │
│  Evidence Search:  │
│  ┌──────────────┐  │
│  │ SQLite       │  │  Keyword search
│  │ USearch      │  │  Semantic search
│  │ Prolog       │  │  Rule inference
│  └──────────────┘  │
│                    │
│  One question      │
│  User answers      │
│                    │
│  Confidence calc:  │
│  ┌──────────────┐  │
│  │ Scryer       │  │  Evidence count → confidence
│  │ Prolog       │  │  (mandatory rules)
│  └──────────────┘  │
│                    │
│  Belief storage:   │
│  ┌──────────────┐  │
│  │ SQLite       │  │  Belief + evidence links
│  │ INSERT       │  │
│  └──────────────┘  │
│                    │
└────────────────────┘
         ↓
    Belief system grows
    (used by future build sessions)
```

### Division of Labor: Flexible vs Rigid Reasoning

**LLM provides flexible reasoning:**
- Semantic analysis (find patterns in observations)
- Strategic synthesis (generate high-value questions)
- Context-aware orchestration (when to search, when to ask)

**Symbolic layer provides rigid governance:**
- Confidence calculation (evidence count → score)
- Validation rules (prevent invalid states)
- Deterministic scoring (cannot be hallucinated)

**Why both**: LLM is creative but can hallucinate. Prolog is rigid but trustworthy. Together: Creative discovery + trustworthy validation.

---

## Implementation Status

### Phase 1: Core Persona System ✅ COMPLETE

**Completed**: Oct 2025

**What works**: `/persona-start` command enables interactive belief extraction

**Components built**:

#### 1.1 Persona Session Scripts
- **Location**: `.claude/bin/persona-start.sh`, `.claude/commands/persona-start.md`
- **What**: Creates context file, orchestrates dialogue
- **Status**: Working

#### 1.2 Prolog Confidence Rules
- **Location**: `.patina/confidence-rules.pl` (212 lines)
- **What**: Mandatory rules for confidence scoring (0.3-0.95 scale)
- **Query interface**: `query_initial_confidence`, `query_confidence_adjustment`, `query_needs_refinement`
- **Status**: Working, Claude shells out to `scryer-prolog`

#### 1.3 SQLite Storage
- **Location**: `.patina/db/facts.db`
- **Tables**: `sessions`, `patterns`, `technologies`, `decisions`, `challenges`, `beliefs`, `belief_observations`
- **What**: Observations + beliefs with evidence linking
- **Status**: Working, 7 sessions extracted manually

#### 1.4 Session Workflow
1. User runs `/persona-start`
2. Claude queries SQLite for observations (keyword search)
3. Claude asks one question
4. User answers
5. Claude shells out to Prolog for confidence calculation
6. Claude inserts belief with evidence links
7. Repeat

**Success criteria met**: ✅ Can extract beliefs through dialogue with evidence-based confidence scoring

---

### Phase 2: Semantic Evidence Discovery 🚧 IN PROGRESS

**Started**: Nov 2025

**Goal**: Add semantic search so Claude finds evidence beyond keyword matching

**Components built**:

#### 2.1 Vector Storage Layer (✅ Complete)
- **Location**: `src/storage/` - BeliefStorage, ObservationStorage
- **What**: Dual storage (SQLite metadata + USearch HNSW indices)
- **Capabilities**: Fast semantic search, persistent memory-mapped indices
- **Status**: 86 tests passing

#### 2.2 ONNX Embeddings (✅ Complete)
- **Location**: `src/embeddings/` - OnnxEmbedder
- **What**: Pure Rust ONNX Runtime with INT8 quantized models (23MB)
- **Model**: all-MiniLM-L6-v2 (384 dimensions)
- **Capabilities**: Cross-platform, on-device, Metal GPU acceleration on Mac
- **Status**: Working

#### 2.3 Embeddings Command (✅ Complete)
- **Location**: `src/commands/embeddings/mod.rs`
- **Command**: `patina embeddings generate`
- **What**: Reads observations from facts.db, generates embeddings, stores in `.patina/storage/observations/`
- **Status**: Working

**What's missing in Phase 2**:

#### 2.4 Semantic Query Command — ❌ Not Built

**Goal**: Enable semantic search for observations in persona sessions.

**Why simple approach**: LLM handles query reformulation, result fusion, and relevance evaluation. Build simple tools that LLMs can compose.

**CLI**:
```bash
patina query semantic "security practices" \
  --type pattern,decision \
  --limit 10 \
  --min-score 0.35
```

**Output (JSON)**:
```json
[
  {
    "id": "obs_172",
    "session_id": "20251008-061520",
    "type": "pattern",
    "text": "Always run security audits before production deploys",
    "similarity": 0.78,
    "evidence_strength": "strong"
  }
]
```

**Implementation**:
1. Call `ObservationStorage::search_with_scores()` (already exists)
2. Filter by `--type`, `--min-score`, `--limit`
3. Map similarity to evidence strength:
   - `sim ≥ 0.70` → `"strong"`
   - `0.50 ≤ sim < 0.70` → `"medium"`
   - `sim < 0.50` → `"weak"`
4. Output JSON

**What we defer** (LLM handles this):
- Query rewrites (LLM generates multiple searches)
- BM25 fusion (SQL LIKE search works for now)
- MMR deduplication (LLM identifies duplicates)
- Reranking (LLM evaluates relevance)

**Success criteria**:
- ✅ Command returns semantic search results
- ✅ LLM can find evidence beyond keyword matching
- ✅ Evidence strength mapping works in persona sessions

---

### Phase 3: Full Dataset Extraction ⏳ FUTURE

**Goal**: Extract all 260 sessions into facts.db (currently only 7 extracted manually)

**Why needed**: More observations = richer persona sessions with better evidence

**Components to build**:
- Session parser (markdown → structured data)
- LLM extraction pipeline (identify patterns/decisions/challenges)
- Batch processor for all sessions
- Command: `patina scrape sessions` or similar

**Success criteria**:
- facts.db contains 260 sessions
- Embeddings generated for all observations
- Persona sessions have comprehensive evidence base

---

### Phase 4: Global Persona ⏳ FUTURE

**Goal**: Accumulate beliefs across all projects in `~/.patina/knowledge.db`

**Why needed**: Learn patterns across projects, not just one codebase

**Open design questions**:
- How to sync project beliefs → global?
- How to handle belief conflicts between projects?
- When to use global vs project-specific beliefs?

**See**: `layer/surface/persona-belief-architecture.md` for initial design

**Note**: Defer until project-level proves valuable

---

### Phase 2.5: Observation Expansion ⏳ FUTURE

**Goal**: Enrich observation sources beyond session markdown files.

**Why needed**: Better observations = better beliefs (garbage in = garbage out)

**Expanded observation sources** (weighted by reliability):

| Source Type | Reliability | Extraction Method | Examples |
|-------------|-------------|-------------------|----------|
| User declarations | 0.95 | Direct statement | `/persona-start` answers |
| Session distillations | 0.85 | LLM extraction | "Key decisions" sections |
| Code patterns | 0.75 | Static analysis | Consistent code style |
| Commit messages | 0.70 | Text analysis | "Always fix tests before commit" |
| Documentation | 0.65 | Document parsing | README, CONTRIBUTING.md |
| Comments | 0.60 | Code scanning | `// IMPORTANT: ...` patterns |
| PR reviews | 0.70 | GitHub API | Repeated feedback themes |
| Issue triaging | 0.65 | Pattern detection | Priority/label patterns |
| External docs | 0.50 | URL references | Bookmarked articles/guides |
| Chat logs | 0.60 | Conversation analysis | Slack/Discord preferences |

**Components to build**:
1. Source registry (session, code, commit, docs, etc.)
2. Source weighting (reliability scores in DB)
3. Multi-source extraction pipeline
4. Contradiction detection (same topic, different sources)

**CLI**:
```bash
patina observe extract \
  --source sessions \
  --source commits \
  --source code-comments \
  --since 90d
```

**Success criteria**:
- Multiple observation sources captured
- Source reliability affects confidence scores
- Contradictions surfaced for user clarification

---

### Phase 2.6: Strategic Questioning ⏳ FUTURE

**Goal**: Graph-aware question generation that maximizes information gain.

**Why cutting-edge**: This is the real innovation - one strategic question updates multiple beliefs.

**Current approach**:
- One observation → One question → One belief
- Linear, local scope

**Graph-aware approach**:
1. **Cluster observations** by semantic similarity + temporal co-occurrence
2. **Identify graph patterns**:
   - Chains (A enables B enables C)
   - Contradictions (A vs B)
   - Reinforcement (A, B, C all support same principle)
3. **Calculate information gain** for potential questions
4. **Ask questions** that maximize belief updates per query

**Example: Security Practices Graph**

```
Observation #1 (session 2025-08-12): "Added pre-commit hook for secret scanning"
Observation #2 (session 2025-09-03): "Rejected PR for hardcoded API key"
Observation #3 (session 2025-10-15): "Spent 2h configuring SOPS for secrets"
Observation #4 (code pattern): "Zero occurrences of 'password' in codebase"
Observation #5 (commit msg): "fix: remove .env from git history"

Graph relationships:
  #1 --[prevents]--> #2
  #3 --[implements]--> #1
  #5 --[fixes_violation_of]--> #1
  #4 --[evidence_of]--> #1

Potential belief: "Never commit secrets" (confidence: 0.95, evidence_count: 5)

Strategic question:
  "I notice you invest heavily in preventing secrets in code (hooks, SOPS,
   cleanup). Do you apply this same rigor to other sensitive data like PII?"

Cascading impact:
  If yes → Updates beliefs about: security, compliance, data handling, tooling
  If no → Splits belief into "secrets vs other sensitive data" (conditional)
```

**Information gain**:
- Single observation question: Updates 1 belief
- Contradiction question: Updates 4+ beliefs, clarifies context
- Strategic question: Updates 8+ beliefs across domains

**Components to build**:
1. Observation graph (relationships between observations)
2. Semantic clustering (group related observations)
3. Contradiction detection engine
4. Information gain calculator
5. Strategic question templates

**CLI**:
```bash
patina observe graph --topic security --suggest-questions
```

**Output**:
```json
{
  "cluster_id": "security_practices_001",
  "observations": [1, 5, 12, 18, 22],
  "pattern": "high_investment_prevention",
  "suggested_question": {
    "text": "Do you apply secret-prevention rigor to other sensitive data?",
    "estimated_belief_updates": 8,
    "topics": ["security", "compliance", "data_handling"]
  }
}
```

**Success criteria**:
- Observation graphs capture relationships
- Strategic questions identified automatically
- One question updates multiple related beliefs

---

## Component Analysis: What's Actually Needed?

The original Phase 2.4 design included many RAG pipeline components. Here's what we actually need:

| Component | Proposed Design | Actually Needed? | Rationale |
|-----------|-----------------|------------------|-----------|
| **Vector Search** | USearch semantic search | ✅ **YES** | Core value - finds "code audit" when searching "security review" |
| **Query Rewrites** | 3-5 paraphrases per query | ❌ NO | LLM does this naturally by reformulating queries |
| **BM25 (FTS5)** | Lexical search, fusion | ⚠️ MAYBE | Nice-to-have, but SQL LIKE works for now |
| **RRF Fusion** | Combine rankings | ❌ NO | LLM combines sources mentally |
| **MMR Deduplication** | Remove near-duplicates | ❌ NO | LLM identifies duplicates |
| **Cross-Encoder Reranker** | ONNX reranking | ❌ NO | LLM evaluates relevance |
| **Evidence Strength Mapping** | similarity → strong/medium/weak | ✅ **YES** | Bridges neural → symbolic layer |
| **Structured Output** | JSON/NDJSON format | ✅ **YES** | Machine-readable for Prolog |

**Verdict**: Only 3 of 8 proposed components are needed. Build simple tools that LLMs can compose.

---

## What's Actually Innovative Here?

**The innovation is NOT the retrieval complexity.**

Standard RAG systems use query rewrites, fusion, reranking. That's table stakes in 2025.

**The innovation IS the neuro-symbolic architecture:**

1. **Interactive belief extraction** - User teaches AI through dialogue
2. **Evidence-based grounding** - Beliefs linked to actual observed behavior
3. **Symbolic governance** - Prolog enforces confidence, prevents hallucination
4. **Adaptive knowledge** - System learns and evolves with user
5. **Local-first privacy** - No cloud RAG, all on-device
6. **LLM orchestration** - LLM shells out to specialized tools (DB, Prolog, embeddings)

**This is cutting-edge because:**
- Most persona systems use prompts (ephemeral, context window limited)
- Most RAG systems lack symbolic validation (hallucination-prone)
- Most agent systems don't ground in evidence (made-up beliefs)
- Most knowledge systems don't adapt to user evolution

Adding RRF/MMR/reranking doesn't make this more innovative. It makes it more complex without addressing the hard problems.

---

## The Real Challenges

**The hard problems are NOT about retrieval quality:**

1. **Belief extraction quality**: Can the LLM generate high-quality strategic questions?
2. **Evidence linking accuracy**: Are the right observations linked to beliefs?
3. **Confidence calibration**: Are Prolog rules giving sensible confidence scores?
4. **Knowledge adaptation**: Do beliefs evolve appropriately as user evolves?
5. **Cross-session learning**: Does the system get smarter over time?

**None of these are solved by better retrieval algorithms.**

**What DOES help:**
- Better embeddings (semantic understanding)
- Better Prolog rules (smarter confidence)
- Better extraction prompts (question quality)
- Better observation capture (richer evidence)
- Strategic questioning (graph-aware information gain)

---

## Current File Structure

**Project-level** (`.patina/`):
```
.patina/
├── db/
│   ├── facts.db                    # Observations + beliefs
│   └── code.db                     # Code symbols (from scrape)
│
├── storage/                        # Vector indices
│   └── observations/
│       ├── observations.db         # Metadata
│       └── observations.usearch    # HNSW index
│
├── confidence-rules.pl             # Prolog confidence rules
├── rules.pl                        # Prolog inference rules
└── facts.pl                        # Exported facts (manual)
```

**Claude adapter** (`.claude/`):
```
.claude/
├── bin/
│   ├── persona-start.sh           # Persona session orchestration
│   └── persona-end.sh             # Session archival
│
└── commands/
    ├── persona-start.md           # Instructions for Claude
    └── persona-end.md
```

---

## Evidence Strength Mapping

Semantic search results are mapped to evidence strength categories for Prolog confidence scoring:

| Similarity Score | Evidence Strength |
|------------------|-------------------|
| `sim ≥ 0.70` | `strong` |
| `0.50 ≤ sim < 0.70` | `medium` |
| `sim < 0.50` | `weak` |

**Prolog handoff schema**:
```prolog
% Evidence facts emitted by query command
evidence(ObservationId, Strength, SessionId, SourcePath, Type).

% Example: Observation #172 is strong evidence
evidence(obs_172, strong, '20251008-061520', 'sessions/20251008-061520.md', pattern).

% Example usage in confidence-rules.pl
strong_evidence_count(ClaimId, Count) :-
    findall(E, evidence(ClaimId, strong, _, _, _), Evidences),
    length(Evidences, Count).
```

---

## Data Versioning

- Store `embedding_model_id` and `content_hash` for every observation
- Refuse mixed indices (enforce model consistency)
- Re-embed observations when model changes:
  ```bash
  patina embeddings reindex --model <new-model-id>
  ```

## Acceptance Criteria (Phase 2.4)

**Simple Semantic Query Command**:
- [ ] `patina query semantic` command implemented
- [ ] JSON output includes: `id`, `session_id`, `type`, `text`, `similarity`, `evidence_strength`
- [ ] Filter by `--type`, `--min-score`, `--limit` flags
- [ ] Evidence strength mapping works (`sim` → `strong`/`medium`/`weak`)
- [ ] Command documented in `/persona-start` instructions
- [ ] Tested in real persona session with Prolog integration

---

## Next Steps

### Immediate: Phase 2.4 - Simple Semantic Query

**Implementation tasks**:
1. Add `src/commands/query/semantic.rs`
2. Call `ObservationStorage::search_with_scores()` (already exists)
3. Filter by `--type`, `--min-score`, `--limit`
4. Map similarity → evidence strength (strong/medium/weak)
5. Output JSON format
6. Update persona-start instructions
7. Test in real persona session

**Why this unblocks value**:
- LLM can find evidence beyond keyword matching
- Enables semantic persona sessions immediately
- Can iterate based on real usage

### Next: Phase 2.5 - Observation Expansion

**Why high value**:
- Better observations = better beliefs (garbage in = garbage out)
- Multi-source validation catches contradictions
- Richer evidence base improves confidence calibration

**What to build**:
- Source registry and weighting
- Multi-source extraction pipeline
- Contradiction detection

### Future: Phase 2.6 - Strategic Questioning

**Why cutting-edge**:
- This is the real innovation - graph-aware information gain
- One question updates multiple beliefs
- Maximizes learning efficiency

**What to build**:
- Observation graphs
- Semantic clustering
- Information gain calculator
- Strategic question generation

---

## Design Decisions

### Why `/persona-start` Works This Way

**Human-in-the-loop by design**:
- User knows their beliefs, system doesn't guess
- One question at a time (conversational, not interrogation)
- User can correct/refine as system learns
- Feels natural, not like filling out a form

**LLM as orchestrator**:
- LLM is good at: evidence search, question generation, synthesis
- LLM is bad at: deciding confidence, storing beliefs
- Prolog enforces rules (confidence calculation)
- SQLite stores truth
- LLM orchestrates but doesn't decide

**Shell-based integration**:
- LLM calls `scryer-prolog` and `sqlite3` directly
- No need for Rust integration layer
- Simpler, more flexible
- Easy to debug (can run commands manually)

### Why USearch + SQLite + Prolog?

**Neural Layer (USearch)**:
- Semantic similarity ("security audit" ≈ "code review")
- Cross-domain discovery (same concepts, different words)
- Finds evidence that keyword search misses

**Symbolic Layer (Scryer Prolog)**:
- Confidence scoring (mandatory rules, not LLM guesses)
- Validation logic (evidence count → confidence)
- Can't be overridden by LLM

**Storage Layer (SQLite)**:
- Source of truth for observations and beliefs
- Relational queries (evidence linking)
- Fast keyword search

**Together**: Each does what it's good at. No single component can do the whole job.

### Why Not Vector DB Server (Qdrant/Chroma)?

**Local-first optimization**:
- No server to run (USearch is library)
- No network overhead
- Simple file-based storage
- Fast enough for single-user scale

**Escape hatch exists**:
- USearch indices can be migrated
- SQLite easy to export
- Not locked in

---

## Testing Strategy

### Unit Tests

**Vector layer** (✅ 86 tests passing):
- BeliefStorage roundtrip
- ObservationStorage search
- Type filtering
- Embedding generation

**Query command** (⏳ to add):
- Semantic search returns results
- Type filtering works
- Output format is parseable

### Integration Tests

**Embeddings workflow** (✅ working):
1. Run `patina embeddings generate`
2. Verify `.patina/storage/observations/` created
3. Query with SemanticSearch API

**Semantic query command** (⏳ to add):
1. Generate embeddings
2. Run `patina query semantic "test"`
3. Verify results match expected observations

### Manual Tests

**Persona session** (✅ testable):
1. Run `/persona-start`
2. Test SQLite queries work
3. Test Prolog confidence queries work
4. Verify belief insertion with evidence links

**Persona session with semantic search** (⏳ requires Phase 2.4):
1. Generate embeddings first
2. Run `/persona-start`
3. Use `patina query semantic` during session
4. Verify semantic evidence discovery works

---

## References

**Working implementation**:
- `.claude/bin/persona-start.sh` - Persona session orchestration (working)
- `.patina/confidence-rules.pl` - Prolog confidence scoring (working)
- `src/embeddings/` - ONNX embeddings (working)
- `src/storage/` - Vector layer (BeliefStorage, ObservationStorage)
- `src/commands/embeddings/mod.rs` - Embeddings generation command
- `src/query/semantic_search.rs` - Semantic search API

**Design docs**:
- `persona-belief-architecture.md` (Oct 25) - Belief system design
- `neuro-symbolic-hybrid-extraction.md` (Oct 24) - Extraction architecture
- `sqlite-usearch-vectors.md` (Nov 2-3) - Vector layer implementation

**Sessions**:
- `20251025-081846.md` - Persona architecture breakthrough
- `20251026-072236.md` - Three-layer integration
- `20251103-111458.md` - Vector storage completion
