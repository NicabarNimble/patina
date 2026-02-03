---
type: feat
id: d1-belief-oracle
status: design
created: 2026-02-02
updated: 2026-02-03
sessions:
  origin: 20260202-202802
  design-resolution: 20260203-065424
related:
  - layer/surface/build/feat/mother-delivery/SPEC.md
  - layer/surface/build/feat/mother-delivery/design.md
beliefs:
  - beliefs-valuable-for-knowledge-not-task
---

# feat: D1 — BeliefOracle (Beliefs as Default Search Channel)

> Make beliefs a first-class search channel so they surface naturally in every query, not just as ignorable annotations.

## Problem

Beliefs appear only via `mode=belief` (explicit) or as post-processing annotations on code results (ignorable). The A/B eval (session 20260202-151214) showed task-oriented queries get **-0.05 delta** — beliefs actively hurt because they're wired as annotations, not as a competing search channel.

**Root cause:** Beliefs are drowned by code in the shared USearch index. A separate oracle gives beliefs their own ranking list in RRF, guaranteeing representation.

---

## Design

A `BeliefOracle` runs on **every default query** as a parallel search channel alongside SemanticOracle, LexicalOracle, TemporalOracle, and PersonaOracle.

```
scry("how should I handle errors?")
  → SemanticOracle:  code results from function_facts
  → LexicalOracle:   FTS5 matches from code + commits
  → TemporalOracle:  co-change clusters
  → PersonaOracle:   cross-project user knowledge
  → BeliefOracle:    hybrid vector + FTS5 against beliefs  ← NEW
  → RRF merge all channels
  → Return with channel tags: [code] [commit] [belief]
```

**"Do X" test:** "Find beliefs relevant to this query." Clear, focused, one job.

### BeliefOracle Design (A+B: Vector + FTS5)

The oracle implements the `Oracle` trait and internally runs two search channels:

```rust
// src/retrieval/oracles/belief.rs

pub struct BeliefOracle;

impl Oracle for BeliefOracle {
    fn name(&self) -> &'static str { "belief" }
    fn query(&self, query: &str, limit: usize) -> Result<Vec<OracleResult>>
    fn is_available(&self) -> bool  // checks beliefs table + index exist
}
```

**Channel A — Vector (semantic similarity):**
1. Embed query using E5-base-v2 pipeline (same as SemanticOracle)
2. Project to 256-dim (same projection)
3. Search USearch index with aggressive over-fetch, filter to `BELIEF_ID_OFFSET` range (4B-5B)
4. Enrich from beliefs table: statement, entrenchment, evidence metrics

Reuses the existing semantic index — beliefs are already embedded there. Filtering happens post-search (USearch doesn't support range filters natively).

**Over-fetch strategy:** With ~48 beliefs in an index of thousands of code entries, a naive `limit * 2` search may return zero beliefs in the top results. The oracle must over-fetch aggressively — `min(limit * 50, total_index_size / 2)` — then filter to the 4B-5B range. This is acceptable because USearch ANN search is sub-millisecond even at high limits, and the post-filter reduces to a small result set. If over-fetching proves insufficient, a dedicated belief USearch index (tiny, fast to build during scrape) is the fallback.

**Channel B — FTS5 (keyword):**
1. Tokenize query for FTS5
2. Search existing `belief_fts` table (already created during scrape)
3. Return: belief_id, statement, BM25 score

Existing table schema (created during `patina scrape`, `scrape/beliefs/mod.rs:103`):
```sql
CREATE VIRTUAL TABLE IF NOT EXISTS belief_fts USING fts5(
    id,
    statement,
    facets,
    content,
    tokenize='porter unicode61'
);
```

This table already exists and is populated during scrape but **not queried by any oracle today**. The BeliefOracle becomes its first consumer. The schema is richer than originally proposed — `content` contains the full belief text, `facets` contains domain tags. No new table needed.

Captures keyword matches vector search misses. A query containing "thiserror" matches a belief about "use thiserror derive macros" via exact keyword, not just semantic proximity.

**Internal merge (weighted sum, one ranked list):**
```rust
const VECTOR_WEIGHT: f32 = 0.7;
const TEXT_WEIGHT: f32 = 0.3;

// Dedup by belief_id
// score = VECTOR_WEIGHT * cosine + TEXT_WEIGHT * bm25_normalized
// Sort descending, return as Vec<OracleResult>
// score_type = "hybrid_belief"
```

The oracle produces ONE ranked list. RRF treats it as one channel competing alongside code/commits/temporal. The internal A+B is an implementation detail hidden behind the Oracle trait (dependable-rust: black-box module).

### OracleResult Shape

```rust
OracleResult {
    doc_id: "belief:error-handling-thiserror",
    content: "Use thiserror derive macros for error types \
              (entrenchment: 0.8, evidence: 3/2, use: 5+2)",
    source: "belief",
    score: 0.83,
    score_type: "hybrid_belief",
    metadata: OracleMetadata {
        file_path: Some("layer/surface/beliefs/error-handling-thiserror.md"),
        ..
    },
}
```

### Wiring into QueryEngine

```rust
// engine.rs
pub fn default_oracles() -> Vec<Box<dyn Oracle>> {
    vec![
        Box::new(SemanticOracle::new()),
        Box::new(LexicalOracle::new()),
        Box::new(TemporalOracle::new()),
        Box::new(PersonaOracle::new()),
        Box::new(BeliefOracle::new()),  // NEW
    ]
}
```

### Intent Detection Update

```rust
// intent.rs — add belief weight
pub struct IntentWeights {
    semantic: f32,   // default 1.0
    lexical: f32,    // default 1.0
    temporal: f32,   // default 1.0
    persona: f32,    // default 1.0
    belief: f32,     // default 1.0  ← NEW
}

// Boosts:
// Rationale ("why", "decided"): belief 1.5, persona 1.5
// Definition ("what is", "explain"): belief 1.5, lexical 1.5
// Temporal ("when", "history"): no belief boost
// General: belief 1.0 (always participates)
```

### Scrape Integration

No scrape changes needed — all infrastructure already exists:
1. Beliefs are already inserted into the `beliefs` table
2. Beliefs are already embedded in USearch index at `BELIEF_ID_OFFSET + rowid`
3. `belief_fts` is already created and populated with `(id, statement, facets, content)`

### Module Layout

```
src/retrieval/oracles/
├── mod.rs          # pub use exports
├── semantic.rs     # existing
├── lexical.rs      # existing
├── temporal.rs     # existing
├── persona.rs      # existing
└── belief.rs       # NEW — BeliefOracle (A+B)
```

### Cross-Project (Federation)

During graph routing, the BeliefOracle runs against each related project's belief table. A query in `cairo-game` that routes to `patina` via LEARNS_FROM also searches patina's beliefs. Reuses the existing `query_repo()` / `query_all_repos()` pattern in `engine.rs` — the QueryEngine already creates fresh oracle instances per repo context, collects results, and performs cross-repo RRF fusion. The BeliefOracle participates in this automatically once wired into `default_oracles()`.

---

## Exit Criteria

- [ ] BeliefOracle wired into default query flow — beliefs appear in standard scry results without `mode=belief` or `--belief` flags
- [ ] BeliefOracle Channel B queries existing `belief_fts` table (no new table needed)
- [ ] Intent boost — Rationale and Definition intents boost belief oracle weight
- [ ] Over-fetch validated — confirm beliefs surface reliably with ~48 beliefs in index of thousands (fallback: dedicated belief USearch index)
- [ ] **Measured:** Re-run task-oriented A/B eval. Target: delta >= 0.0 (beliefs no longer hurt). Stretch: delta >= +0.5

---

## See Also

- [[design.md]] — ADR-1 (Why A+B, not Option C)
- [[../SPEC.md]] — Parent spec (implementation order, non-goals)
