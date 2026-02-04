---
type: feat
id: mother-delivery
status: design
created: 2026-02-02
updated: 2026-02-03
sessions:
  origin: 20260202-202802
  design-resolution: 20260203-065424
  decomposition: 20260203-120615
related:
  - layer/surface/build/feat/mother/SPEC.md
  - layer/surface/build/feat/v1-release/SPEC.md
  - layer/surface/build/feat/epistemic-layer/SPEC.md
  - layer/surface/build/refactor/mother-naming/SPEC.md
beliefs:
  - beliefs-valuable-for-knowledge-not-task
---

# feat: Mother Delivery Layer

> Get the right knowledge to the LLM at the right moment — across projects, adapter-agnostic, through MCP.

## Problem

Patina has the richest knowledge layer of any open-source dev tool: 47+ beliefs with verification, semantic/temporal/dependency indices across 19 repos, a relationship graph with weight learning. None of it reaches the LLM when it matters.

**The measurement (A/B eval, session 20260202-151214):**
- Knowledge-seeking queries: **+2.2 delta** (beliefs help enormously)
- Task-oriented queries: **-0.05 delta** (beliefs actively hurt)

Task queries are the dominant use case. The system that's built doesn't serve it.

**Root cause:** Beliefs are wired as post-processing annotations on code results. The LLM calls `scry`, gets code, and beliefs appear as small tags that get ignored. The belief data is correct — it's an orchestration problem, not a data problem.

---

## Design Changes

Six changes, decomposed into focused sub-specs:

| ID | Change | Sub-Spec |
|----|--------|----------|
| **D0** | Unified search — QueryEngine as default CLI path | [[d0-unified-search/SPEC.md]] |
| **D1** | BeliefOracle — beliefs as default search channel | [[d1-belief-oracle/SPEC.md]] |
| **D2** | Three-layer delivery (description → response → breadcrumbs) | [[d2-three-layer-delivery/SPEC.md]] |
| **D3** | Two-step retrieval (snippets → detail) | [[d3-two-step-retrieval/SPEC.md]] |
| **D4** | Simplify routing to graph-only | Inline below |
| **D5** | Mother naming cleanup (mothership → mother) | [[mother-naming/SPEC.md]] (existing) |

**Principle:** Delivery through MCP and CLI. Our adapters (Claude Code, OpenCode, Gemini CLI) consume both interfaces. No CLAUDE.md instructions, no adapter-specific hooks, no system prompt injection. Steer adapters into patina with minimal surface per adapter.

**Platform:** Mac-focused. Linux support for containerized agents (Docker). Zero Windows.

### D4: Simplify Routing to Graph-Only (inline)

**Current:** Three routing strategies (Daemon, All, Graph) with `--routing` flag. Default is `All` when no flag is passed (`main.rs:963`: `.unwrap_or(RoutingStrategy::All)`).

**Change:** Graph routing is the only strategy. Daemon becomes a transport layer under graph routing. "All" is removed. This also changes the default behavior — queries without `--routing` currently search all 19 repos (brute-force), after D4 they search local-only unless graph edges exist.

```
scry query flow:
  1. Always search local project (weight 1.0)
  2. If --all-repos or --repo:
     graph.get_related() → search related repos → apply edge weights
  3. If no graph edges for this project: return local results only
  4. If PATINA_MOTHER env set: same logic, oracles execute over HTTP

Daemon = graph routing where oracle execution happens remotely
All    = removed (measured failure: 0% repo recall at G0)
Graph  = the only strategy
```

**Implementation:**
- Remove `RoutingStrategy` enum and `--routing` flag
- `execute_graph_routing()` becomes the sole cross-repo path
- `execute_via_mother()` becomes a transport wrapper around graph routing
- `execute_all_repos()` deleted (measured baseline, no longer needed)
- If no graph edges exist, just return local results (not brute-force noise)

---

## Implementation Order

```
D0: Unified search (QueryEngine as default CLI path)
 │   Foundation — one pipeline for CLI and MCP
 │   Removes --hybrid, --lexical, --dimension flags
 │   CLI output converges on FusedResult format
 │
D1: BeliefOracle (beliefs as search channel)
 │   Wires into QueryEngine once, works everywhere
 │   Directly addresses -0.05 delta
 │
D4: Routing simplification (graph-only)
 │   Independent cleanup — can happen alongside D0/D1
 │
D3: Two-step retrieval (search → detail)
 │   Implements snippets on FusedResult (one formatter)
 │   Depends on D0 for unified output format
 │
D2: Context as briefing (dynamic beliefs + recall directive)
 │   Depends on D1 (belief querying) and D3 (response shape)
 │
D5: Naming cleanup (mothership → mother) ✅ COMPLETE
     Verified 2026-02-03 — all renames applied
```

D0 is the foundation — everything else is simpler because there's one pipeline. D0 and D4 can be worked in parallel (both are cleanup/simplification). D1 follows D0 (wires BeliefOracle into the now-unified QueryEngine). D3 follows D0 (implements snippets on the now-unified FusedResult format). D2 follows D1+D3.

---

## Exit Criteria

### v0.11.0 — Required

Rollup criteria. Sub-spec checkboxes must all pass for these to be checked.

- [ ] **D0: One search pipeline.** CLI `patina scry "query"` uses QueryEngine with all oracles — no `--hybrid` flag, same output format as MCP. Sub-spec: [[d0-unified-search/SPEC.md]]
- [ ] **D1: Beliefs surface in default queries.** Run `patina scry "how should I handle errors?"` — beliefs appear in results alongside code/commits without `mode=belief`. Sub-spec: [[d1-belief-oracle/SPEC.md]]
- [x] **D2: Tool descriptions and recall directive live.** `context` response includes dynamic beliefs + recall directive. `scry` and `context` MCP descriptions include belief/recall language. CLI `patina context` command added. Sub-spec: [[d2-three-layer-delivery/SPEC.md]] ✅ 2026-02-04
- [x] **D3: Snippets are the default.** `scry` returns compact snippets; `--detail` returns full content for a single result; `--full` preserves legacy behavior. Sub-spec: [[d3-two-step-retrieval/SPEC.md]] ✅ 2026-02-04
- [x] **D4: `--routing` flag removed.** Graph routing is the sole cross-repo strategy, default is local-only. ✅ 2026-02-04
- [x] **D5: Naming cleanup applied.** mothership → mother across codebase. ✅ Verified 2026-02-03.
- [ ] **A/B eval passes.** Task-oriented delta >= 0.0 (beliefs no longer hurt). 10 queries, same methodology as session 20260202-151214.
- [x] **Token efficiency measured.** CLI 6%, MCP 4% reduction. Modest because enrichment already compact. Real value is scan-then-focus capability. ✅ 2026-02-04

### v0.11.0 — Stretch

Land if time permits, otherwise carried to v0.12.0.

- [ ] **D2 Layer 3: Graph breadcrumbs in results.** Belief results show links (attacks/supports/reaches), code results show belief impact + structural edges, dig-deeper commands formatted per delivery channel.
- [ ] **D2 cross-project beliefs in context.** `context` response includes beliefs from related projects via graph traversal (depends on D1 federation being validated).
- [ ] **A/B eval stretch target.** Task-oriented delta >= +0.5.

### Federation (v0.12.0 or later)

Carried from Phase 1. Depends on D1 local belief search being validated first.

- [ ] Cross-project belief search works via graph routing — query in project A surfaces beliefs from project B via relationship edge
- [ ] Results tagged with provenance — every result shows `[project:channel]` origin
- [ ] `patina mother sync` populates all 19 registered repos as graph nodes
- [ ] Graph routing still passes G2 benchmark (100% repo recall for targeted queries)

---

## Non-Goals

- **Intent classifier** — OpenClaw and Gastown prove you don't need one. Mandatory recall + beliefs-as-channel is simpler and works.
- **Mother semantic tier** (beliefs.usearch, patterns.usearch at ~/.patina/mother/) — deferred. Local project belief search + graph routing is sufficient for v0.11.0.
- **Automatic edge creation (G3)** — still deferred. Manual edges + weight learning is sufficient for 19 repos.
- **Context pressure distillation** — interesting (from OpenClaw), but a prompt engineering concern for session skills, not a code change for this milestone.
- **Adapter-specific hooks** — no SessionStart hooks, no CLAUDE.md instructions. All delivery through MCP tools and CLI commands.
- **Windows support** — zero Windows/Microsoft. Mac-focused with Linux for containerized agents.

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-02 | design | Initial monolithic spec (session 20260202-202802) |
| 2026-02-03 | design | Resolved D1/D3/recall design questions (session 20260203-065424) |
| 2026-02-03 | design | Decomposed into sub-specs, grounded against codebase (session 20260203-120615) |
| 2026-02-03 | design | Added D0: discovered CLI/MCP bifurcation, unified search as foundation |

---

## See Also

- [[analysis-three-servers.md]] — Historical analysis: how CLI/MCP/serve became three independent search paths
- [[design.md]] — Ref repo evidence (OpenClaw, Gastown), all ADRs, resolved design questions
- [[d0-unified-search/SPEC.md]] — Foundation: QueryEngine as default CLI path
- [[d1-belief-oracle/SPEC.md]] — BeliefOracle design and implementation
- [[d2-three-layer-delivery/SPEC.md]] — Three-layer delivery design
- [[d3-two-step-retrieval/SPEC.md]] — Two-step retrieval design
- **Parent spec:** [[mother]] (`layer/surface/build/feat/mother/SPEC.md`)
- **Naming cleanup:** [[mother-naming]] (`layer/surface/build/refactor/mother-naming/SPEC.md`)
- **v1 release:** [[v1-release]] (`layer/surface/build/feat/v1-release/SPEC.md`)
- **A/B eval session:** [[20260202-151214]]
- **Design resolution session:** [[20260203-065424]]
