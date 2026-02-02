---
type: feat
id: epistemic-e4.6c
status: complete
created: 2026-02-02
updated: 2026-02-02
sessions:
  origin: 20260202-130018
related:
  - layer/surface/build/feat/epistemic-layer/SPEC.md
  - layer/surface/build/feat/epistemic-layer/design.md
---

# feat: Forge Semantic Integration (E4.6c)

> Issues and PRs are scraped but invisible to semantic search — embed them.

**Parent:** [epistemic-layer](../SPEC.md) phase E4.6
**Prerequisite:** Forge scraper operational (it is). Discovery from session 20260202-130018.

---

## Problem

Forge data (GitHub/Gitea issues and PRs) is scraped into eventlog as `forge.issue`
and `forge.pr` events with materialized views (`forge_issues`, `forge_prs`). But forge events
are **never embedded** — `oxidize` skips them entirely. Issues and PRs are lexical-only citizens:
findable via `scry --include-issues` (FTS5 text match) but invisible to semantic search, belief
grounding, and impact analysis.

**Content type reach audit:**

| Content | Scraped | Embedded | Lexical | Semantic Neighbors | Grounding Hop |
|---------|---------|----------|---------|-------------------|---------------|
| Code functions | Yes | Yes (1B offset) | Yes | Yes | Multi-hop (E4.6a-fix) |
| Commits | Yes | Yes (3B offset) | Yes | Yes | Direct cosine |
| Sessions | Yes | Yes (eventlog seq) | Yes | Yes | Direct cosine |
| Patterns | Yes | Yes (2B offset) | Yes | Yes | Direct cosine |
| Beliefs | Yes | Yes (4B offset) | Yes | Yes | Direct cosine |
| **Forge issues** | **Yes** | **No** | **Yes (opt-in)** | **No** | **None** |
| **Forge PRs** | **Yes** | **No** | **Yes (opt-in)** | **No** | **None** |

This means: an issue titled "eventlog module should be shared infrastructure" is invisible when
computing grounding for the `eventlog-is-infrastructure` belief. A PR titled "refactor: extract
eventlog to lib" that directly implements a belief produces no semantic signal.

---

## Solution

Add forge events to the `oxidize` embedding pipeline with a new ID offset range.

**ID offset scheme (extend existing):**

```
0          - 999,999,999   → eventlog entries (sessions, observations)
1,000,000,000 - 1,999,999,999 → CODE_ID_OFFSET (function_facts)
2,000,000,000 - 2,999,999,999 → PATTERN_ID_OFFSET (patterns)
3,000,000,000 - 3,999,999,999 → COMMIT_ID_OFFSET (commits)
4,000,000,000 - 4,999,999,999 → BELIEF_ID_OFFSET (beliefs)
5,000,000,000 - 5,999,999,999 → FORGE_ID_OFFSET (issues + PRs)  ← NEW
```

**Embedding content for forge events:**
- Issues: `"{title}\n{body}"` (title + body, truncated to embedding model's context window)
- PRs: `"{title}\n{body}"` (same format — PRs are issues with merge metadata)

---

## What This Enables

- `scry "authentication"` finds semantically relevant issues without `--include-issues` flag
- Belief grounding walks through forge events: belief → nearest issues/PRs → linked commits/files
- Impact analysis: when code changes, surface not just beliefs but related issues
- Multi-hop: belief → issue → PR → merge commit → commit_files → code (full traceability)

---

## Build Steps

- [x] 1. Add `FORGE_ID_OFFSET` constant (5B) to enrichment.rs offset scheme
- [x] 2. Add forge event embedding to oxidize pipeline — read from eventlog directly
  (forge_issues/forge_prs views have broken event_seq; queried eventlog WHERE event_type IN)
- [x] 3. Add forge enrichment to `enrich_results()` — resolve forge keys via eventlog seq
- [x] 4. Semantic oracle has no include_issues gate — forge results appear naturally after embedding
- [x] 5. Add forge to belief grounding — grounding_forge_count column, 24/47 beliefs have forge neighbors
- [x] 6. Tested: 81 forge events embedded, issue #26 + PR #27 appear in semantic scry results

---

## Exit Criteria

- [x] Forge issues/PRs appear in semantic scry results without `--include-issues`
- [x] `scry --belief <id>` finds semantically related issues
- [x] Belief grounding counts include forge neighbors (24/47 beliefs)
- [x] No regression on existing semantic search quality (307 tests pass)

---

## Constraint Note

This project uses forge data from a single GitHub repo. The embedding cost is proportional to
issue+PR count — typically hundreds, not millions. Local-first, edge-hardware compatible. The
same model and projection used for all other content types.

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-02 | ready | Specced during session 20260202-130018 |
| 2026-02-02 | complete | Built during session 20260202-155143 — 81 events embedded, 24/47 beliefs with forge grounding |
