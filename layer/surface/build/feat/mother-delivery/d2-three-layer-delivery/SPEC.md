---
type: feat
id: d2-three-layer-delivery
status: implementation
created: 2026-02-02
updated: 2026-02-04
sessions:
  origin: 20260202-202802
  design-resolution: 20260203-065424
  revised: 20260204-152405
related:
  - layer/surface/build/feat/mother-delivery/SPEC.md
  - layer/surface/build/feat/mother-delivery/d1-belief-oracle/SPEC.md
  - layer/surface/build/feat/mother-delivery/design.md
---

# feat: D2 — Three-Layer Delivery (Description → Response → Graph Breadcrumbs)

> Deliver knowledge at multiple touch points so the LLM encounters it regardless of which tool it calls first.

## Problem

**Updated 2026-02-04:** D1 (BeliefOracle) and D3 (two-step retrieval) shipped since this spec was written. Beliefs are now a first-class search channel in `scry` — they compete via RRF fusion alongside code, commits, and patterns. The original premise ("beliefs are post-processing annotations, not a search channel") is no longer true.

**What's still broken (observed via live queries):**

1. **`context(topic="error handling")` hides beliefs entirely.** The `show_beliefs` gate (`server.rs:1600-1606`) only shows beliefs when topic is None or contains "belief"/"epistemic". A belief called `error-analysis-over-architecture` exists and is directly relevant — the user never sees it.

2. **Pattern topic filtering is broken.** `read_patterns()` does substring matching on full markdown bodies. `context(topic="error handling")` returns embedding architecture, metal parser, orchestration agent docs — none about error handling. The word "error" appears somewhere in their markdown, so they match.

3. **No CLI `context` command.** Only MCP has it. Any CLI-connected LLM (Claude Code via bash, OpenCode via exec) can't access patterns or beliefs through `context` at all.

4. **Tool descriptions don't mention beliefs.** The `context` description says "Returns core patterns and surface patterns" — no mention of beliefs. LLMs don't know beliefs exist until they stumble on them.

5. **`server.rs` is 2,021 lines** mixing protocol, formatting, handlers, query execution, and context logic. Adding more context features makes a monolith worse. Context logic should be extracted before extending it.

---

## Design

**Revised 2026-02-04.** Original design had three delivery layers. D1 (BeliefOracle) has since solved the core delivery problem — beliefs are now a default search channel in `scry`. The revised design focuses on fixing what's broken and adding what D1 doesn't cover.

### Prerequisite: Extract Context Module

`server.rs` is 2,021 lines mixing five concerns: protocol, schema, handlers, formatting, and context logic. Context logic (`get_project_context()`, `get_belief_metrics()`, `read_patterns()`, `extract_summary()`) is ~250 lines that both MCP and CLI should share.

**Extract to `src/commands/context.rs`** — follows existing pattern where `src/commands/` owns business logic and `src/mcp/server.rs` is a thin MCP adapter. `server.rs` imports from the extracted module; the new CLI command calls it directly.

### Layer 1: Tool/Command Description (the nudge)

**MCP tool descriptions** — recall language already shipped. Remaining: mention beliefs explicitly.

```
scry: "Search codebase knowledge — USE THIS FIRST for any question about the code.
       Fast hybrid search over indexed symbols, functions, types, git history,
       and session learnings. Prefer this over manual file exploration.
       TIP: ..."

context: "Get project patterns and conventions — USE THIS to understand design rules
          before making architectural changes. Returns core patterns (eternal
          principles), surface patterns (active architecture), and project beliefs
          ranked by relevance."
```

**CLI commands:**
- Update `Scry` doc comment to match MCP description tone
- Add `patina context [--topic]` CLI command (thin wrapper over extracted module)

### Layer 2: Response-Level Recall (in the output)

**Rule: beliefs are always eligible.** Topic changes the query and budget, not whether beliefs exist.

- **`topic.is_some()`:** Fetch top N beliefs via `BeliefOracle::query(topic, limit)` (semantic ranking). Fetch patterns via title/filename matching (not full-body substring).
- **`topic.is_none()`:** Show current "starter pack" (all patterns) plus belief aggregate metrics + top beliefs by use count.

This fixes the "`error-analysis-over-architecture` exists but never appears" class of failure.

**Pattern filtering fix:** Stop substring matching on full markdown bodies. Match against filename and extracted title only. `context(topic="error handling")` should return nothing rather than four irrelevant architecture docs.

**Recall directive** in every `context` response:
```
## Recall Directive
Project knowledge accumulates in beliefs — check them before assuming defaults.
  CLI:  patina scry --content-type beliefs "your question"
  MCP:  scry(content_type="beliefs", query="your question")
```

Concrete syntax so the LLM doesn't guess.

### Layer 3: Graph Breadcrumbs (deferred to v0.11.0 stretch)

Graph breadcrumbs (attacks/supports/reaches links, structural edges, dig-deeper commands) are deferred. The basics (Layer 1 + Layer 2) don't work yet — polish on broken plumbing has no value. Layer 3 design from the original spec remains valid for when the foundation is solid.

### Implementation

Five commits, ordered to separate refactor from behavior change:

**Commit 1 — Extract + CLI surface (refactor, zero behavior change):**
- Extract `get_project_context()`, `get_belief_metrics()`, `read_patterns()`, `extract_summary()` to `src/commands/context.rs`
- Add `pub mod context;` to `src/commands/mod.rs`
- `server.rs` calls `crate::commands::context::get_project_context()`
- Add `Context { topic }` variant to `Commands` enum in `main.rs`
- CLI handler: `commands::context::execute(topic)`
- Verify: MCP `context` tool returns identical output. `patina context` CLI works.

**Commit 2 — Fix gating + pattern filtering (behavior fix):**
- Remove `show_beliefs` gate — beliefs always shown regardless of topic
- Change `read_patterns()` topic filter: match filename and title only, not full body
- Title extraction: first `# ` line from markdown (already parsed by `extract_summary()`)
- Verify: `context(topic="error handling")` no longer returns irrelevant docs. Beliefs always appear.

**Commit 3 — Topic-ranked beliefs via BeliefOracle (new capability):**
- Expose `BeliefOracle` query method publicly (currently private to `src/retrieval/oracles/`)
- In `get_project_context()`: when topic is provided, call `BeliefOracle::query(topic, 5)` to get semantically ranked beliefs
- Format ranked beliefs as "Active Beliefs" section with statement, entrenchment, and evidence count
- When no topic: keep existing `get_belief_metrics()` aggregate stats
- Verify: `context(topic="error handling")` returns `error-analysis-over-architecture` belief

**Commit 4 — Recall directive + descriptions (text changes):**
- Append recall directive to every `context` response with both CLI and MCP syntax
- Update MCP `context` description to mention beliefs
- Update MCP `scry` description to mention session learnings
- Update CLI `Scry` doc comment to match MCP tone
- Verify: descriptions visible in `patina scry --help` and MCP tool listing

**Commit 5 — D4 routing cleanup (independent, if time permits):**
- Remove `RoutingStrategy` enum and `--routing` CLI flag
- `execute_graph_routing()` becomes sole cross-repo path
- Delete `execute_all_repos()` (measured failure: 0% repo recall at G0)
- Verify: `patina scry --all-repos "query"` uses graph routing

---

## Exit Criteria

**Prerequisite (refactor):**
- [x] Context logic extracted to `src/commands/context.rs` — `server.rs` imports from it ✅ 2026-02-04
- [x] CLI `patina context [--topic]` command exists and returns same output as MCP tool ✅ 2026-02-04

**Layer 1 (descriptions):**
- [x] MCP tool descriptions include recall language ("USE THIS FIRST", "USE THIS to understand design rules")
- [x] MCP `context` description mentions beliefs explicitly ✅ 2026-02-04
- [x] MCP `scry` description mentions session learnings (already present)
- [x] CLI `Scry` doc comment matches MCP description tone ✅ 2026-02-04

**Layer 2 (response-level recall):**
- [x] Beliefs always shown in context — no gating on topic string ✅ 2026-02-04
- [x] Pattern topic filtering uses filename/title only — `context(topic="error handling")` does not return irrelevant architecture docs ✅ 2026-02-04
- [x] Context with topic returns semantically ranked beliefs — `context(topic="error handling")` returns `error-analysis-over-architecture` at rank 3 (score 0.82) ✅ 2026-02-04
- [x] Recall directive in every context response — includes both CLI and MCP syntax with concrete examples ✅ 2026-02-04

**Deferred to v0.11.0 stretch (Layer 3):**
- [ ] Graph breadcrumbs — belief results show links (attacks/supports/reaches), code results show belief impact + structural edges
- [ ] Dig-deeper commands — every result includes actionable commands formatted for the delivery channel
- [ ] Cross-project beliefs in context response — beliefs from related projects via graph traversal

---

## Revision History

| Date | Change | Rationale |
|------|--------|-----------|
| 2026-02-02 | Original design | Three delivery layers spec |
| 2026-02-03 | Design resolution | Resolved D1/D3/recall questions |
| 2026-02-04 | **Revised** | D1 shipped — beliefs are a search channel. Reframed D2 from "deliver beliefs everywhere" to "fix broken context tool + add CLI surface." Added extraction prerequisite. Moved Layer 3 to stretch. |

---

## See Also

- [[design.md]] — ADR-3 (Why three-layer delivery)
- [[d1-belief-oracle/SPEC.md]] — D1 shipped: beliefs are a default search channel (changes D2 assumptions)
- [[d3-two-step-retrieval/SPEC.md]] — D3 shipped: snippets + detail mode
- [[../SPEC.md]] — Parent spec (implementation order, non-goals)
