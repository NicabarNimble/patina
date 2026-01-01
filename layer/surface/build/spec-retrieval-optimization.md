# Spec: Retrieval Optimization

**Purpose:** Improve knowledge retrieval for LLM frontends: faster response, fewer tokens, more accurate results.

**Origin:** Session 20260101-070900. Deep code analysis of QueryEngine, oracles, MCP interface.

**Status:** Ready for Implementation

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
| **Latency** | Unknown (not instrumented) | <100ms p50 | Add timing to QueryEngine |
| **Tokens** | ~2000/query | ~1400 | Character count in format_results |
| **Accuracy** | Unknown baseline | +15% MRR | Run existing `patina eval` |

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

Loading ONNX model from disk: **200-500ms per query**
Actual embedding inference: **~50-100ms**

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

### Decision 3: Intent Classification → Heuristics First

**Choice:** Rule-based classification with comprehensive logging.

**Rationale:**
- Heuristics ship in 30 minutes
- Classifier needs training data we don't have
- Logging builds the dataset for future classifier IF needed

```rust
fn classify_intent(query: &str) -> QueryIntent {
    let lower = query.to_lowercase();
    if lower.contains("where") || lower.contains("find") { Location }
    else if lower.contains("recent") || lower.contains("changed") { Temporal }
    else { Conceptual }
}
```

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
let boost = 1.0 + 0.1 * (1.0 + use_count as f32).ln();
result.fused_score *= boost;
```

### Decision 7: Truncation → Score-Based

**Choice:** High-score results get full content, low-score get path only.

```rust
fn format_result_content(result: &FusedResult) -> String {
    match result.fused_score {
        s if s > 0.08 => truncate_content(&result.content, 300),
        s if s > 0.05 => truncate_content(&result.content, 100),
        _ => String::new(),  // Path only
    }
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

**Exit criteria:** Baseline latency and accuracy recorded.

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

**Expected impact:** Query latency drops from ~500ms+ to ~100ms (model already loaded).

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

**Expected impact:** -30% tokens, -30ms on cache hits.

---

### Phase 3: Intent Routing (~100 lines, 1-2 sessions)

**Why this phase matters:** Intent routing is the precondition for future structural signal integration (see spec-work-deferred.md). It also improves latency by skipping irrelevant oracles.

**Task 3.1:** Intent classification with logging

Create `src/retrieval/intent.rs`:
- Heuristic classifier (temporal, location, conceptual)
- Log classifications to eventlog for future analysis
- Track which oracles contributed to successful results

**Task 3.2:** Wire intent → oracle selection

Modify QueryEngine to filter oracles based on intent:
- `Temporal` → temporal oracle only (skip semantic/lexical)
- `Location` → lexical + semantic (skip temporal/persona)
- `Conceptual` → all oracles (default)

**Task 3.3:** Collect usage data for future structural reintegration

Log when structural priors would help (orientation queries, ambiguous results). This data informs whether/how to reintroduce structural signals.

**Expected impact:** -30ms latency (skip irrelevant oracles), data for future improvements.

**Note:** StructuralOracle was previously tried and removed (see Gaps section). Don't re-add to RRF without solving granularity mismatch.

---

### Phase 4: Usage Learning (~60 lines, 1 session)

**Task 4.1:** Usage-based score boosting

Create `src/retrieval/learning.rs`:
- Query eventlog for scry.use events
- Log-dampened boost based on usage count

**Expected impact:** +5-10% accuracy as usage data accumulates.

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
| Train intent classifier | Heuristics + logging first |
| Skip baseline measurement | Flying blind is the real risk |
| Polish before shipping | Deploy teaches more than spec |

**StructuralOracle Lesson:** Structural signals (importer_count, is_entry_point, etc.) are *priors* not *relevance signals*. They tell you "this file is important in general" not "this file is relevant to your query." Mixing granularities (file-level vs symbol-level) in RRF fusion doesn't work. Revisit only after intent routing provides query-type context.

---

## Timeline

| Day | Tasks | Exit Criteria |
|-----|-------|---------------|
| 1 | Phase 0: Run eval, add timing | Baseline recorded |
| 1-2 | Phase 1: Fix model loading | SemanticOracle + PersonaOracle own resources |
| 2 | Phase 2: Cache, truncation, dedup | Tokens reduced |
| 3-4 | Phase 3: Intent routing | Selective oracle calls, logging |
| 5 | Phase 4: Usage boosting | Learning active |

**Total: 5 focused sessions, not weeks.**

---

## Validation

### Metrics Dashboard

After each phase, record:

| Metric | Baseline | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|--------|----------|---------|---------|---------|---------|
| Query latency (first) | TBD | | | | |
| Query latency (subsequent) | TBD | | | | |
| Semantic P@5 | TBD | | | | |
| Semantic P@10 | TBD | | | | |
| Temporal file→file P@10 | TBD | | | | |
| Feedback precision | TBD | | | | |
| Tokens/response | TBD | | | | |
| Cache hit rate | 0% | | | | |

### Success Criteria

| Metric | Target | Rationale |
|--------|--------|-----------|
| Subsequent query latency | <100ms | Model already loaded |
| Tokens/response | -30% | Truncation + dedup |
| Cache hit rate | >30% | Validates cache value |
| P@10 | +10% | Meaningful improvement |

### Error Analysis

After each phase, examine:
1. **Slowest 5 queries** - Why slow? Cache miss? Large result set?
2. **Lowest precision queries** - What's missing? Wrong oracle? Bad ranking?
3. **Highest token queries** - Truncation not working? Too many results?

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

| File | Action | Aligns With |
|------|--------|-------------|
| `src/retrieval/oracles/semantic.rs` | **Modify** - own embedder, projection, index | dependable-rust |
| `src/commands/persona/mod.rs` | **Modify** - own embedder, index | dependable-rust |
| `src/retrieval/cache.rs` | **Create** - embedding cache | unix-philosophy |
| `src/retrieval/intent.rs` | **Create** - classification + logging | unix-philosophy |
| `src/retrieval/learning.rs` | **Create** - usage boost | unix-philosophy |
| `src/retrieval/engine.rs` | **Modify** - timing, dedup, intent routing | - |
| `src/mcp/server.rs` | **Modify** - truncation | - |

---

## Exit Criteria

| Criteria | Status |
|----------|--------|
| Phase 0: Baseline recorded via `patina eval` | [ ] |
| Phase 0: Query timing added | [ ] |
| Phase 1: SemanticOracle owns embedder + projection + index | [ ] |
| Phase 1: PersonaOracle owns embedder + index | [ ] |
| Phase 1: Subsequent query latency <100ms | [ ] |
| Phase 2: Cache module created | [ ] |
| Phase 2: Truncation implemented | [ ] |
| Phase 2: File dedup implemented | [ ] |
| Phase 2: Token reduction >25% | [ ] |
| Phase 3: Intent classification with logging | [ ] |
| Phase 3: Intent-based oracle routing | [ ] |
| Phase 4: Usage boosting active | [ ] |
| Overall: MRR improvement >0.08 | [ ] |

---

## References

- `layer/core/unix-philosophy.md` - One tool, one job
- `layer/core/dependable-rust.md` - Black-box modules, stable interfaces
- `layer/core/adapter-pattern.md` - Trait-based oracles
- `src/commands/eval/mod.rs` - Existing eval framework (597 lines)
- `src/commands/scry/internal/search.rs:70` - Model loading bottleneck (semantic)
- `src/commands/persona/mod.rs:256` - Model loading bottleneck (persona)
- `spec-work-deferred.md:210-258` - StructuralOracle removal rationale
