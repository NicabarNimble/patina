---
type: feat
id: d3-two-step-retrieval
status: design
created: 2026-02-02
updated: 2026-02-03
sessions:
  origin: 20260202-202802
  design-resolution: 20260203-065424
related:
  - layer/surface/build/feat/mother-delivery/SPEC.md
  - layer/surface/build/feat/mother-delivery/d1-belief-oracle/SPEC.md
  - layer/surface/build/feat/mother-delivery/design.md
---

# feat: D3 — Two-Step Retrieval (Search → Fetch)

> Default scry returns snippets. Full content on demand. Same behavior in both MCP and CLI.

## Problem

`scry` returns full content for every result. 10 results x full function bodies + annotations + impact analysis = heavy token load. Both MCP and CLI return the same verbose output. LLMs pay for tokens they don't need.

---

## Design

Default scry returns **snippets** in BOTH interfaces. Both MCP and CLI are LLM tools — Claude Code calls CLI via Bash, OpenCode calls via exec, Gemini CLI calls via MCP. All consumers benefit from token-efficient snippets with on-demand detail.

OpenClaw's `memory_search` always returns 700-char snippets — there is no "full content search" mode. Their two-step is fundamental, not conditional. We adopt the same principle: one behavior, two interfaces.

### Step 1 — Search (default, both MCP and CLI)

```
# CLI (called by Claude Code via Bash, OpenCode via exec)
patina scry "vector similarity search"

# MCP (called by Claude Code, Gemini CLI via MCP)
scry(query="vector similarity search")

# Both return the same snippet format:

  query_id: q_20260202_143000_abc

  1. [code]   src/retrieval/engine.rs::query_with_options  (0.87)
              SemanticOracle(0.92) + LexicalOracle(0.71)
  2. [belief] error-handling-thiserror                     (0.83)
              "Use thiserror derive macros" (entrenchment: 0.8)
              → reaches: src/retrieval/engine.rs (+4 files)
  3. [commit] abc1234: "feat: add cosine distance metric"  (0.79)
              TemporalOracle(0.85) + LexicalOracle(0.68)
  4. [code:USearch] src/index.hpp::search                  (0.76)
              SemanticOracle(0.76) — via LEARNS_FROM

  --- Belief Impact ---
  1 belief matched — dig deeper:
    patina scry --belief error-handling-thiserror --content-type code
```

Each result shows: doc_id, fused score, channel tag, oracle contributions, and a content-type-aware snippet.

**Snippet format per content type:**
- **[code]**: function signature + first doc comment line, truncated to ~200 chars. `pub fn query_with_options(&self, query: &str, ...) -> Result<Vec<FusedResult>> // Query with intent detection and...`
- **[belief]**: full statement + entrenchment. `"Use thiserror derive macros for error types" (entrenchment: 0.8)`
- **[commit]**: subject line + changed file count. `abc1234: "feat: add cosine distance metric" (4 files)`
- **[pattern]**: first sentence of pattern content.

OpenClaw uses 700-char snippets. 120 chars is too aggressive — function signatures alone can exceed that. Target ~200 chars for code, full statement for beliefs (typically <150 chars), subject line for commits.

When D2 Layer 3 (graph breadcrumbs) is implemented, breadcrumbs appear after the snippet. D3 ships without breadcrumbs initially — the snippet format is self-contained and useful on its own.

### Step 2 — Fetch (on demand, both MCP and CLI)

```
# CLI
patina scry --detail q_20260202_143000_abc 1

# MCP
scry(mode="detail", query_id="q_20260202_143000_abc", rank=1)

# Both return:

  src/retrieval/engine.rs::query_with_options
  [full function signature, body, structural annotations]

  Belief impact:
    ← error-handling-thiserror (reach: 0.9)
    ← onnx-runtime-for-ml (reach: 0.7)

  Graph:
    → imports: src/retrieval/fusion.rs, src/retrieval/oracle.rs
    → co-changes: src/mcp/server.rs (28 times)
```

The LLM sees the landscape first, then drills into what matters. This is OpenClaw's `memory_search` → `memory_get` pattern, but our `--detail` preserves search context (you're drilling into a ranked result by query_id + rank, not navigating to a file by path).

### Implementation

- The split happens after RRF fusion, before enrichment
- `enrich_results()` in `enrichment.rs` already reconstructs content from SQLite by ID ranges — this becomes the `detail` path
- `query_id` infrastructure already exists (Phase 3 feedback loop, `log_mcp_query()` in server.rs)
- Add `mode=detail` / `--detail` handler to both MCP server and CLI command
- Default mode returns: `doc_id, fused_score, sources[], content_snippet`
- Content snippets: type-aware formatting (~200 chars for code, full statement for beliefs, subject for commits)
- Graph breadcrumbs added later by D2 Layer 3 — D3 defines the snippet skeleton, D2 adds the breadcrumb section

### Migration: Breaking Change Strategy

Switching to snippets-by-default changes the MCP response shape. All existing consumers (Claude Code, OpenCode) currently receive full content from `scry`.

**Approach: `--full` escape hatch during transition.**
```
# Full content (current behavior, preserved)
patina scry --full "vector similarity search"
scry(query="...", mode="full")

# Snippets (new default)
patina scry "vector similarity search"
scry(query="...")

# Detail (new, single result)
patina scry --detail q_abc 1
scry(mode="detail", query_id="q_abc", rank=1)
```

`--full` is the current behavior under a new flag. Default switches to snippets. This avoids a hard break — existing adapter CLAUDE.md instructions that mention `scry` continue to work, they just get a more compact default. Any workflow that needs full content can opt in with `--full` or `mode=full`.

`--full` is an escape hatch, not a permanent API surface. It can be deprecated once adapters are confirmed working with snippets + detail.

---

## Exit Criteria

- [ ] Snippets by default in BOTH MCP and CLI — default mode returns doc_id + score + content-type-aware snippet (~200 chars for code, full statement for beliefs, subject for commits)
- [ ] `--detail` / `mode=detail` fetches single result — `patina scry --detail <query_id> <rank>` and `scry(mode=detail, ...)` return full content + annotations for one result
- [ ] `--full` / `mode=full` escape hatch — preserves current full-content behavior during transition
- [ ] Token efficiency measured: compare average tokens per scry response before/after

---

## See Also

- [[design.md]] — ADR-2 (Why two-step applies to both MCP and CLI)
- [[d2-three-layer-delivery/SPEC.md]] — D2 depends on D3's response shape being stable
- [[../SPEC.md]] — Parent spec (implementation order, non-goals)
