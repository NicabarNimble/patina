---
type: feat
id: d3-two-step-retrieval
status: complete
created: 2026-02-02
updated: 2026-02-04
sessions:
  origin: 20260202-202802
  design-resolution: 20260203-065424
  implementation: 20260204-142556
related:
  - layer/surface/build/feat/mother-delivery/SPEC.md
  - layer/surface/build/feat/mother-delivery/d1-belief-oracle/SPEC.md
  - layer/surface/build/feat/mother-delivery/design.md
---

# feat: D3 — Two-Step Retrieval (Search → Fetch)

> Default scry returns snippets. Full content on demand. Same behavior in both MCP and CLI.

## Problem

`scry` returns full content for every result via MCP (`server.rs:1165`). CLI already truncates display to ~200 chars via `truncate_content()` (`enrichment.rs:400-407`), but this is display-only truncation — the underlying data is still full content, and MCP (the primary LLM interface) sends everything. 10 MCP results x full function bodies + annotations + impact analysis = heavy token load. LLMs pay for tokens they don't need.

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
- **[code]**: function signature + first doc comment line, truncated to ~200 chars. CLI already uses `truncate_content()` at this length for display — D3 makes this the actual response format for MCP too. `pub fn query_with_options(&self, query: &str, ...) -> Result<Vec<FusedResult>> // Query with intent detection and...`
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

- [x] Snippets by default in BOTH MCP and CLI — `snippet()` in `src/retrieval/snippet.rs`, MCP `format_results()` uses it, CLI already truncated via `truncate_content()`
- [x] `--detail` / `mode=detail` fetches single result — `patina scry --detail <query_id> --rank <N>` and `scry(mode="detail", ...)` return full content from eventlog
- [x] `--full` / `mode=full` escape hatch — preserves full-content behavior, deprecated from day one
- [x] Token efficiency measured — see results below

### Token Efficiency Measurement (2026-02-04, session [[20260204-142556]])

10 queries, limit=10, CLI and MCP paths:

| Path | Snippets | Full | Reduction |
|------|----------|------|-----------|
| CLI  | ~4,406 tokens | ~4,723 tokens | **6%** |
| MCP  | ~2,467 tokens | ~2,596 tokens | **4%** |

**Why modest?** The enrichment step already produces compact descriptions (not raw code). Beliefs pass through fully (<150 chars). Commits are already "SHA: subject" format. Code descriptions are "Function `name` in `file`, params: ..." which is often under 200 chars.

**The real value is capability, not optimization.** D3's contribution is `--detail` (scan-then-focus), not token savings. The LLM can now scan 10 snippets and drill into the one it needs, instead of receiving everything upfront. See [[capability-not-optimization]].

Token savings will increase as the index grows and code content gets richer (full function bodies from tree-sitter, longer pattern documents, multi-paragraph session summaries).

---

## Implementation Notes (2026-02-04)

**Commit:** [[commit-0c4c5500]] `feat(d3): two-step retrieval — snippets by default, detail on demand`

**Files changed (1 new, 5 modified):**
- `src/retrieval/snippet.rs` — NEW: type-aware snippet extraction, UTF-8 safe, 7 tests
- `src/retrieval/mod.rs` — wire snippet module
- `src/mcp/server.rs` — `format_results()` uses snippets; `detail`/`full` modes; `handle_detail()` + `format_detail_content()`; MCP schema updated
- `src/commands/scry/mod.rs` — `full` field on ScryOptions; `execute_detail()` + `format_detail()`; belief prefix stripping
- `src/commands/scry/internal/hybrid.rs` — `--full` bypasses truncation
- `src/main.rs` — `--detail`, `--rank`, `--full` CLI args

---

## See Also

- [[design.md]] — ADR-2 (Why two-step applies to both MCP and CLI)
- [[d2-three-layer-delivery/SPEC.md]] — D2 depends on D3's response shape being stable
- [[../SPEC.md]] — Parent spec (implementation order, non-goals)
- [[capability-not-optimization]] — Belief captured from D3: frame features as capabilities, not optimizations
