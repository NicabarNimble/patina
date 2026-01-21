# Spec: Vocabulary Gap

**Status:** Complete
**Created:** 2026-01-08
**Completed:** 2026-01-21
**Tag:** spec/vocabulary-gap
**Origin:** Phase 0.25b benchmark revealed temporal MRR 0.100 (target: 0.4)

**Phase 1 Implementation:** Commit `1df7ecce`
- Added `expanded_terms` parameter to MCP scry tool schema
- LLM can provide code-specific synonyms to improve FTS5 matching
- Non-deterministic: depends on LLM using the parameter

---

## Problem

FTS5 keyword matching fails when user vocabulary differs from codebase vocabulary.

```
User query:     "when did we add commit message search"
Ground truth:   "feat(scrape): add commits_fts table for git narrative search"
```

The user says "message search" but the code says "commits_fts" and "narrative". Intent-aware weights don't help because they boost lexical globally, not commits specifically.

**Evidence:** Temporal queryset MRR 0.100 vs baseline 0.133 (regression).

---

## Root Cause

Same issue found in two contexts:
1. **Temporal queries** (this spec) - user terms vs commit message terms
2. **Ref repo semantic** (session 20260107) - query "unification algorithm" vs code "unify"

The gap is **domain terminology** - users describe concepts, code uses implementation terms.

---

## Solutions (by effort/impact)

| Solution | Effort | Impact | Notes |
|----------|--------|--------|-------|
| **LLM query expansion** | Low | High | MCP param, LLM rewrites query |
| **Semantic search on commits** | Medium | High | Embed commit messages |
| **Separate commits oracle** | Medium | Medium | Don't mix with code symbols |
| **FTS5 stemming** | Low | Low | "search" → "search*" |

---

## Phase 1: LLM Query Expansion (Recommended)

**Insight:** We have a frontier LLM as the UI. Let it do the vocabulary bridging.

**MCP Schema Change:**

```json
{
  "name": "scry",
  "inputSchema": {
    "properties": {
      "query": { "type": "string" },
      "expanded_terms": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Additional search terms the LLM derives from user query"
      }
    }
  }
}
```

**Flow:**
```
User: "when did we add commit message search"
              │
              ▼
LLM understands domain, expands:
  expanded_terms: ["commits_fts", "git narrative", "FTS5"]
              │
              ▼
Scry searches: original + expanded terms
              │
              ▼
Better recall on domain vocabulary
```

**Tasks:**

| # | Task | Effort |
|---|------|--------|
| 1 | Add `expanded_terms` param to MCP schema | ~10 lines |
| 2 | Modify LexicalOracle to include expanded terms | ~20 lines |
| 3 | Update MCP tool description with expansion guidance | ~5 lines |
| 4 | Measure temporal MRR with expansion | benchmark |

**Exit Criteria:**
- [ ] Temporal MRR > 0.4 (baseline: 0.133)
- [ ] No regression on non-temporal queries

---

## Phase 2: Commit Semantic Search (If Needed)

If LLM expansion isn't enough, add semantic search over commits.

**Architecture:**

```
commits table
      │
      ▼
oxidize semantic --commits
      │
      ▼
commits.usearch (embedded messages)
      │
      ▼
CommitsOracle (new, parallel to SemanticOracle)
```

**Why this helps:** Embeddings capture semantic similarity, bridging "message search" ↔ "commits_fts".

**Tasks:**

| # | Task | Effort |
|---|------|--------|
| 1 | Generate commit message embeddings in oxidize | ~50 lines |
| 2 | Create CommitsOracle | ~100 lines |
| 3 | Wire into QueryEngine | ~10 lines |
| 4 | Measure MRR improvement | benchmark |

**Exit Criteria:**
- [ ] CommitsOracle contributes to temporal queries
- [ ] Temporal MRR > 0.4

---

## Non-Goals

- Complex NLP preprocessing (overkill for this problem)
- Fine-tuning embedding model (premature)
- Rebuilding FTS5 with synonyms (fragile)

---

## Related

- [spec-mothership.md](./spec-mothership.md) Phase 0.25b - origin of measurement
- [spec-ref-repo-semantic.md](./spec-ref-repo-semantic.md) - same vocabulary gap in code search
- Session 20260107-204850 - ref repo eval identified terminology gap
