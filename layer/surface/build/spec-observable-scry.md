# Spec: Observable Scry

**Status:** Phase 3 Complete (Dec 2025). Gaps documented in spec-work-deferred.md.
**Parent:** [build.md](../../core/build.md)
**Depends on:** Phase 1.5 signals complete
**Creates:** Feedback loop infrastructure (this spec enables it, not the other way around)

## Problem

scry returns ranked results but doesn't explain its reasoning. Users can't see:
- Which oracle contributed each result (semantic vs lexical vs temporal)
- Why a result ranked where it did
- What signals exist for a result (even if not used for ranking)

Without this visibility:
- Error analysis is impossible (can't see where failures come from)
- Users can't steer toward what they need
- No feedback loop to learn from real usage

## Goal

Make scry **observable** (see what happened), **steerable** (choose the right mode), and **instrumented** (learn from usage).

## Design Principles

### Anchored in layer/core

**unix-philosophy:** Each mode does one job well.
- `find` = relevance ranking
- `orient` = importance ranking
- `recent` = temporal ranking
- `why` = explanation

No mode tries to do everything. Complex behavior emerges from composition.

**dependable-rust:** Clean interface, implementation hidden.
- Stable JSON schema for MCP (agents reason over structure, not text)
- CLI pretty-prints, but interface is the schema
- Per-oracle contributions surfaced without exposing fusion internals

**adapter-pattern:** Structured response works across LLM frontends.
- Same JSON schema whether Claude, Gemini, or future LLMs consume it
- Agents can parse and reason; humans can read CLI output
- Modes are handles any frontend can invoke

### ML Systems Thinking

**1. Data quality > model complexity**

We spent a session optimizing structural boost. It didn't help. Why? Because we optimized without understanding failure modes. Observable output enables error analysis → targeted fixes.

**2. Simple steering > smart routing**

Don't build a query classifier. Give users explicit modes. Learn which modes people use. That's training data for a real router later.

**3. Explain, don't compete**

Structural signals failed as a relevance oracle. But they're valuable as **explanatory metadata** on semantic results. Show them alongside, not instead of.

## Design

### 1. Structured Response (Two-Tiered)

**Principle:** Concise by default, expandable on demand. Verbose output = nobody uses it.

**Key insight: Store rich, display simple.**

Oracle scores are on different scales (cosine similarity 0-1, BM25 unbounded). RRF fusion uses **rank position**, not raw scores, precisely because ranks are comparable across oracles. Our display should reflect this reality.

| Layer | What | Why |
|-------|------|-----|
| **Eventlog** | Raw scores + ranks | Phase 3 analysis needs full data |
| **Default CLI** | Ranks only | Human readability, comparable |
| **--explain CLI** | Ranks + raw scores | Debugging, expert mode |
| **MCP JSON** | Both (structured) | Agents can use either |

This separation enables Phase 3 error analysis ("queries where semantic score > 0.8 but user didn't use result") while keeping CLI output readable.

**Oracle score types:**

| Oracle | Score Type | Range | Notes |
|--------|-----------|-------|-------|
| Semantic | cosine similarity | 0.0 - 1.0 | From E5 embeddings |
| Lexical | BM25 | unbounded positive | From FTS5, higher = more matches |
| Temporal | co-change count | integer | Number of times files changed together |
| Persona | cosine similarity | 0.0 - 1.0 | Cross-project knowledge |

Temporal is notably different - it's a count, not a similarity. Display as "co-changes: 12" rather than a decimal.

**Contributions: only show oracles that matched**

If a doc appears in semantic results but not lexical, omit lexical from contributions (don't show nulls). This keeps output clean and makes it obvious which oracles contributed.

**Current output:**
```
1. ./src/retrieval/engine.rs::rrf_fuse (0.82)
2. ./src/retrieval/fusion.rs::fuse (0.79)
```

**Default output** (ranks - comparable across oracles):
```
1. engine.rs::rrf_fuse     (sem #1 | lex #2 | temp #5 | imp 12)
2. fusion.rs::fuse         (sem #3 | lex #1 | imp 4)
3. persona:direct:2025-12  (sem #8 | persona #1)
```

Format: `(sem #<rank> | lex #<rank> | temp #<rank> | persona #<rank> | imp <importers>)`

Only oracles that contributed are shown. Result #2 has no temporal match. Result #3 is from persona (cross-project knowledge).

**With `--explain`** (full breakdown with raw scores):
```
1. ./src/retrieval/engine.rs::rrf_fuse
   Semantic: #1 (0.91 cosine)
   Lexical:  #2 (12.4 BM25) matched: "RRF", "fuse"
   Temporal: #5 (co-changes: 8)
   Structural: 12 importers, high centrality, not test

2. ./src/retrieval/fusion.rs::fuse
   Semantic: #3 (0.85 cosine)
   Lexical:  #1 (15.2 BM25) matched: "fuse"
   Structural: 4 importers, medium centrality

3. persona:direct:2025-12-08T17:01:25+00:00
   Semantic: #8 (0.72 cosine)
   Persona:  #1 (0.89 cosine) "I prefer ? operator over unwrap()"
```

**MCP: Stable JSON Schema**

For programmatic access, MCP returns structured data with both ranks and raw scores. Only oracles that contributed are included in `contributions`.

```json
{
  "query": "RRF fusion",
  "mode": "find",
  "query_id": "q_20251221_083000_abc",
  "results": [
    {
      "rank": 1,
      "doc_id": "./src/retrieval/engine.rs::rrf_fuse",
      "fused_score": 0.82,
      "contributions": {
        "semantic": { "rank": 1, "raw_score": 0.91, "score_type": "cosine" },
        "lexical": { "rank": 2, "raw_score": 12.4, "score_type": "bm25", "matches": ["RRF", "fuse"] },
        "temporal": { "rank": 5, "raw_score": 8, "score_type": "co_change_count" }
      },
      "annotations": {
        "structural": {
          "importers": 12,
          "centrality": "high",
          "flags": ["entry_point"]
        }
      }
    },
    {
      "rank": 2,
      "doc_id": "./src/retrieval/fusion.rs::fuse",
      "fused_score": 0.79,
      "contributions": {
        "semantic": { "rank": 3, "raw_score": 0.85, "score_type": "cosine" },
        "lexical": { "rank": 1, "raw_score": 15.2, "score_type": "bm25", "matches": ["fuse"] }
      },
      "annotations": {
        "structural": {
          "importers": 4,
          "centrality": "medium",
          "flags": []
        }
      }
    },
    {
      "rank": 3,
      "doc_id": "persona:direct:2025-12-08T17:01:25+00:00",
      "fused_score": 0.45,
      "contributions": {
        "semantic": { "rank": 8, "raw_score": 0.72, "score_type": "cosine" },
        "persona": { "rank": 1, "raw_score": 0.89, "score_type": "cosine", "snippet": "I prefer ? operator..." }
      },
      "annotations": {}
    }
  ]
}
```

This schema enables:
- Agents to reason over results (use ranks for comparison, raw scores for thresholds)
- Phase 3 analysis to query patterns in raw scores
- Lab integration to diagnose "why did this query fail?"
- Missing oracles indicate no match (result #2 has no temporal, result #3 has no lexical/temporal)

Structure is metadata, not ranking signal.

**Implementation note (Phase 1):** `centrality` display is deferred. The raw `centrality_score` from call_graph is not normalized (project-specific scale: sparse CLI might have 0-2, dense monolith 0-50). Will add when `assay derive` computes percentile rank like `file_size_rank` does. Current annotations show: `importers`, `activity_level`, `is_entry_point`, `is_test_file`.

### 2. Explicit Modes

| Mode | Intent | Primary Signal | Use Case |
|------|--------|----------------|----------|
| `scry find <query>` | "Where is X?" | Semantic + lexical (RRF) | Targeted implementation queries |
| `scry orient <path>` | "What's important here?" | Structural composite | Codebase orientation |
| `scry recent <query>` | "What changed related to X?" | Temporal first | Recent activity |
| `scry why <doc_id>` | "Why this result?" | Explain provenance | Debugging/trust |

**Mode Semantics (explicit):**

- **`scry find`**: Semantic + lexical fused via RRF. Structural shown as annotations only, does not affect ranking. Returns symbol-level results.

- **`scry orient`**: File-level outputs only (by design). Ranked by structural composite score. Does not pretend to return symbols. Answers "what matters here?" not "where is X?"

- **`scry recent`**: Searches by semantic/lexical first, then re-ranks by recency. Not purely temporal (that would just return newest files regardless of relevance).

- **`scry why`**: Given a doc_id, explains all signals: semantic similarity, lexical matches, temporal context, structural annotations. Shows how it would rank in each mode.

Implementation: CLI subcommands or flags. MCP tool variants.

This is manual routing. Users choose intent explicitly. We learn which modes are used → informs future automatic routing.

### 3. Feedback Logging

**Principle:** Automatic capture > manual feedback. Usage events happen 10× more often than explicit ratings.

Current eventlog captures:
```sql
event_type: 'scry.query'
payload: {"query": "...", "results": [...], "mode": "find"}
```

**Add automatic usage capture:**

```sql
event_type: 'scry.use'
payload: {"query_id": "...", "result_used": "doc_id", "rank": 3}
```

Capture methods (automatic, no user effort):
- **`scry open <query_id> <rank>`** — opens file/location, logs usage automatically
- **MCP callback** — agent reports "used result X" when inserting context or opening file
- **Clipboard action** — `scry copy <query_id> <rank>` copies and logs

**Add optional explicit feedback:**

```sql
event_type: 'scry.feedback'
payload: {"query_id": "...", "signal": "good|bad", "comment": "..."}
```

Manual feedback is useful but rare. Design for automatic capture first.

**Replay buffer:**
- Query → results → what was used → (optional) good/bad
- After N queries, error analysis becomes data-driven
- Identifies actual bottlenecks vs imagined ones

## Tasks

### Phase 1: Structured Response (Complete - Dec 2025)

| Task | Status |
|------|--------|
| Refactor `QueryEngine::query` to return per-oracle results alongside fused | Done |
| Add `--explain` flag to scry CLI that shows oracle contributions | Done |
| Update MCP scry tool to include oracle breakdown in response | Done |
| Surface structural signals as annotations (from module_signals table) | Done |

**Commits:** `3b6b1eb7`, `d3c8ddca`, `4922fd4c`, `40e2b826`

### Phase 2: Explicit Modes (Complete - Dec 2025)

| Task | Status |
|------|--------|
| Add `scry orient <path>` subcommand (structural-first ranking) | Done |
| Add `scry recent <query>` subcommand (temporal-first) | Done |
| Add `scry why <doc_id>` subcommand (explain single result) | Done |
| Update MCP with mode variants or mode parameter | Done |

**Commits:** `fc6ac8de`, `46c9c87d`, `ada02b8b`, `e585cbe1`

**Usage:**
```bash
# CLI
patina scry orient src/retrieval/          # Structural importance ranking
patina scry recent --days 7                # Recent changes
patina scry recent retrieval --days 3      # Recent changes matching pattern
patina scry why "src/engine.rs" "query"    # Explain specific result

# MCP
scry({ mode: "orient", path: "src/..." })
scry({ mode: "recent", days: 7 })
scry({ mode: "why", doc_id: "...", query: "..." })
```

### Phase 3: Feedback Logging (Complete - Dec 2025)

| Task | Status |
|------|--------|
| Add `scry.use` event type to eventlog | Done |
| Add `scry open <query_id> <rank>` — opens file, logs usage automatically | Done |
| Add `scry copy <query_id> <rank>` — copies to clipboard, logs usage | Done |
| Add MCP callback for "result used" (agent reports which result it consumed) | Done |
| Add `scry feedback <query_id> good\|bad` for optional explicit rating | Done |
| Create SQL views for query analysis (usage, ratings, combined) | Done |

**Commits:** `58dfa873`, `9452dd73`, `8e1f769e`, `cec8b50a`

**Gaps identified:** LLM workflow doesn't naturally use these commands. See spec-work-deferred.md.

## Validation

| Criteria | How to Test |
|----------|-------------|
| `scry find` shows which oracle contributed each result | Run query, verify output shows semantic/lexical/temporal breakdown |
| `scry orient` returns structural-ranked files for a path | Run on `src/retrieval/`, verify high-centrality files rank first |
| `scry why` explains a specific result's scores | Query a doc_id, verify all signal sources shown |
| Queries logged with structure for analysis | Run queries, check eventlog has mode + results + structure |
| Usage capturable | Use a result, verify `scry.use` event logged |

## The Sharp Tests

Two tests to validate the system handles both query types correctly.

### Test 1: Orientation Query

```
scry find "What should I know about the retrieval module?"
```

Response should include:
- **What it is** (semantic matches to retrieval code)
- **Whether it's used** (structural: importer count)
- **Whether it's alive** (temporal: recent commits)
- **Where it connects** (structural: centrality, co-change)
- **Why it believes that** (oracle scores, signal sources)

### Test 2: Targeted Query

```
scry find "Where is RRF fusion implemented?"
```

Pass criteria:
- **Top results are implementation locations** (semantic/lexical win)
- **Structural signals shown but do not distort ranking** (high-activity files don't jump ahead of actual RRF code)
- **`--explain` makes it obvious why the top hit won** (semantic score dominates)

This test ensures we don't regress into priors. The correct answer might be a small, low-activity file that exactly matches the query.

If either test fails, the spec isn't done.

## Integration with Lab

Observable scry and lab (`patina bench`) complement each other:

| Tool | Role | Question Answered |
|------|------|-------------------|
| **Lab (bench)** | Aggregate metrics | "How good is retrieval overall?" (MRR, Recall) |
| **Observable scry** | Per-query explainability | "Why did this specific result rank here?" |

**Workflow:**
```
Lab says:     "MRR dropped from 0.62 to 0.55"
              ↓
You ask:      "Which queries failed? Why?"
              ↓
Observable:   "Query X: semantic #1, lexical not found, temporal #12"
              ↓
Insight:      "Lexical oracle missing keyword coverage for this query type"
              ↓
Fix:          Improve lexical indexing, re-run lab to verify
```

Lab's `--verbose` mode shows what was retrieved. Observable scry shows **why** — which oracle contributed what. Together they form the feedback loop for improvement.

Phase 3 ties them together: eventlog stores rich query data (raw scores + ranks + usage), enabling SQL analysis like "queries where semantic rank was #1 but user selected rank #3."

## Why This Gets Us Further

| Without This | With This |
|--------------|-----------|
| Guess at what's broken | See what's broken (per-oracle visibility) |
| Debate MRR in lab | Learn from real queries (feedback logging) |
| One-size-fits-all | Modes for different intents |
| Structural signals unused | Structural as metadata on every response |
| No path to improvement | Feedback → error analysis → targeted fixes |

This is infrastructure for learning, not a feature.

## References

- [spec-work-deferred.md](spec-work-deferred.md) — StructuralOracle lessons (prior vs relevance)
- [spec-robust-signals.md](spec-robust-signals.md) — Signal definitions
- [spec-pipeline.md](spec-pipeline.md) — scrape → oxidize/assay → scry architecture
