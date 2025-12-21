# Spec: Observable Scry

**Status:** Planned
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

Current output:
```
1. ./src/retrieval/engine.rs::rrf_fuse (0.82)
2. ./src/retrieval/fusion.rs::fuse (0.79)
```

**Default output** (concise, one line per result):
```
1. engine.rs::rrf_fuse     (sem 0.91 | lex RRF,fuse | 2d | imp 12)
2. fusion.rs::fuse         (sem 0.85 | lex fuse | 14d | imp 4)
```

Format: `(sem <score> | lex <matches> | <recency> | imp <importers>)`

**With `--explain`** (full breakdown):
```
1. ./src/retrieval/engine.rs::rrf_fuse
   Semantic: 0.91
   Lexical: matched "RRF", "fuse"
   Temporal: 2 days ago, 3 commits this week
   Structural: 12 importers, high centrality, not test

2. ./src/retrieval/fusion.rs::fuse
   Semantic: 0.85
   Lexical: matched "fuse"
   Temporal: 14 days ago
   Structural: 4 importers, medium centrality
```

**MCP: Stable JSON Schema**

For programmatic access, MCP returns structured data (not pretty text):

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
        "semantic": { "score": 0.91 },
        "lexical": { "score": 0.78, "matches": ["RRF", "fuse"] },
        "temporal": { "score": 0.65 }
      },
      "annotations": {
        "temporal": {
          "last_modified_days": 2,
          "commits_this_week": 3
        },
        "structural": {
          "importers": 12,
          "centrality": "high",
          "flags": ["entry_point"]
        }
      }
    }
  ]
}
```

This schema enables agents to reason over results, not just display them.

Structure is metadata, not ranking signal.

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

### Phase 1: Structured Response

| Task | Scope |
|------|-------|
| Refactor `QueryEngine::query` to return per-oracle results alongside fused | ~50 lines |
| Add `--explain` flag to scry CLI that shows oracle contributions | ~30 lines |
| Update MCP scry tool to include oracle breakdown in response | ~20 lines |
| Surface structural signals as annotations (from module_signals table) | ~40 lines |

### Phase 2: Explicit Modes

| Task | Scope |
|------|-------|
| Add `scry orient <path>` subcommand (structural-first ranking) | ~60 lines |
| Add `scry recent <query>` subcommand (temporal-first) | ~40 lines |
| Add `scry why <doc_id>` subcommand (explain single result) | ~50 lines |
| Update MCP with mode variants or mode parameter | ~30 lines |

### Phase 3: Feedback Logging

| Task | Scope |
|------|-------|
| Add `scry.use` event type to eventlog | ~20 lines |
| Add `scry open <query_id> <rank>` — opens file, logs usage automatically | ~40 lines |
| Add `scry copy <query_id> <rank>` — copies to clipboard, logs usage | ~30 lines |
| Add MCP callback for "result used" (agent reports which result it consumed) | ~30 lines |
| Add `scry feedback <query_id> good\|bad` for optional explicit rating | ~20 lines |
| Create SQL view for query analysis (queries joined with usage/feedback) | ~20 lines |

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
