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

Five changes, decomposed into focused sub-specs:

| ID | Change | Sub-Spec |
|----|--------|----------|
| **D1** | BeliefOracle — beliefs as default search channel | [[d1-belief-oracle/SPEC.md]] |
| **D2** | Three-layer delivery (description → response → breadcrumbs) | [[d2-three-layer-delivery/SPEC.md]] |
| **D3** | Two-step retrieval (snippets → detail) | [[d3-two-step-retrieval/SPEC.md]] |
| **D4** | Simplify routing to graph-only | Inline below |
| **D5** | Mother naming cleanup (mothership → mother) | [[mother-naming/SPEC.md]] (existing) |

**Principle:** Delivery through MCP and CLI. Our adapters (Claude Code, OpenCode, Gemini CLI) consume both interfaces. No CLAUDE.md instructions, no adapter-specific hooks, no system prompt injection. Steer adapters into patina with minimal surface per adapter.

**Platform:** Mac-focused. Linux support for containerized agents (Docker). Zero Windows.

### D4: Simplify Routing to Graph-Only (inline)

**Current:** Three routing strategies (Daemon, All, Graph) with `--routing` flag.

**Change:** Graph routing is the only strategy. Daemon becomes a transport layer under graph routing. "All" is removed.

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
D1: BeliefOracle (beliefs as search channel)
 │   Highest impact — directly addresses -0.05 delta
 │
D3: Two-step retrieval (search → detail)
 │   Changes MCP response shape — do before D2 to stabilize interface
 │
D2: Context as briefing (dynamic beliefs + recall directive)
 │   Depends on D1 (belief querying) being stable
 │
D4: Routing simplification (graph-only)
 │   Independent cleanup — can happen anytime
 │
D5: Naming cleanup (mothership → mother)
     Independent — can happen anytime
```

D1 and D4 can be worked in parallel. D3 should precede D2 so the context tool can reference the new scry interface shape in its recall directive.

---

## Exit Criteria

### Delivery (required for v0.11.0)

- [ ] **D1:** BeliefOracle wired into default query flow — see [[d1-belief-oracle/SPEC.md]]
- [ ] **D2:** Three-layer delivery operational — see [[d2-three-layer-delivery/SPEC.md]]
- [ ] **D3:** Two-step retrieval in both MCP and CLI — see [[d3-two-step-retrieval/SPEC.md]]
- [ ] **D4:** Routing simplified — `--routing` flag removed, graph routing is the sole cross-repo strategy
- [ ] **D5:** Naming cleanup applied — mothership → mother across codebase

### Federation (carried from Phase 1)

- [ ] Cross-project belief search works via graph routing — query in project A surfaces beliefs from project B via relationship edge
- [ ] Results tagged with provenance — every result shows `[project:channel]` origin
- [ ] `patina mother sync` populates all 19 registered repos as graph nodes

### Measurement

- [ ] Task-oriented A/B eval re-run with delivery changes (10 queries, same methodology as session 20260202-151214)
- [ ] Token efficiency measured: compare average tokens per scry response before/after D3
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

## See Also

- [[design.md]] — Ref repo evidence (OpenClaw, Gastown), all ADRs, resolved design questions
- [[d1-belief-oracle/SPEC.md]] — BeliefOracle design and implementation
- [[d2-three-layer-delivery/SPEC.md]] — Three-layer delivery design
- [[d3-two-step-retrieval/SPEC.md]] — Two-step retrieval design
- **Parent spec:** [[mother]] (`layer/surface/build/feat/mother/SPEC.md`)
- **Naming cleanup:** [[mother-naming]] (`layer/surface/build/refactor/mother-naming/SPEC.md`)
- **v1 release:** [[v1-release]] (`layer/surface/build/feat/v1-release/SPEC.md`)
- **A/B eval session:** [[20260202-151214]]
- **Design resolution session:** [[20260203-065424]]
