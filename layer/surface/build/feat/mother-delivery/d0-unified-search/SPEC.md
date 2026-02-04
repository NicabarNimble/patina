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
beliefs:
  - mcp-is-shim-cli-is-product
---

# feat: D0 — Unified Search (CLI Owns the Pipeline)

> One search pipeline. CLI is the product. MCP wraps it. Every query gets oracles + RRF fusion.

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

### CLI-First Architecture

**Belief: [[mcp-is-shim-cli-is-product]]** — MCP exists as a discovery shim so LLM adapters know what tools to call. The CLI is the real interface.

**Before D0:** MCP has its own implementation in `server.rs` — `format_results()`, `format_results_with_query_id()`, `get_project_context()` — parallel to CLI code. Two formatters, two code paths, two behaviors.

**After D0:** CLI owns the QueryEngine pipeline and output formatting. MCP's `server.rs` becomes a thin dispatcher: parse MCP JSON params → call the same code `patina scry` uses → return the output. MCP tool descriptions remain (that's the discovery shim — the reason MCP exists), but MCP handlers delegate to CLI logic.

```
# CLI is the product:
patina scry "how to handle errors"

# MCP wraps CLI:
scry(query="how to handle errors")  →  calls same code path  →  same output
```

### Output Format

CLI outputs `FusedResult` (doc_id, fused_score, oracle contributions, metadata). MCP returns the same output — it's a passthrough, not a re-formatter.

```
query_id: q_20260203_143000_abc

1. [code]   src/retrieval/engine.rs::query_with_options  (0.87)
            SemanticOracle(0.92) + LexicalOracle(0.71)
2. [belief] error-handling-thiserror                     (0.83)
            BeliefOracle(0.83)
3. [commit] abc1234: "feat: add cosine distance"         (0.79)
            TemporalOracle(0.85) + LexicalOracle(0.68)
```

D3 (snippets) implements one formatter in CLI code. MCP passes it through.

### Performance

The oracle system runs oracles in parallel via `rayon`. Each oracle is sub-millisecond (USearch ANN, FTS5 BM25). RRF fusion is O(n) over results. Total overhead vs direct search: negligible for the query sizes we handle (~10-50 results).

MCP has been running the QueryEngine path for every query since January with no performance complaints — validating the oracle system is production-ready.

### What This Enables

- **D1 wires in once.** Add BeliefOracle to `default_oracles()`, done. Works everywhere.
- **D3 implements once.** Snippet formatting in CLI code. MCP passes through.
- **D2 is consistent.** One output format to enhance — CLI owns it.
- **Intent detection works everywhere.** `detect_intent()` → `IntentWeights` → weighted RRF already exists.
- **MCP surface shrinks.** `server.rs` handlers become thin dispatchers, not parallel implementations.

---

## Exit Criteria

- [ ] Default CLI `patina scry "query"` uses QueryEngine with all oracles (no `--hybrid` flag)
- [ ] `--hybrid` flag removed from CLI args
- [ ] `--lexical` and `--dimension` flags removed (oracles handle internally)
- [ ] `--legacy` escape hatch available for old direct-search behavior
- [ ] CLI output format uses FusedResult with oracle contributions
- [ ] MCP scry handler delegates to CLI code path (thin wrapper, not parallel implementation)
- [ ] `--belief` and `--file` modes unchanged (specialized, not default query path)
- [ ] `patina eval` benchmarks pass — no regression in retrieval quality vs old default

---

## See Also

- [[../analysis-three-servers.md]] — Historical analysis: how CLI/MCP/serve became three independent search paths (grounded in git + sessions)
- [[design.md]] — ADR-7 (to be added: Why unify CLI search path)
- [[d1-belief-oracle/SPEC.md]] — Depends on D0: BeliefOracle wires into QueryEngine which is now the only path
- [[d3-two-step-retrieval/SPEC.md]] — Depends on D0: snippets implement on FusedResult only
- [[../SPEC.md]] — Parent spec (implementation order updated)
