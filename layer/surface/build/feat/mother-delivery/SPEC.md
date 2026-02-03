---
type: feat
id: mother-delivery
status: design
created: 2026-02-02
updated: 2026-02-02
sessions:
  origin: 20260202-202802
related:
  - layer/surface/build/feat/mother/SPEC.md
  - layer/surface/build/feat/v1-release/SPEC.md
  - layer/surface/build/feat/epistemic-layer/SPEC.md
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

## Evidence: Ref Repo Research

Three reference architectures informed this design:

### OpenClaw ([[openclaw/openclaw]], 8,308 commits)

**Mandatory recall pattern:** System prompt instructs the agent: "Before answering anything about prior work, decisions, preferences — run memory_search first." The LLM follows the instruction. No intent classifier needed.

**Two-step access:** `memory_search` returns 700-char snippets. `memory_get` fetches full content. The agent decides what's worth the tokens. Context-efficient.

**Candidate oversampling:** Loads 4x candidates per channel, merges down. Score = `0.7 * vector + 0.3 * text`. Simple, tunable.

### Gastown ([[steveyegge/gastown]], 2,957 commits)

**Ephemeral injection:** `gt prime` runs as a SessionStart hook. Generates fresh, role-specific context at every session start — not stored in files, regenerated each time.

**State-aware delivery:** Same agent gets different context based on: fresh start vs crash recovery vs handoff, what role (mayor/polecat/witness), what work is assigned. Delivery is **situational**, not static.

**No RAG at all:** 179,000 lines of Go with zero embeddings, zero vector search. All context is structural (role detection + templates + state). Proves that delivery architecture matters more than search quality.

### What No Ref Repo Does

None solve automatic query-time belief routing. OpenClaw uses mandatory recall instructions. Gastown uses role-based injection. Both are adapter-specific (OpenClaw: OpenAI/Gemini system prompts, Gastown: Claude Code hooks). Neither federates across projects.

**Patina's opportunity:** Adapter-agnostic delivery through MCP tool design, federated across the knowledge graph.

---

## Design

### Principle: Delivery Through MCP, Not Adapter Files

The delivery mechanism is the MCP tool interface itself — tool descriptions, tool responses, and tool behavior. Every adapter that connects via MCP gets delivery for free. No CLAUDE.md instructions, no adapter-specific hooks, no system prompt injection.

### D1: Beliefs as a Default Search Channel

**Current:** Beliefs appear only via `mode=belief` (explicit) or as post-processing annotations on code results (ignorable).

**Change:** A BeliefOracle runs on **every default query** as a parallel search channel alongside SemanticOracle, LexicalOracle, and TemporalOracle.

```
scry("how should I handle errors?")
  → SemanticOracle:  code results from function_facts
  → LexicalOracle:   FTS5 matches from code + commits
  → TemporalOracle:  co-change clusters
  → BeliefOracle:    semantic search against beliefs table  ← NEW
  → RRF merge all channels
  → Return with channel tags: [code] [commit] [belief]
```

The belief channel uses the same embedding pipeline as code — beliefs are already embedded in the semantic index (ID range 4B-5B). The change is wiring them into the default query flow, not a separate mode.

**Cross-project extension:** During graph routing, the BeliefOracle runs against each related project's belief table. A query in `cairo-game` that routes to `patina` via LEARNS_FROM also searches patina's beliefs.

**Implementation:**
- Add `BeliefOracle` to `src/retrieval/` (parallel to SemanticOracle, LexicalOracle)
- Wire into `QueryEngine::query_with_options()` default oracle set
- Extend graph routing in `routing.rs` to include belief search per related repo
- Tag results with `[belief]` provenance in MCP output

### D2: Context as Session Briefing

**Current:** `context` reads `layer/core/` and `layer/surface/` markdown files. Returns static patterns. LLM must decide to call it.

**Change:** `context` becomes a dynamic briefing tool. The MCP tool description directs the LLM to call it, and the response includes actionable directives.

**Tool description (adapter-agnostic, lives in MCP schema):**
```
"Get project patterns and conventions — USE THIS to understand design rules
before making architectural changes. Returns core patterns (eternal principles)
and surface patterns (active architecture)."
```

**Response structure:**
```
## Core Patterns
[existing — layer/core/ principles]

## Active Beliefs (top N by relevance)
[NEW — query beliefs table, ranked by use_count + relevance to topic]

  B-12: "Error handling should use thiserror derive macros" (entrenchment: 0.8)
  B-07: "Prefer explicit Result<T,E> over panics" (entrenchment: 0.9)
  B-23: "MCP tools should be adapter-agnostic" (entrenchment: 0.7)

## Cross-Project Beliefs (from Mother graph)
[NEW — beliefs from related projects via graph traversal]

## Recall Directive
Before answering questions about project conventions, design decisions, or
architectural patterns: search for relevant beliefs using scry with
content_type="beliefs". Project knowledge accumulates in beliefs —
check them before assuming defaults.
```

The recall directive is in the **tool response**, not in any adapter file. Every LLM that calls `context` sees it. This is the OpenClaw "mandatory recall" pattern made adapter-agnostic.

**Implementation:**
- Extend `get_project_context()` in `server.rs` to query beliefs by topic relevance
- Add cross-project belief aggregation via graph traversal
- Append recall directive to every context response
- Optional: rank beliefs by cosine similarity to topic, not just use_count

### D3: Two-Step Retrieval (Search → Fetch)

**Current:** `scry` returns full content for every result. 10 results × full function bodies + annotations + impact analysis = heavy token load.

**Change:** Default scry returns **snippets** (doc_id, score, channel tag, one-line summary). A new `mode=detail` fetches full content for a specific result.

**Step 1 — Search (default):**
```
scry("vector similarity search") →

  query_id: q_20260202_143000_abc

  1. [code]   src/retrieval/engine.rs::query_with_options  (0.87)
              SemanticOracle(0.92) + LexicalOracle(0.71)
  2. [belief] B-15: "Use USearch for vector indices"       (0.83)
              BeliefOracle(0.83)
  3. [commit] abc1234: "feat: add cosine distance metric"  (0.79)
              TemporalOracle(0.85) + LexicalOracle(0.68)
  4. [code:USearch] src/index.hpp::search                  (0.76)
              SemanticOracle(0.76) — via LEARNS_FROM
```

**Step 2 — Fetch (on demand):**
```
scry(mode=detail, query_id=q_20260202_143000_abc, rank=1) →

  src/retrieval/engine.rs::query_with_options
  [full function signature, body, structural annotations]

  Belief impact:
    B-15: "Use USearch for vector indices" (reach: 0.9)
    B-03: "ONNX Runtime for all ML inference" (reach: 0.7)
```

The LLM sees the landscape first, then drills into what matters. This is OpenClaw's `memory_search` → `memory_get` pattern.

**Implementation:**
- The split happens after RRF fusion, before enrichment
- `enrich_results()` in `enrichment.rs` already reconstructs content from SQLite by ID ranges — this becomes the `detail` path
- `query_id` infrastructure already exists (Phase 3 feedback loop)
- Add `mode=detail` handler to MCP server alongside existing `mode=why` and `mode=use`
- Default mode returns: `doc_id, fused_score, sources[], one_line_summary`
- `one_line_summary`: first line of content, truncated to ~120 chars

### D4: Simplify Routing to Graph-Only

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

### D5: Mother Naming Cleanup

**Dependency:** Spec already written at `layer/surface/build/refactor/mother-naming/SPEC.md`.

Apply the existing naming cleanup (mothership → mother) as part of this milestone. ~150 line changes across env vars, function names, struct fields, user-facing output.

---

## Exit Criteria

### Delivery (required for v0.11.0)

- [ ] **D1: BeliefOracle wired into default query flow** — beliefs appear in standard scry results without `mode=belief` or `--belief` flags
- [ ] **D1 measured:** Re-run task-oriented A/B eval. Target: delta ≥ 0.0 (beliefs no longer hurt). Stretch: delta ≥ +0.5
- [ ] **D2: Context returns dynamic beliefs** — `context(topic="error handling")` returns relevant beliefs ranked by topic similarity, not just use_count
- [ ] **D2: Recall directive in context response** — every context response includes the recall instruction
- [ ] **D3: Scry returns snippets by default** — default mode returns doc_id + score + one-line summary, not full content
- [ ] **D3: mode=detail fetches single result** — `scry(mode=detail, query_id, rank)` returns full content + annotations for one result
- [ ] **D4: Routing simplified** — `--routing` flag removed, graph routing is the sole cross-repo strategy
- [ ] **D5: Naming cleanup applied** — mothership → mother across codebase

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
- **Mother semantic tier** (beliefs.usearch, patterns.usearch at ~/.patina/mother/) — deferred. Local project belief search + graph routing is sufficient for v0.11.0. The centralized semantic tier is a Phase 3+ concern.
- **Automatic edge creation (G3)** — still deferred. Manual edges + weight learning is sufficient for 19 repos.
- **Context pressure distillation** — interesting (from OpenClaw), but a prompt engineering concern for session skills, not a code change for this milestone.
- **Adapter-specific hooks** — no SessionStart hooks, no CLAUDE.md instructions. All delivery through MCP tools.

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

## Architectural Decisions

### Why beliefs-as-channel, not intent classifier

The ref repo evidence is unanimous: no production system uses an intent classifier for belief routing. OpenClaw uses a static instruction. Gastown uses role-based templates. Both work. An intent classifier adds complexity (training data, false positives, latency) for a problem that simpler patterns solve.

If beliefs appear as a default search channel and the LLM sees them in results, the LLM handles intent matching naturally — it knows which beliefs are relevant to its current task better than any classifier we'd build.

### Why two-step retrieval, not just truncation

Truncating results loses information. Two-step retrieval preserves all information but lets the LLM control what it consumes. The LLM sees the full landscape (all channels, all scores) and decides what to drill into. This matches how developers use search: scan results, click into the interesting ones.

The query_id + rank infrastructure for this already exists in the Phase 3 feedback loop. This is reusing existing plumbing.

### Why delivery through MCP, not adapter files

Patina supports Claude and Gemini adapters. Any delivery mechanism in CLAUDE.md is invisible to Gemini. Any mechanism in Gemini's config is invisible to Claude. MCP is the shared interface — tool descriptions and tool responses are the only adapter-agnostic delivery channel.

The recall directive in the `context` tool response is seen by every LLM that calls the tool, regardless of adapter. This is the Gastown `gt prime` pattern (ephemeral injection) adapted for MCP: knowledge injected at tool-call time, not at session start.

### Why remove "All" routing

G0 measurement proved brute-force fails: 0% repo recall. The "All" strategy exists as a measured baseline and fallback. The measurement is complete — graph won definitively (100% recall). Keeping "All" adds complexity (3 strategies, --routing flag, user confusion) for a path that's proven inferior.

If a project has no graph edges, returning local-only results is better than searching 19 repos and drowning signal in noise. The user can add edges with `patina mother link` if they want cross-project results.

---

## Related

- **Parent spec:** [[mother]] (`layer/surface/build/feat/mother/SPEC.md`)
- **Naming cleanup:** [[mother-naming]] (`layer/surface/build/refactor/mother-naming/SPEC.md`)
- **Epistemic layer (complete):** [[epistemic-layer]] (`layer/surface/build/feat/epistemic-layer/SPEC.md`)
- **v1 release:** [[v1-release]] (`layer/surface/build/feat/v1-release/SPEC.md`)
- **Ref repos:** [[openclaw/openclaw]], [[steveyegge/gastown]]
- **A/B eval session:** [[20260202-151214]]
