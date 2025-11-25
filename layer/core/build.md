# Build Recipe

Persistent roadmap across sessions. **Start here when picking up work.**

---

## Current Direction (2025-11-25)

**Principle:** "Don't optimize what you can't measure."

~~Patina has strong architectural foundations but is stuck at "semantic only"~~

**Progress:** Temporal dimension and Scry MVP complete. Key insight discovered:
- **Semantic** (text→text): Working well, query interface matches training signal
- **Temporal** (text→files): Query interface mismatch - trained on file co-change relationships but queried with arbitrary text. Needs file-to-file query interface.

**Immediate Path:**
1. ~~Build temporal dimension~~ ✅ Done
2. ~~Build Scry MVP~~ ✅ Done
3. Build evaluation framework (discover which query interfaces work for which dimensions)
4. THEN decide on remaining dimensions and query interface designs

**Explicitly Deferred:**
- MLX runtime (nice-to-have, not blocking)
- Qwen3/model upgrades (invalidates all projections, premature optimization)
- Dependency/syntactic/architectural/social dimensions (until eval clarifies query patterns)
- Mothership service (needs Scry working first)

---

## Active Work

### Phase 2.5: Validate Multi-Dimension RAG

**Goal:** Prove the architecture works end-to-end before investing in all 6 dimensions.

#### 2.5a: Temporal Dimension ✅
**Status:** Complete (2025-11-25)
**Effort:** 1-2 days
**Why first:** `co_changes` table already materialized, simplest pairing logic

- [x] Create `src/commands/oxidize/temporal.rs`
- [x] Pairing logic: files changed in same commit = related
- [x] Training signal: 590 files with 17,685 co-change relationships
- [x] Output: `temporal.safetensors` (4.2MB) + `temporal.usearch` (2.1MB, 1807 vectors)

**⚠️ Insight:** Pipeline works but query interface mismatch discovered:
- Training: file paths ↔ file paths (co-change relationships)
- Current query: arbitrary text → file paths (doesn't make sense)
- Needed: file-to-file queries (`--file src/foo.rs` → related files)

#### 2.5b: Scry MVP ✅
**Status:** Complete (2025-11-25)
**Effort:** 3-5 days
**Why:** Can't validate retrieval quality without query interface

- [x] `patina scry "query"` - basic vector search
- [x] Load semantic.usearch and temporal.usearch via `--dimension` flag
- [x] SQLite metadata enrichment (event_type, source_id, content)
- [x] Result formatting with scores
- [x] Options: `--limit`, `--min-score`, `--dimension`

#### 2.5c: Evaluation Framework ✅
**Status:** Complete (2025-11-25)
**Effort:** 2-3 days
**Why:** Without metrics, dimension value is speculation

**Expanded scope:** Eval isn't just "does retrieval work?" but "which query interfaces make sense for which dimensions?"

- [x] Semantic eval: text→text queries against session observations
- [x] Temporal eval: file→file queries against co-change relationships
- [x] Baseline comparison: vector retrieval vs random
- [x] Query interface discovery: what input types work for each dimension?

**Actual Results (2025-11-25):**
| Dimension | Query Type | P@10 | vs Random | Verdict |
|-----------|------------|------|-----------|---------|
| Semantic | text → text | 7.8% | **8.6x** | ✅ Works |
| Temporal | text → files | N/A | N/A | ❌ No ground truth |
| Temporal | file → files | 24.4% | **3.2x** | ✅ Works |

**Conclusions:**
1. Semantic retrieval validated - significantly better than random
2. Temporal file→file validated - co-change relationships are learned
3. Temporal needs file-based query interface (not text queries)

---

## Completed Phases

### Phase 1: Scrape Pipeline ✅ (2025-11-22)
**Specs:** [spec-eventlog-architecture.md](../surface/build/spec-eventlog-architecture.md), [spec-scrape-pipeline.md](../surface/build/spec-scrape-pipeline.md)

Unified eventlog with 16,027 events across 17 types:
- Git: 707 commits → commits, commit_files, co_changes views
- Sessions: 2,174 events → sessions, observations, goals views
- Code: 13,146 events → functions, call_graph, symbols views

### Phase 2: Oxidize (Semantic Only) ✅ (2025-11-24)
**Spec:** [spec-oxidize.md](../surface/build/spec-oxidize.md)

Working pipeline for single dimension:
- Recipe format: `oxidize.yaml`
- E5-base-v2 embeddings (768-dim)
- 2-layer MLP projection (768→1024→256)
- Safetensors export (v0.7, MLX-compatible)
- USearch HNSW index (1,807 vectors)

**Output:**
- `.patina/data/embeddings/e5-base-v2/projections/semantic.safetensors` (4.2MB)
- `.patina/data/embeddings/e5-base-v2/projections/semantic.usearch` (2.1MB)

---

## Future Phases (Blocked on 2.5)

### Phase 3: Additional Dimensions
**Blocked until:** Scry + eval prove 2-dimension retrieval valuable

| Dimension | Training Signal | Data Available | Status |
|-----------|-----------------|----------------|--------|
| Semantic | Same session = related | 2,174 session events | ✅ Done |
| Temporal | Same commit = related | 590 files, 17,685 co-changes | ✅ Done |
| Dependency | Caller/callee = related | 9,634 code.call events | After eval |
| Syntactic | Similar AST = related | 790 code.function events | After eval |
| Architectural | Same module = related | 13,146 code.* events | After eval |
| Social | Same author = related | 707 commits | Likely skip (single-user noise) |

### Phase 4: Mothership Service
**Spec:** [spec-mothership-service.md](../surface/build/spec-mothership-service.md)
**Blocked until:** Scry MVP working

### Phase 5: Persona
**Spec:** [spec-persona-capture.md](../surface/build/spec-persona-capture.md)
**Blocked until:** Mothership working

### Phase 6: Model Upgrades (MLX/Qwen3)
**Spec:** [spec-model-runtime.md](../surface/build/spec-model-runtime.md)
**Blocked until:** Evaluation proves current architecture valuable

**Why deferred:**
- E5-base-v2 validated on real data (+68% vs baseline)
- Model swap invalidates ALL trained projections
- "Don't optimize what you can't measure"

---

## Architecture Summary

```
Event Sources          →  scrape  →  Unified DB    →  oxidize  →  Vectors    →  scry
.git/ (commits)                      patina.db                    *.usearch       ↓
layer/sessions/*.md                  ├── eventlog                               Results
src/**/* (AST)                       └── views
```

**What's Git-Tracked:**
- `layer/sessions/*.md` - session events (decisions, observations)
- `.patina/oxidize.yaml` - recipe for building projections

**What's Local (rebuilt via scrape/oxidize):**
- `.patina/data/patina.db` - unified eventlog
- `.patina/data/embeddings/` - projection weights + indices

**6-Dimension Model:**
```
Query → E5-base-v2 (768-dim) → [Semantic MLP] → 256-dim ─┐
                              → [Temporal MLP] → 256-dim ─┼→ Concatenated → USearch
                              → [Dependency MLP] → 256-dim─┘   (768-dim with 3)
```

---

## Key Sessions (Context Recovery)

When context is lost, read these sessions for architectural decisions:

| Session | Topic | Key Insight |
|---------|-------|-------------|
| 20251125-095019 | Build Continue | Temporal + Scry + Eval complete. Query interface per dimension. |
| 20251125-065729 | RAG design review | "Don't optimize what you can't measure" |
| 20251124-220659 | Direction deep dive | Path C: 2-3 dims → Scry → validate |
| 20251120-110914 | Progressive adapters | Adapter pattern at every layer |
| 20251116-194408 | E5 benchmark | +68% vs baseline, domain > benchmarks |
| 20251123-222456 | MLX research | Hybrid runtime strategy (future) |

---

## Validation Criteria

**Phase 2.5 is complete when:**
1. ✅ Semantic dimension trained and indexed
2. ✅ Temporal dimension trained and indexed (2025-11-25)
3. ✅ `patina scry "query"` returns ranked results (2025-11-25)
4. ⚠️ 2-dim vs 1-dim: Not directly comparable (different query interfaces needed)
5. ✅ Vector retrieval > random baseline (semantic 8.6x, temporal 3.2x)

**Phase 2.5 Complete!** Proceed to Phase 3 with insight: each dimension may need its own query interface.
