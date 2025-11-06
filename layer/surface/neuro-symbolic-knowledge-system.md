---
id: neuro-symbolic-knowledge-system
version: 2
status: active
created_date: 2025-11-03
updated_date: 2025-11-06
oxidizer: nicabar
tags: [architecture, neuro-symbolic, integration, sqlite, usearch, prolog, implementation, embedded-prolog]
---

# Neuro-Symbolic Knowledge System

**The Vision**: A hybrid AI system that combines neural (semantic search), symbolic (logical rules), and storage (relational) layers to build a persona belief system through interactive dialogue.

**Current Phase**: Phase 2.7 Complete - Production integration achieved

**Timeline**:
- ✅ Phase 1: Core persona system (SQLite + Prolog orchestration)
- ✅ Phase 2: Semantic evidence discovery (USearch integration, multi-source observations)
- ✅ Phase 2.7: Embedded Prolog integration (ReasoningEngine + CLI + persona workflow)
- ⏳ Phase 3: Full dataset extraction (260 sessions)
- ⏳ Phase 4: Global persona (cross-project beliefs)

**Integration Status**: **Production neuro-symbolic integration achieved.** Rust embeds Prolog as library, symbolic rules reason over neural search results through dynamic fact injection. ReasoningEngine powers both CLI (`patina belief validate`) and persona workflow (`/persona-start`). Zero shell overhead, LLM-agnostic design.

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

### System Architecture (Current Implementation)

```
User: /persona-start (kick back, dialogue)
         ↓
    Rust Application (patina):
         ↓
┌────────┴──────────────────┐
│                           │
│  1. Evidence Search       │
│  ┌─────────────────────┐  │
│  │ ObservationStorage  │  │  Query semantic search
│  │   ↓                 │  │
│  │ SQLite metadata     │  │  Observation metadata
│  │ USearch HNSW index  │  │  Vector similarity
│  └─────────────────────┘  │
│         ↓                 │
│  ScoredObservation[]      │
│         ↓                 │
│  2. Belief Validation     │
│  ┌─────────────────────┐  │
│  │ ReasoningEngine     │  │  Embedded Scryer Prolog
│  │   ↓                 │  │
│  │ load_observations() │  │  Inject as Prolog facts
│  │   ↓                 │  │
│  │ validate_belief()   │  │  Symbolic reasoning:
│  │                     │  │  - Weighted evidence
│  │ Prolog Rules:       │  │  - Quality metrics
│  │ - confidence.pl     │  │  - Contradiction check
│  │ - validation.pl     │  │
│  └─────────────────────┘  │
│         ↓                 │
│  ValidationResult         │
│  {valid, reason, metrics} │
│         ↓                 │
│  3. Belief Storage        │
│  ┌─────────────────────┐  │
│  │ SQLite              │  │  Store belief + evidence
│  │ BeliefStorage       │  │  links with confidence
│  └─────────────────────┘  │
│                           │
└───────────────────────────┘
         ↓
    Belief system grows
    (symbolic validation ensures quality)
```

**Integration Level**: **Production LLM-orchestrated neuro-symbolic workflow.** Scryer Prolog embedded as Rust library, symbolic rules reason over neural search results via dynamic fact injection. ReasoningEngine integrated into both CLI and persona workflow, achieving zero-overhead validation.

### Division of Labor: Flexible vs Rigid Reasoning

**LLM provides flexible reasoning:**
- Semantic analysis (find patterns in observations)
- Strategic synthesis (generate high-value questions)
- Context-aware orchestration (when to search, when to ask)
- Belief candidate generation (formulate beliefs from user responses)
- Observation filtering (select relevant evidence for validation)

**Symbolic layer provides rigid governance (embedded Prolog):**
- ✅ Confidence calculation (evidence count → score)
- ✅ Confidence bounds enforcement (max 0.95, min 0.30)
- ✅ Weighted evidence scoring (similarity × reliability)
- ✅ Evidence quality metrics (strong evidence count, diversity)
- ✅ Automatic belief validation (validates before insertion)
- ✅ Reliability-weighted aggregation (multi-source evidence)
- ⚠️ Contradiction detection (disabled - LLM filters contradictions before passing to Prolog; heuristic detection remains challenging)

**Why both**: LLM is creative but can hallucinate. Prolog is rigid but trustworthy. Together: Creative discovery (neural) + trustworthy validation (symbolic).

**Integration achieved**: Scryer Prolog embedded as Rust library in `ReasoningEngine`. Symbolic rules reason over neural search results through dynamic fact injection. Zero shell overhead achieved - integrated into both CLI and persona workflow.

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

### Phase 2: Semantic Evidence Discovery ✅ COMPLETE

**Completed**: Nov 2025

**Goal**: Add semantic search so Claude finds evidence beyond keyword matching

**What was built**: Full semantic search infrastructure with multi-source observations and reliability tracking

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

#### 2.4 Semantic Query Command (✅ Complete)

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
    "evidence_strength": "strong",
    "source_type": "session_distillation",
    "reliability": 0.85
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

### Phase 2.5: Observation Expansion ✅ COMPLETE

**Completed**: Nov 2025

**Goal**: Enrich observation sources beyond session markdown files.

**Why needed**: Better observations = better beliefs (garbage in = garbage out)

**What was built** (MVP approach):
- Extended `ObservationMetadata` with `source_type` and `reliability` fields
- Updated embeddings command to set source metadata (session_distillation: 0.85)
- Added commit message extraction (commit_message: 0.70, 90-day window, conventional commits)
- Updated semantic query output to display source_type and reliability
- Demonstrated multi-source observations with different reliability scores

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

**Future enhancements** (not yet built):
- Additional source extractors (code patterns, PR reviews, documentation, comments)
- Contradiction detection between sources
- CLI for selective source extraction (`patina observe extract --source commits`)

**Success criteria met**:
- ✅ Multiple observation sources captured (sessions + commits)
- ✅ Source reliability tracked in metadata
- ⏳ Contradictions detection (future - LLM can identify manually for now)

---

### Phase 2.6: Strategic Questioning ✅ COMPLETE

**Completed**: Nov 2025

**Goal**: Graph-aware question generation that maximizes information gain.

**Why cutting-edge**: This is the real innovation - one strategic question updates multiple beliefs.

**What was built** (LLM-powered approach):
- No new code needed! LLMs can already do this with existing tools
- Updated persona-start instructions with strategic questioning workflow
- Demonstrates how to find observation clusters using semantic search
- Shows how to generate high-value questions that update multiple beliefs
- Example: Security practices cluster → strategic question about PII handling

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

**Key insight**: LLMs can already perform clustering, relationship detection, and strategic question generation using semantic search. Building autonomous systems for this would duplicate LLM capabilities.

**How it works** (using existing tools):
1. LLM runs `patina query semantic "topic"` to find related observations
2. LLM analyzes results: source types, reliability scores, evidence strength
3. LLM identifies clusters and relationships (chains, contradictions, reinforcement)
4. LLM generates strategic question targeting the cluster
5. User answer updates multiple beliefs across domains

**Success criteria met**:
- ✅ LLM can identify observation clusters (via semantic search)
- ✅ LLM generates strategic questions (via analysis + synthesis)
- ✅ One question updates multiple beliefs (workflow documented in persona-start)
- ✅ No new code needed (follows "tools FOR LLMs" philosophy)

---

### Phase 2.7: Embedded Prolog Integration ✅ COMPLETE

**Completed**: Nov 6, 2025

**Goal**: Embed Scryer Prolog as Rust library to enable true neuro-symbolic integration - symbolic rules reasoning over neural search results.

**Why needed**: Current system shells out to `scryer-prolog` binary. Embedding enables zero-overhead Prolog queries and dynamic fact injection from Rust.

**What was built**:

#### 2.7.1 ReasoningEngine (src/reasoning/engine.rs)
- **Embedded Scryer Prolog**: Uses `scryer-prolog` crate as library (no shell calls)
- **Dynamic fact injection**: `load_observations()` converts semantic search results to Prolog facts
- **Confidence calculation**: `calculate_confidence()` queries embedded Prolog for evidence-based scoring
- **Belief validation**: `validate_belief()` runs Prolog validation rules with quality metrics
- **7 passing tests**: Engine creation, confidence calculation, observation injection, validation logic

#### 2.7.2 Validation Rules (.patina/validation-rules.pl)
- **Weighted evidence scoring**: `similarity × reliability` aggregation across observations
- **Strong evidence counting**: Tracks high-quality observations (sim≥0.70, rel≥0.70)
- **Quality metrics**: Average reliability, average similarity, source diversity
- **Validation logic**: Adequate evidence (score≥3.0) vs weak evidence (score<3.0)
- **Custom utilities**: `member/2`, `sum_list/2`, `list_length/2` (Scryer doesn't auto-load stdlib)

**Validation Threshold Rationale**:
- **3.0 score** = ~4 medium-strength observations (0.75 × 0.75 = 0.56 each) OR 2-3 strong observations
- **5.0 score** = Sufficient strong evidence tier, requires multiple high-quality observations
- Thresholds are tunable via Prolog rules (not hardcoded in Rust)
- Based on testing with realistic observation data from session extractions
- Example: 5 observations @ 0.85 sim × 0.80 rel = 3.4 weighted score → valid (adequate_evidence)

**Architecture**: Dynamic fact injection approach (not FFI)

Scryer's FFI is designed for loading external `.dylib`/`.so` C libraries. We use a simpler, safer approach:

1. **Rust runs semantic search**: `ObservationStorage::search()` returns `Vec<ScoredObservation>`
2. **Inject as Prolog facts**: `engine.load_observations(&observations)` via `consult_module_string()`
3. **Prolog reasons over facts**: Validation rules calculate metrics, check evidence quality
4. **Return structured results**: `ValidationResult` with validity, reason, and metrics

**Example usage**:
```rust
use patina::reasoning::{ReasoningEngine, ScoredObservation};

// 1. Initialize engine (loads confidence + validation rules)
let mut engine = ReasoningEngine::new()?;

// 2. Run semantic search (neural layer)
let observations = observation_storage.search("security practices", 20)?;

// 3. Convert to scored observations
let scored_obs: Vec<ScoredObservation> = observations.iter()
    .map(|o| ScoredObservation {
        id: o.id.to_string(),
        content: o.content.clone(),
        similarity: o.similarity,
        reliability: o.reliability,
        source_type: o.source_type.clone(),
        observation_type: o.observation_type.clone(),
    })
    .collect();

// 4. Inject observations as Prolog facts
engine.load_observations(&scored_obs)?;

// 5. Validate belief (symbolic layer reasons over neural results)
let result = engine.validate_belief()?;

// Result includes:
// - valid: bool (meets evidence threshold?)
// - reason: string ("adequate_evidence", "weak_evidence", etc.)
// - weighted_score: f32 (sum of similarity × reliability)
// - strong_evidence_count: usize (high-quality observations)
// - has_diverse_sources: bool (multiple source types?)
// - avg_reliability: f32
// - avg_similarity: f32
```

**ReasoningEngine capabilities**:
- ✅ Zero shell overhead achieved (embedded Prolog, no external processes)
- ✅ Symbolic rules reason over neural search results
- ✅ Automatic validation with quality metrics
- ✅ Type-safe Rust ↔ Prolog interface
- ✅ Compile-time rule loading via `include_str!()`
- ✅ Production integration: CLI + persona workflow

**Why dynamic facts over FFI**:
- Simpler: No C FFI, no external build artifacts
- Safer: Pure Rust, no `unsafe` pointers
- Achieves goal: Prolog still reasons over semantic search results
- Flexible: LLM orchestrates (Rust injects → Prolog reasons → Rust validates)

**Success criteria met**:
- ✅ Scryer Prolog embedded as library (not external process)
- ✅ Confidence calculation works in-process
- ✅ Belief validation uses Prolog rules
- ✅ Symbolic reasoning over neural search results
- ✅ Quality metrics extraction (weighted scores, diversity)
- ✅ Comprehensive tests (7/7 passing)

#### 2.7.3 CLI Integration (src/commands/belief/validate.rs)
**Command**: `patina belief validate <QUERY>`

Exposes ReasoningEngine via command-line interface for neuro-symbolic validation.

**Usage**:
```bash
patina belief validate "I prefer Rust for systems programming" \
  --min-score 0.50 \
  --limit 20
```

**Output** (JSON):
```json
{
  "query": "I prefer Rust for systems programming",
  "valid": true,
  "reason": "adequate_evidence",
  "metrics": {
    "weighted_score": 3.45,
    "strong_evidence_count": 4,
    "has_diverse_sources": true,
    "avg_reliability": 0.78,
    "avg_similarity": 0.82
  },
  "observations": [
    {"id": "...", "content": "...", "similarity": 0.85, "reliability": 0.80, ...},
    ...
  ]
}
```

**Workflow**:
1. Creates embedder and semantic search engine
2. Embeds query and searches observations (neural layer)
3. Filters by min_score and converts to `ScoredObservation`
4. Initializes `ReasoningEngine` and loads observations
5. Runs `validate_belief()` (symbolic layer)
6. Returns structured JSON result

**First production integration** of ReasoningEngine - proves neuro-symbolic pipeline works end-to-end.

#### 2.7.4 Persona Workflow Integration (resources/claude/persona-start.{sh,md})
**Command**: `/persona-start` (Claude adapter)

Replaced shell-based `scryer-prolog` calls with `patina belief validate` CLI command.

**Before** (shell-based):
```bash
# Old: Shell out to external Prolog binary
CONFIDENCE=$(scryer-prolog .patina/confidence-rules.pl \
  -g "query_initial_confidence($EVIDENCE_COUNT, C), write(C), halt.")
```

**After** (embedded):
```bash
# New: Use embedded ReasoningEngine via CLI
RESULT=$(patina belief validate "belief statement" --min-score 0.50 --limit 20)
VALID=$(echo "$RESULT" | jq -r '.valid')
WEIGHTED_SCORE=$(echo "$RESULT" | jq -r '.metrics.weighted_score')
```

**Validation workflow** (Step 7 in persona-start.md):
1. Run `patina belief validate` → semantic search + Prolog validation
2. Parse JSON result: `valid`, `reason`, `weighted_score`, `strong_evidence_count`
3. Use metrics to inform confidence:
   - `weighted_score >= 5.0` → high confidence (0.85-0.95)
   - `weighted_score >= 3.0` → moderate confidence (0.70-0.85)
4. Only codify if `valid == true` (symbolic layer enforces rules, not LLM judgment)

**LLM-agnostic design**: Uses CLI command, enabling future adapters (Gemini, local models) to use same validation workflow.

**Adapter changes**:
- Added `persona-start.sh` and `persona-start.md` templates to `resources/claude/`
- Updated `session_scripts.rs` to copy templates during `patina init --llm=claude`
- Added `/persona-start` command to Claude adapter (6 commands total)
- Claude adapter version: 0.6.0 → 0.7.0

#### 2.7.5 End-to-End Integration Tests (tests/neuro_symbolic_integration.rs)
**3 test scenarios** (all passing):

1. **Strong evidence validation**: 6 high-similarity observations → `valid: true`
   - Proves symbolic layer accepts adequate evidence
   - Weighted score >= 3.0, multiple strong observations

2. **Weak evidence rejection**: 2 low-quality observations → `valid: false`
   - Proves symbolic layer rejects insufficient evidence
   - Demonstrates threshold enforcement

3. **Confidence calculation**: Tests 0, 1, 2, 3, 5 evidence counts
   - Verifies Prolog rules: 0.50 baseline, 0.15/0.10 increments, 0.85 cap

**Integration proof**: SemanticSearch → ReasoningEngine → ValidationResult pipeline works end-to-end.

---

## Neuro-Symbolic Integration - Production Ready

**Status**: Production neuro-symbolic integration achieved. Built on embedded Scryer Prolog with dynamic fact injection, integrated into both CLI and persona workflows.

**How it works**:

Scryer Prolog **is written in Rust** and we've embedded it as a library. Components built:

1. **✅ Embed Prolog in Patina** - Using `scryer-prolog` crate as library (enables zero shell overhead)
2. **✅ Dynamic fact injection** - Rust converts semantic search results to Prolog facts via `consult_module_string()`
3. **✅ Prolog validates neural search results** - Automatic validation, quality metrics, evidence weighting
4. **✅ CLI integration** - `patina belief validate` command exposes ReasoningEngine
5. **✅ Persona workflow integration** - `/persona-start` uses CLI command, replacing shell-based Prolog
6. **✅ End-to-end tests** - 3/3 passing integration tests prove neuro-symbolic pipeline

**What we implemented** (Phase 2.7):

```rust
// Rust orchestrates the neuro-symbolic workflow
let mut engine = ReasoningEngine::new()?;

// Neural layer: semantic search
let observations = storage.search("security practices", 20)?;

// Bridge: convert to Prolog facts
engine.load_observations(&scored_observations)?;

// Symbolic layer: validate with rules
let result = engine.validate_belief()?;
// Returns: {valid, reason, weighted_score, metrics}
```

**Behind the scenes**:
```prolog
% Prolog validates based on injected facts
validate_belief(Valid, Reason) :-
    weighted_evidence_score(Score),
    count_strong_evidence(StrongCount),
    (Score >= 3.0, StrongCount >= 2
     -> Valid = true, Reason = 'adequate_evidence'
     ;  Valid = false, Reason = 'weak_evidence').

% Observations are Prolog facts injected by Rust
observation('obs_1', 'pattern', 'Security audits', 0.85, 0.85, 'session').
observation('obs_2', 'decision', 'Pre-commit hooks', 0.78, 0.70, 'commit').
```

**ReasoningEngine provides**:
- ✅ Automatic validation (no LLM manual checking needed)
- ✅ Symbolic evidence weighting using similarity × reliability
- ✅ Quality metrics extraction (diversity, averages, strong evidence count)
- ✅ LLM-orchestrated neuro-symbolic workflow (neural search → symbolic validation)

**Why dynamic facts instead of FFI**:

Scryer's FFI is designed for loading external C `.dylib`/`.so` libraries. We use dynamic fact injection instead:

- **Simpler**: No C FFI, no external build artifacts, no `unsafe` code
- **Safer**: Pure Rust, type-safe interface
- **Achieves goal**: Prolog still reasons over semantic search results
- **Flexible**: LLM orchestrates the workflow (Rust → Prolog → Rust)

**Future enhancements** (not yet needed):

If we need Prolog to call Rust functions directly (not just reason over injected facts), we could:
1. Build separate `.dylib` with C-compatible exports
2. Load via Scryer's `use_foreign_module/2`
3. Call Rust functions from Prolog predicates

But current approach handles all validation use cases without this complexity.

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
│       ├── observations.db         # Metadata (SQLite)
│       └── observations.usearch    # HNSW index (USearch)
│
├── confidence-rules.pl             # Prolog confidence scoring (212 lines)
├── validation-rules.pl             # Prolog belief validation (164 lines) ✨ NEW
├── rules.pl                        # Prolog inference rules
└── facts.pl                        # Exported facts (manual)
```

**Rust source** (`src/`):
```
src/
├── reasoning/                      # ✨ NEW - Embedded Prolog integration
│   ├── mod.rs                      # Module exports
│   └── engine.rs                   # ReasoningEngine with Scryer
│
├── storage/                        # Vector + metadata storage
│   ├── observations.rs             # ObservationStorage (SQLite + USearch)
│   └── beliefs.rs                  # BeliefStorage
│
├── embeddings/                     # ONNX embeddings
│   └── mod.rs                      # all-MiniLM-L6-v2
│
└── commands/                       # CLI commands
    ├── query/semantic.rs           # Semantic search command
    └── embeddings/mod.rs           # Embedding generation
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
- `.patina/validation-rules.pl` - Prolog belief validation (✨ NEW)
- `src/reasoning/engine.rs` - ReasoningEngine with embedded Scryer (✨ NEW)
- `src/embeddings/` - ONNX embeddings (working)
- `src/storage/` - Vector layer (BeliefStorage, ObservationStorage)
- `src/commands/embeddings/mod.rs` - Embeddings generation command
- `src/query/semantic_search.rs` - Semantic search API

**Design docs**:
- `neuro-symbolic-knowledge-system.md` (This doc - Phase 2.7 complete)
- `persona-belief-architecture.md` (Oct 25) - Belief system design
- `neuro-symbolic-hybrid-extraction.md` (Oct 24) - Extraction architecture
- `sqlite-usearch-vectors.md` (Nov 2-3) - Vector layer implementation

**Sessions**:
- `20251025-081846.md` - Persona architecture breakthrough
- `20251026-072236.md` - Three-layer integration
- `20251103-111458.md` - Vector storage completion
- `20251105-154337.md` - Phases 2.4-2.6: Semantic query, multi-source observations, strategic questioning
- `20251106-111208.md` - Phase 2.7 components: Embedded Prolog, ReasoningEngine, validation rules (8 commits)
- `20251106-125659.md` - Phase 2.7 integration: Peer review, CLI command, integration tests, persona workflow (4 commits)
