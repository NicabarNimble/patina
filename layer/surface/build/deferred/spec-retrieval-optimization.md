# Spec: Retrieval Optimization

**Purpose:** Improve knowledge retrieval for LLM frontends: faster response, fewer tokens, more accurate results.

**Origin:** Session 20260101-070900. Deep code analysis of QueryEngine, oracles, MCP interface.

**Status:** Parked (Phase 0-1 Complete)

---

> **Why Deferred:**
>
> Phase 0-1 shipped with a **6.8x latency improvement** (p50 150ms → 22ms).
> Model-loading bottleneck fixed by having oracles own their embedders.
>
> **Phases 2-4 are parked** - they have data dependencies we don't have yet:
> - Phase 2 (Cache/truncation): Nice-to-have, not blocking
> - Phase 3 (Intent routing): Needs 100+ logged queries (we have 6)
> - Phase 4 (Usage learning): Needs weeks of real usage data
>
> **Resume trigger:** 100+ queries logged in eventlog with oracle contributions.
> Query data accumulates organically while using patina.

---

---

## Philosophy

### Andrew Ng's Pragmatic ML

> "Don't start with the model. Start with the data, the metrics, and a fast iteration loop."

1. **Measure first** - Can't improve what you can't measure
2. **Simple baseline** - Complex beats simple only when you can prove it
3. **Error analysis** - Look at failures, not just aggregate metrics
4. **Ship fast, learn fast** - A deployed system teaches more than a polished spec
5. **Don't break prod** - Wrap changes in feature flags, compare before/after

### Patina Core Values

Every change must honor Patina's core principles:

**unix-philosophy:** One tool, one job, done well. Cache is a separate module. Intent classification is a separate module. Don't bloat QueryEngine.

**dependable-rust:** Tiny stable external interface, push details behind internal module. SemanticOracle should own its embedder (not create per-query).

**adapter-pattern:** Trait-based adapters, runtime selection, black-box implementations. Oracle trait stays minimal.

---

## Problem Statement

When a user asks Claude Code a question, Patina's retrieval determines what context the LLM sees.

| Dimension | Current State | Target | How We'll Measure |
|-----------|--------------|--------|-------------------|
| **Latency** | Unknown (not instrumented) | TBD (verify in Phase 0) | Add timing to QueryEngine |
| **Tokens** | Unknown | TBD after baseline | Character count in format_results |
| **Accuracy** | Unknown baseline | TBD after baseline | Run existing `patina eval` |

---

## Ground Truth: Code-Verified Architecture

Traced the actual MCP → scry flow in the codebase:

```
MCP Server (server.rs:70)
    │
    ├── QueryEngine::new() ← Created ONCE at startup
    │       └── Creates 4 oracles (semantic, lexical, temporal, persona)
    │
    └── engine.query_with_options() ← Called per request
            │
            ├── SemanticOracle.query()
            │       └── scry_text() (search.rs:44)
            │               ├── create_embedder() ← LOADS MODEL EVERY TIME ❌
            │               ├── Projection::load_safetensors() ← LOADS EVERY TIME ❌
            │               └── Index::load() ← LOADS EVERY TIME ❌
            │
            ├── LexicalOracle.query() → FTS5 (fast, ~10ms)
            ├── TemporalOracle.query() → SQL (fast, ~10ms)
            └── PersonaOracle.query() → persona::query() → create_embedder() ❌
```

### The REAL Bottleneck (Code-Verified)

**Not** "100ms embedding latency" — it's **model loading on every query**:

```rust
// src/commands/scry/internal/search.rs:70
let mut embedder = create_embedder()?;  // Loads ONNX model from disk!
```

Also in PersonaOracle:
```rust
// src/commands/persona/mod.rs:127
let mut embedder = create_embedder()?;  // Same issue
```

Loading ONNX model from disk: **estimated 200-500ms per query** (verify in Phase 0)
Actual embedding inference: **estimated ~50-100ms** (verify in Phase 0)

This violates **dependable-rust**: The SemanticOracle should own its embedder, not recreate it per query.

---

## Existing Infrastructure (Use It, Don't Rebuild)

### Eval Command Already Exists

```bash
patina eval              # Precision benchmarks (semantic, temporal)
patina eval --feedback   # Real-world precision from usage data
```

Located: `src/commands/eval/mod.rs` (597 lines, comprehensive)

Features:
- Precision@5 and Precision@10 per dimension
- Random baseline comparison
- Feedback loop: correlates scry queries with subsequent commits
- Precision by rank analysis

### Benchmark Files Already Exist

```
resources/bench/patina-retrieval-v1.json   # 10 query benchmark
resources/bench/patina-dogfood-v1.json     # Additional benchmark
```

**Action:** Run `patina eval` FIRST to get baseline. Don't build new eval infrastructure.

---

## Gaps (Code-Verified)

| Gap | Evidence | Root Cause |
|-----|----------|------------|
| Model loaded per-query | `search.rs:70` calls `create_embedder()` | SemanticOracle doesn't own embedder |
| Projection loaded per-query | `search.rs:75-81` loads safetensors | No caching |
| Index loaded per-query | `search.rs:94-98` calls `Index::load()` | No caching |
| PersonaOracle same issue | `persona/mod.rs:256` loads model per-query | Same pattern |
| PersonaOracle index per-query | `persona/mod.rs:253` loads index | No caching |
| No query timing | QueryEngine has no instrumentation | Never added |
| No intent routing | All queries hit all oracles | No classification |
| Token waste | Same snippet length for all scores | No relevance-based truncation |
| No diversity | Redundant same-file results | No dedup |

### Previously Tried and Removed

**StructuralOracle + Boost Layer** (see `spec-work-deferred.md:210-258`):
- StructuralOracle (~170 lines) was implemented and removed
- Design bug: returned file-level doc_ids (`./src/main.rs`) when others return symbol-level (`./src/main.rs::fn:main`)
- RRF fusion can't merge different doc_id granularities
- Boost layer workaround also tried: results were 0.1 neutral, 0.5 regression
- **Key lesson:** Structural signals are priors (importance), not relevance signals
- **Rebuild condition:** Query-type routing (Phase 3) + granularity resolution

---

## Design Decisions

### Decision 1: Embedder Lifecycle → Oracle Owns It

**Choice:** SemanticOracle owns embedder instance, created once at startup.

**Rationale:**
- Fixes the REAL bottleneck (model loading, not embedding)
- Aligns with dependable-rust (oracle is black box, owns its resources)
- First query loads model, subsequent queries are fast

```rust
// BEFORE (current - loads model every query)
impl Oracle for SemanticOracle {
    fn query(&self, query: &str, limit: usize) -> Result<Vec<OracleResult>> {
        let results = scry_text(query, &options)?;  // Creates embedder inside
    }
}

// AFTER (model loaded once)
pub struct SemanticOracle {
    embedder: Box<dyn EmbeddingEngine>,  // Owned, created once
    projection: Option<Projection>,       // Cached
    index: Index,                          // Cached
}
```

### Decision 2: Query Embedding Cache → Per-Session LRU

**Choice:** Session-scoped LRU cache, not persistent.

**Rationale:**
- Simple to implement (no persistence layer)
- Session queries ARE clustered (user explores one area)
- Measure hit rate first, upgrade IF data shows need

```rust
// Simple. Works. Ships today.
let cache: LruCache<String, Vec<f32>> = LruCache::new(100);
```

### Decision 3: Intent Classification → Data First

**Choice:** Collect oracle contribution data, analyze patterns, then build classifier IF data supports it.

**Rationale:**
- We have only 6 logged queries - not enough to design heuristics
- Guessing keywords without data violates "measure first" principle
- Logging oracle contributions builds the dataset we need
- Classifier implementation is conditional on observed patterns

**Interface (stable):**
```rust
pub trait IntentClassifier {
    fn classify(&self, query: &str) -> QueryIntent;
}
```

**Implementation:** TBD after analyzing 100+ queries with oracle contribution data.

**Fallback:** If no clear patterns emerge, keep all-oracle default (no wasted effort).

### Decision 4: StructuralOracle Scoring → Reuse Orient Composite

**Choice:** Extract existing composite score from orient mode.

**Rationale:**
- Already tested and working in `handle_orient()` (server.rs:1067)
- No arbitrary weight tuning needed
- ~10 lines to extract and reuse

```rust
// Already exists in server.rs:1067 - just extract
let composite =
    is_entry_point * 20 +
    min(importer_count * 2, 20) +
    activity_score +
    commit_score -
    is_test * 5;
```

### Decision 5: Diversity → Hard Cap (Max 2/File)

**Choice:** Simple file-count limit, not MMR.

**Rationale:**
- MMR adds compute AND complexity
- Hard cap is trivial to implement and explain
- Log same-file usage to validate decision later

```rust
let mut file_count: HashMap<String, usize> = HashMap::new();
results.retain(|r| {
    let count = file_count.entry(file_of(r)).or_insert(0);
    *count += 1;
    *count <= 2
});
```

### Decision 6: Learning System → Usage Boosting Only

**Choice:** Simple frequency boost now, defer L2R to future.

**Rationale:**
- L2R needs labeled training data we don't have
- Usage boosting = SQL query + 10 lines of Rust
- Collect data now, evaluate L2R in 4+ weeks

```rust
// Boost factor 0.1 is initial estimate - verify impact in Phase 4
let boost = 1.0 + config.usage_boost_factor * (1.0 + use_count as f32).ln();
result.fused_score *= boost;
```

### Decision 7: Truncation → Score-Based

**Choice:** High-score results get full content, low-score get path only.

**Thresholds:** The score thresholds below are initial estimates. Verify against actual score distributions in Phase 0 error analysis (Task 0.4). Adjust based on observed data.

```rust
fn format_result_content(result: &FusedResult, config: &TruncationConfig) -> String {
    match result.fused_score {
        s if s > config.full_threshold => truncate_content(&result.content, 300),
        s if s > config.summary_threshold => truncate_content(&result.content, 100),
        _ => String::new(),  // Path only
    }
}

// Initial values - verify in Phase 0
struct TruncationConfig {
    full_threshold: f32,     // Start with 0.08, adjust based on score distribution
    summary_threshold: f32,  // Start with 0.05, adjust based on score distribution
}
```

---

## Implementation Plan

### Phase 0: Baseline (MANDATORY FIRST)

> "Flying blind is the real risk."

**Task 0.1:** Run existing eval to get baseline

```bash
patina eval              # Get precision baseline
patina eval --feedback   # Get real-world precision
```

Record results in Validation section below.

**Task 0.2:** Add query timing to QueryEngine

```rust
// src/retrieval/engine.rs
pub fn query(&self, query: &str, limit: usize) -> Result<Vec<FusedResult>> {
    let start = Instant::now();
    // ... existing logic ...
    eprintln!("patina: query completed in {:?}", start.elapsed());
    Ok(results)
}
```

**Task 0.3:** Add per-oracle contribution logging

Before RRF fusion, log which oracle returned each doc_id and at what rank.
This data enables future intent analysis (Phase 3).

```rust
// src/retrieval/engine.rs, before rrf_fuse()
fn log_oracle_contributions(oracle_results: &[Vec<OracleResult>], query: &str) {
    // Log: query, oracle_name, doc_id, rank for each result
    // Enables analysis: "Which oracles find results that get used?"
}
```

Minimal code (~20 lines). Starts collecting immediately.

**Task 0.4:** Establish error analysis workflow

After baseline, identify:
- 5 slowest queries → investigate why (cache miss? large result set?)
- 5 lowest precision queries → what's missing? wrong oracle?
- 5 highest token responses → truncation working?

Action: Create `patina eval --errors` or document manual analysis steps.
This is not optional - error analysis drives Phase 1-4 priorities.

**Exit criteria:** Baseline latency and accuracy recorded. Oracle contribution logging active. Error analysis workflow documented.

---

### Phase 1: Fix Model Loading (~100 lines, 1 session)

This is the **highest impact change** — fixes the real bottleneck.

**Task 1.1:** SemanticOracle owns embedder

Modify `src/retrieval/oracles/semantic.rs`:
- Add embedder, projection, index as struct fields
- Load once in `new()`, reuse in `query()`
- Handle &mut self requirement (interior mutability or trait change)

**Task 1.2:** PersonaOracle same fix

Apply same pattern to `src/commands/persona/mod.rs`.

**Task 1.3:** QueryEngine error handling

Handle oracle creation failures gracefully (oracle unavailable, not server crash).

**Expected impact:** Significant latency reduction (model already loaded). Measure before/after.

---

### Phase 2: Quick Wins (~80 lines, 1 session)

Each change: implement → measure → compare → ship if better.

**Task 2.1:** Query embedding cache

Create `src/retrieval/cache.rs` (~40 lines):
- LRU cache keyed by query text
- Track hits/misses for monitoring

**Task 2.2:** Relevance-based truncation

Modify `format_results()` in server.rs (~20 lines).

**Task 2.3:** File deduplication

Add after RRF fusion in engine.rs (~20 lines).

**Expected impact:** Reduced tokens, faster cache hits. Measure before/after.

---

### Phase 3: Intent Analysis (~60 lines, 1 session)

**Precondition:** 100+ queries logged with oracle contributions (from Phase 0 instrumentation).

**Why this phase matters:** Intent-based routing could improve latency by skipping irrelevant oracles. But we need data to know IF patterns exist and WHAT they are.

**Task 3.1:** Analyze collected query data

Query the eventlog to answer:
- Which oracles contribute to results that get used (scry.use)?
- Are there query patterns that favor specific oracles?
- What's the distribution of query types?

```bash
# Example analysis queries
patina eval --oracle-contributions  # New flag: analyze oracle hit rates
```

**Task 3.2:** Build classifier IF data supports it

Based on analysis:
- If clear patterns exist → implement routing based on observed patterns
- If no patterns → document finding, keep all-oracle default (no wasted work)

**Task 3.3:** A/B comparison (if routing implemented)

Route subset of queries with new logic, compare precision to baseline.
Only ship if measurable improvement.

**Fallback behavior:** If classification is uncertain or fails, default to all oracles (current behavior). Never reduce oracle coverage without confidence.

**Expected impact:** Data-driven decision on routing. Measure before/after IF routing implemented.

**Note:** StructuralOracle was previously tried and removed (see Gaps section). Don't re-add to RRF without solving granularity mismatch.

---

### Phase 4: Usage Learning (~60 lines, 1 session)

**Task 4.1:** Usage-based score boosting

Create `src/retrieval/learning.rs`:
- Query eventlog for scry.use events
- Log-dampened boost based on usage count

**Expected impact:** Improved accuracy as usage data accumulates. Measure before/after.

---

## What NOT To Do

> "You can always make it smarter later. You can't get back the time spent on premature ML."

| Anti-Pattern | Why Avoid |
|--------------|-----------|
| Build new eval infrastructure | Already exists (`patina eval`, 597 lines) |
| Re-add StructuralOracle to RRF | Already tried, failed (see `spec-work-deferred.md:210-258`) |
| Build L2R ranker now | No training data yet |
| Implement MMR diversity | Complexity without proven need |
| Persist cache across sessions | Session scope sufficient for v1 |
| Build intent classifier without data | Collect 100+ queries with oracle contributions first |
| Skip baseline measurement | Flying blind is the real risk |
| Polish before shipping | Deploy teaches more than spec |

**StructuralOracle Lesson:** Structural signals (importer_count, is_entry_point, etc.) are *priors* not *relevance signals*. They tell you "this file is important in general" not "this file is relevant to your query." Mixing granularities (file-level vs symbol-level) in RRF fusion doesn't work. Revisit only after intent routing provides query-type context.

---

## Rollback Strategy

> Ng principle: "Don't break prod"

Each phase must be reversible. If a change degrades performance, roll back immediately.

| Phase | Risk | Rollback Plan |
|-------|------|---------------|
| 0 | None (instrumentation only) | Remove logging code |
| 1 | Oracle fails to load model at startup | Graceful degradation: oracle reports unavailable, QueryEngine continues with remaining oracles |
| 2 | Cache causes memory issues or stale results | Disable cache via config flag, fall back to no-cache |
| 3 | Intent routing skips needed oracles | Fallback to all-oracle default (already documented) |
| 4 | Usage boosting over-weights old results | Disable boost via config, scores unchanged |

**Implementation:** Each new module should have a feature flag or config option to disable it without code changes.

```rust
// Example: cache.rs
pub fn get_embedding(query: &str, config: &Config) -> Option<Vec<f32>> {
    if !config.cache_enabled { return None; }
    // ... cache logic
}
```

---

## Phase Summary

| Phase | Tasks | Exit Criteria |
|-------|-------|---------------|
| 0 | Run eval, add timing, add oracle logging, establish error analysis | Baseline + instrumentation + error workflow |
| 1 | Fix model loading | SemanticOracle + PersonaOracle own resources |
| 2 | Cache, truncation, dedup | Tokens reduced |
| 3 | Analyze query data, build classifier IF patterns exist | Data-driven routing OR "no patterns" documented |
| 4 | Usage boosting | Learning active |

---

## Validation

### Metrics Dashboard

After each phase, record:

| Metric | Baseline | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|--------|----------|---------|---------|---------|---------|
| MRR | 0.519 | 0.519 | | | |
| Query latency (p50) | 150ms | **22ms** | | | |
| Query latency (p95) | 198ms | 184ms | | | |
| Query latency (mean) | 159ms | **41ms** | | | |
| Subsequent query | ~150ms | **17-26ms** | | | |
| Semantic P@5 | 4.0% | 4.0% | | | |
| Semantic P@10 | 4.0% | 4.0% | | | |
| Temporal file→file P@10 | 29.5% | 29.5% | | | |
| Feedback precision | 0% | 0% | | | |
| Tokens/response | TBD | TBD | | | |
| Cache hit rate | 0% | N/A | | | |

### Success Criteria

| Metric | Target | Rationale |
|--------|--------|-----------|
| Subsequent query latency | Significant reduction vs baseline | Model already loaded |
| Tokens/response | Measurable reduction | Truncation + dedup |
| Cache hit rate | TBD after Phase 0 | Validates cache value |
| P@10 | Improvement vs baseline | Meaningful improvement |

### Error Analysis

After each phase, examine:
1. **Slowest 5 queries** - Why slow? Cache miss? Large result set?
2. **Lowest precision queries** - What's missing? Wrong oracle? Bad ranking?
3. **Highest token queries** - Truncation not working? Too many results?

**Workflow (manual for Phase 0):**

```bash
# Slowest queries - run bench with timing
PATINA_LOG=1 patina bench retrieval --query-set eval/retrieval-queryset.json 2>&1 | grep DEBUG

# Lowest precision - check bench output for R@10=0%
patina bench retrieval --query-set eval/retrieval-queryset.json
# Look for queries with RR=0.00

# Oracle contributions - see which oracles miss
PATINA_LOG=1 patina scry --hybrid "your query" 2>&1 | grep "retrieval::oracle"
```

**Phase 0 findings:**
- q3-scry (RR=0.00): Semantic returns sessions, lexical finds code but ranks it #7
- 99.7% of query time is oracle execution (confirms model-loading bottleneck)

---

## Future Work (Post v1)

Deferred until we have data proving need:

| Item | Trigger to Revisit |
|------|-------------------|
| Persistent cache | Session hit rate <20% |
| ML intent classifier | Heuristic accuracy <80% |
| MMR diversity | Same-file result #3+ used frequently |
| Learning-to-rank | 4+ weeks of usage data collected |
| Query embedding fine-tuning | Semantic MRR plateaus |

---

## Files to Modify/Create

| File | Action | "Do X" | Aligns With |
|------|--------|--------|-------------|
| `src/retrieval/oracles/semantic.rs` | **Modify** | Own embedder, projection, index | dependable-rust |
| `src/commands/persona/mod.rs` | **Modify** | Own embedder, index | dependable-rust |
| `src/retrieval/cache.rs` | **Create** | Cache query embeddings | unix-philosophy |
| `src/retrieval/intent.rs` | **Create** (IF data supports) | Classify query intent | unix-philosophy |
| `src/retrieval/learning.rs` | **Create** | Boost scores by usage | unix-philosophy |
| `src/retrieval/engine.rs` | **Modify** | Coordinate oracle queries, fuse results | unix-philosophy |
| `src/mcp/server.rs` | **Modify** | Format results for MCP response | - |

**Note on engine.rs:** The "Do X" is "coordinate oracle queries and fuse results." Timing and oracle logging are instrumentation (internal). Dedup is post-fusion cleanup (internal). Intent routing delegates to `intent.rs` if it exists. Engine.rs remains a coordinator, not a dumping ground.

---

## Exit Criteria

| Criteria | Status |
|----------|--------|
| Phase 0: Baseline recorded via `patina eval` | [x] |
| Phase 0: Query timing added | [x] |
| Phase 0: Oracle contribution logging active | [x] |
| Phase 0: Error analysis workflow documented | [x] |
| Phase 1: SemanticOracle owns embedder + projection + index | [x] |
| Phase 1: PersonaOracle owns embedder + index | [x] |
| Phase 1: Subsequent query latency significantly reduced | [x] (6.8x faster) |
| Phase 2: Cache module created | [ ] |
| Phase 2: Truncation implemented | [ ] |
| Phase 2: File dedup implemented | [ ] |
| Phase 2: Token reduction measurable | [ ] |
| Phase 3: 100+ queries with oracle contributions logged | [ ] |
| Phase 3: Query pattern analysis complete | [ ] |
| Phase 3: Routing implemented OR documented "no patterns found" | [ ] |
| Phase 4: Usage boosting active | [ ] |
| Overall: MRR improvement vs baseline | [ ] |

---

## References

- `layer/core/unix-philosophy.md` - One tool, one job
- `layer/core/dependable-rust.md` - Black-box modules, stable interfaces
- `layer/core/adapter-pattern.md` - Trait-based oracles
- `src/commands/eval/mod.rs` - Existing eval framework (597 lines)
- `src/commands/scry/internal/search.rs:70` - Model loading bottleneck (semantic)
- `src/commands/persona/mod.rs:256` - Model loading bottleneck (persona)
- `spec-work-deferred.md:210-258` - StructuralOracle removal rationale
