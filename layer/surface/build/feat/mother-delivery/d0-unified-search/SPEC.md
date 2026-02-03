---
type: feat
id: d0-unified-search
status: design
created: 2026-02-03
updated: 2026-02-03
sessions:
  origin: 20260203-120615
related:
  - layer/surface/build/feat/mother-delivery/SPEC.md
  - layer/surface/build/feat/mother-delivery/d1-belief-oracle/SPEC.md
  - layer/surface/build/feat/mother-delivery/d3-two-step-retrieval/SPEC.md
  - layer/surface/build/feat/mother-delivery/design.md
---

# feat: D0 — Unified Search (QueryEngine as Default)

> One search pipeline for CLI and MCP. No flags. Every query gets oracles + RRF fusion.

## Problem

CLI and MCP scry are different pipelines:

| Path | When | Pipeline | Uses Oracles? |
|------|------|----------|---------------|
| MCP `find` | Every MCP call | QueryEngine → 4 oracles → RRF | Yes |
| CLI `--hybrid` | Explicit flag | QueryEngine → 4 oracles → RRF | Yes |
| CLI default | Most CLI calls | Heuristic → scry_text() OR scry_lexical() | **No** |

The CLI default auto-detects between direct vector search and FTS5, bypassing the oracle system entirely. This means:
- D1 (BeliefOracle) only works for MCP and CLI `--hybrid`
- D3 (snippets) needs two implementations (ScryResult formatter + FusedResult formatter)
- D2 (delivery) has inconsistent behavior across interfaces
- The adapter pattern is violated: same LLM gets different results depending on which interface it calls

**History:** `--hybrid` was introduced Dec 16, 2025 (`49cf30c4`) as experimental — "needs feedback-driven tuning before becoming default." MCP was built later and used the oracle path from day one. MCP has been running QueryEngine for every query since January 2026. The experimental phase is over.

**Ref repo evidence:** OpenClaw has ONE search path (`memory_search` → hybrid scoring → snippets). Gastown has ONE delivery path per role. OpenCode calls CLI via exec expecting consistent behavior. None have dual search paths. The bifurcation is a Patina development artifact, not an architectural choice.

---

## Design

Make QueryEngine the default for ALL scry paths. What was `--hybrid` becomes standard. No flag needed.

### Before (three paths)

```
CLI execute()
  ├─→ mother configured? → execute_via_mother()
  ├─→ --all-repos? → routing match (graph/all)
  ├─→ --hybrid? → QueryEngine (4 oracles + RRF)     ← opt-in
  └─→ default: heuristic auto-detect
      ├─→ --belief? → scry_belief()
      ├─→ --file? → scry_file()
      └─→ text? → scry_text() OR scry_lexical()      ← bypasses oracles
```

### After (one path)

```
CLI execute()
  ├─→ mother configured? → execute_via_mother()
  ├─→ --all-repos? → execute_graph_routing()
  ├─→ --belief? → scry_belief()                       (explicit belief grounding, unchanged)
  ├─→ --file? → scry_file()                           (file co-change query, unchanged)
  └─→ default: QueryEngine (5 oracles + RRF)          ← always
      ├─→ SemanticOracle  (wraps scry_text logic)
      ├─→ LexicalOracle   (wraps scry_lexical logic)
      ├─→ TemporalOracle  (co-change clusters)
      ├─→ PersonaOracle   (cross-project knowledge)
      └─→ BeliefOracle    (D1, wires in naturally)
```

`--belief` and `--file` remain as specialized query modes — they're not "search everything" queries, they're "find neighbors of this specific entity" queries. The default text query always uses the full oracle pipeline.

### What Changes

**Remove:**
- `--hybrid` flag from CLI args (`main.rs`)
- `hybrid` field from `ScryOptions` struct (`mod.rs:65`)
- `execute_hybrid()` function (`hybrid.rs`) — its logic moves into the default path
- Heuristic auto-detection (`is_lexical_query()`) — oracles handle this internally (LexicalOracle runs FTS5, SemanticOracle runs vector, RRF fuses them)
- `--lexical` flag — LexicalOracle already participates in every query via RRF
- `--dimension` flag — the oracle system handles dimension selection internally

**Keep (as oracle internals):**
- `scry_text()` — SemanticOracle calls equivalent logic internally
- `scry_lexical()` — LexicalOracle calls equivalent logic internally
- `enrich_results()` — still used by oracles to hydrate results from SQLite

**Add:**
- `--legacy` escape hatch — preserves old direct-search behavior during transition. Deprecated from day one, removed in v0.12.0.

### Output Format Convergence

CLI currently outputs `ScryResult` (event_type, source_id, score). After D0, CLI outputs `FusedResult` (doc_id, fused_score, oracle contributions, metadata).

This is the same format MCP already uses. D3 (snippets) then only needs to implement one formatter.

```
# CLI output after D0 (same as MCP):
query_id: q_20260203_143000_abc

1. [code]   src/retrieval/engine.rs::query_with_options  (0.87)
            SemanticOracle(0.92) + LexicalOracle(0.71)
2. [belief] error-handling-thiserror                     (0.83)
            BeliefOracle(0.83)
3. [commit] abc1234: "feat: add cosine distance"         (0.79)
            TemporalOracle(0.85) + LexicalOracle(0.68)
```

### Performance

The oracle system runs oracles in parallel via `rayon`. Each oracle is sub-millisecond (USearch ANN, FTS5 BM25). RRF fusion is O(n) over results. Total overhead vs direct search: negligible for the query sizes we handle (~10-50 results).

MCP has been running this path for every query since January with no performance complaints.

### What This Enables

- **D1 wires in once.** Add BeliefOracle to `default_oracles()`, done. Works for CLI and MCP.
- **D3 implements once.** Snippet formatting on FusedResult. One formatter, two interfaces.
- **D2 is consistent.** Breadcrumbs, recall directives, dig-deeper commands — one output format to enhance.
- **Intent detection works everywhere.** `detect_intent()` → `IntentWeights` → weighted RRF already exists. CLI gets it for free.

---

## Exit Criteria

- [ ] Default CLI `patina scry "query"` uses QueryEngine with all oracles (no `--hybrid` flag)
- [ ] `--hybrid` flag removed from CLI args
- [ ] `--lexical` and `--dimension` flags removed (oracles handle internally)
- [ ] `--legacy` escape hatch available for old direct-search behavior
- [ ] CLI output format matches MCP output format (FusedResult with oracle contributions)
- [ ] `--belief` and `--file` modes unchanged (specialized, not default query path)
- [ ] `patina eval` benchmarks pass — no regression in retrieval quality vs old default

---

## See Also

- [[design.md]] — ADR-7 (to be added: Why unify CLI search path)
- [[d1-belief-oracle/SPEC.md]] — Depends on D0: BeliefOracle wires into QueryEngine which is now the only path
- [[d3-two-step-retrieval/SPEC.md]] — Depends on D0: snippets implement on FusedResult only
- [[../SPEC.md]] — Parent spec (implementation order updated)
