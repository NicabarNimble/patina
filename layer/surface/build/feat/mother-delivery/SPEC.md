---
type: feat
id: mother-delivery
status: design
created: 2026-02-02
updated: 2026-02-03
sessions:
  origin: 20260202-202802
  design-resolution: 20260203-065424
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

**Mandatory recall — belt and suspenders (verified in code):**
Recall instruction lives in TWO places simultaneously:
1. Tool description (`memory-tool.ts:39`): `"Mandatory recall step: semantically search MEMORY.md + memory/*.md ... before answering questions about prior work, decisions, dates, people, preferences, or todos"`
2. System prompt (`system-prompt.ts:48`): `"Before answering anything about prior work, decisions, dates, people, preferences, or todos: run memory_search ... If low confidence after search, say you checked."`

Both the tool description (seen by any LLM with the tool) and the system prompt (adapter-specific) carry the same instruction. The tool description is adapter-agnostic; the system prompt is not.

**Two-step access (`memory-tool.ts`):**
- `memory_search`: returns 700-char snippets (`SNIPPET_MAX_CHARS = 700`). Always snippets, never full content. Result shape: `{ path, startLine, endLine, score, snippet, source }`.
- `memory_get`: NOT "fetch search result" — it's a file line reader. Takes `{ path, from?, lines? }`, returns raw file content. The LLM decides what lines to pull after seeing snippets.
- The two-step is fundamental, not conditional — there is no "full content search" mode.

**Hybrid scoring (`hybrid.ts`):**
- Simple weighted sum: `score = 0.7 * vectorScore + 0.3 * textScore`
- `DEFAULT_HYBRID_CANDIDATE_MULTIPLIER = 4` (4x oversampling per channel, then merge down)
- NOT RRF — direct weighted combination. Simpler than our multi-oracle fusion.
- Unified index: one content type (markdown files), one search. No multi-oracle architecture needed because there's only one kind of content.

### Gastown ([[steveyegge/gastown]], 2,957 commits)

**Context recovery, not session start (verified: commit `e16d584`):**
`gt prime` is called via SessionStart hook (`"gt prime --hook"` in Claude Code settings) but its purpose is context recovery. It detects role from CWD path analysis, then renders a Go template (`*.md.tmpl`) with structural data.

**Role-based delivery (`prime.go:87-256`):**
The `runPrime()` function layers context in order:
1. Role detection from CWD (mayor/witness/polecat/crew/refinery/boot)
2. Render role-specific template (`mayor.md.tmpl` = ~300 lines of behavioral contract)
3. Check handoff marker (prevents handoff loop bug)
4. Check for slung work → autonomous mode (skip normal startup)
5. Output molecule context (workflow steps)
6. Output checkpoint for crash recovery
7. Run `bd prime` for beads workflow context
8. Run `gt mail check --inject` for pending mail
9. Output startup directive based on state

**State-aware delivery:** Same agent gets different context based on: fresh start vs crash recovery vs handoff, what role, what work is assigned. Delivery is **situational**, not static.

**No RAG at all:** 179,000 lines of Go with zero embeddings, zero vector search. All context is structural (role detection + templates + state). Proves that delivery architecture matters more than search quality.

**Delivery is adapter-coupled:** Output goes through Claude Code's `SessionStart` hook. The role templates contain behavioral directives ("DO NOT wait for confirmation", "if you stall, the whole town stalls") embedded in the template text. Other adapters would need equivalent hooks.

### What No Ref Repo Does

None solve automatic query-time belief routing. OpenClaw uses mandatory recall instructions. Gastown uses role-based injection. Both are adapter-specific (OpenClaw: OpenAI/Gemini system prompts, Gastown: Claude Code hooks). Neither federates across projects.

**Patina's opportunity:** Adapter-agnostic delivery through MCP tools and CLI, federated across the knowledge graph. Our adapters (Claude Code, OpenCode, Gemini CLI) consume both MCP and CLI — we steer them into patina's system with minimal adapter surface.

---

## Design

### Principle: Delivery Through MCP and CLI

The delivery mechanism is both MCP tools and CLI commands. Our adapters (Claude Code, OpenCode, Gemini CLI) consume both interfaces — Claude Code uses MCP tools directly and calls CLI via Bash; OpenCode uses CLI via exec. Both paths must deliver the same knowledge. No CLAUDE.md instructions, no adapter-specific hooks, no system prompt injection. Steer adapters into patina with minimal surface per adapter.

**Platform:** Mac-focused. Linux support for containerized agents (Docker). Zero Windows.

### D1: Beliefs as a Default Search Channel

**Current:** Beliefs appear only via `mode=belief` (explicit) or as post-processing annotations on code results (ignorable).

**Change:** A `BeliefOracle` runs on **every default query** as a parallel search channel alongside SemanticOracle, LexicalOracle, TemporalOracle, and PersonaOracle.

```
scry("how should I handle errors?")
  → SemanticOracle:  code results from function_facts
  → LexicalOracle:   FTS5 matches from code + commits
  → TemporalOracle:  co-change clusters
  → PersonaOracle:   cross-project user knowledge
  → BeliefOracle:    hybrid vector + FTS5 against beliefs  ← NEW
  → RRF merge all channels
  → Return with channel tags: [code] [commit] [belief]
```

**"Do X" test:** "Find beliefs relevant to this query." Clear, focused, one job.

#### BeliefOracle Design (A+B: Vector + FTS5)

The oracle implements the `Oracle` trait and internally runs two search channels:

```rust
// src/retrieval/oracles/belief.rs

pub struct BeliefOracle;

impl Oracle for BeliefOracle {
    fn name(&self) -> &'static str { "belief" }
    fn query(&self, query: &str, limit: usize) -> Result<Vec<OracleResult>>
    fn is_available(&self) -> bool  // checks beliefs table + index exist
}
```

**Channel A — Vector (semantic similarity):**
1. Embed query using E5-base-v2 pipeline (same as SemanticOracle)
2. Project to 256-dim (same projection)
3. Search USearch index with `limit * 2` (oversample to compensate for filtering)
4. Filter results to `BELIEF_ID_OFFSET` range (4B-5B)
5. Enrich from beliefs table: statement, entrenchment, evidence metrics

Reuses the existing semantic index — beliefs are already embedded there. Filtering happens post-search (USearch doesn't support range filters natively).

**Channel B — FTS5 (keyword):**
1. Tokenize query for FTS5
2. Search `beliefs_fts` table (NEW — created during scrape)
3. Return: belief_id, statement, BM25 score

New table schema (created during `patina scrape`):
```sql
CREATE VIRTUAL TABLE beliefs_fts USING fts5(
    statement,
    evidence_summary,
    tags,
    content='beliefs',
    content_rowid='rowid'
);
```

Captures keyword matches vector search misses. A query containing "thiserror" matches a belief about "use thiserror derive macros" via exact keyword, not just semantic proximity.

**Internal merge (weighted sum, one ranked list):**
```rust
const VECTOR_WEIGHT: f32 = 0.7;
const TEXT_WEIGHT: f32 = 0.3;

// Dedup by belief_id
// score = VECTOR_WEIGHT * cosine + TEXT_WEIGHT * bm25_normalized
// Sort descending, return as Vec<OracleResult>
// score_type = "hybrid_belief"
```

The oracle produces ONE ranked list. RRF treats it as one channel competing alongside code/commits/temporal. This is correct — we want beliefs to have guaranteed representation in the final results without dominating. The internal A+B is an implementation detail hidden behind the Oracle trait (dependable-rust: black-box module).

**Why not just improve tagging (Option C):** Beliefs already appear in SemanticOracle results when they score high enough. The problem is they're drowned by code (vastly more code than beliefs in the index). A separate oracle gives beliefs their own ranking list in RRF, guaranteeing they surface. The -0.05 task delta was caused by beliefs appearing as ignorable annotations, not as primary results.

**Why A+B, not A alone or B alone:** Mirrors the proven dual-channel pattern — SemanticOracle (vector) + LexicalOracle (FTS5) work in parallel for code. Same approach for beliefs. Vector catches semantic similarity ("how to structure modules" → belief about "dependable-rust black-box pattern"); FTS5 catches keywords ("thiserror" → belief mentioning "thiserror").

#### OracleResult Shape

```rust
OracleResult {
    doc_id: "belief:error-handling-thiserror",
    content: "Use thiserror derive macros for error types \
              (entrenchment: 0.8, evidence: 3/2, use: 5+2)",
    source: "belief",
    score: 0.83,
    score_type: "hybrid_belief",
    metadata: OracleMetadata {
        file_path: Some("layer/surface/beliefs/error-handling-thiserror.md"),
        ..
    },
}
```

#### Wiring into QueryEngine

```rust
// engine.rs
pub fn default_oracles() -> Vec<Box<dyn Oracle>> {
    vec![
        Box::new(SemanticOracle::new()),
        Box::new(LexicalOracle::new()),
        Box::new(TemporalOracle::new()),
        Box::new(PersonaOracle::new()),
        Box::new(BeliefOracle::new()),  // NEW
    ]
}
```

#### Intent Detection Update

```rust
// intent.rs — add belief weight
pub struct IntentWeights {
    semantic: f32,   // default 1.0
    lexical: f32,    // default 1.0
    temporal: f32,   // default 1.0
    persona: f32,    // default 1.0
    belief: f32,     // default 1.0  ← NEW
}

// Boosts:
// Rationale ("why", "decided"): belief 1.5, persona 1.5
// Definition ("what is", "explain"): belief 1.5, lexical 1.5
// Temporal ("when", "history"): no belief boost
// General: belief 1.0 (always participates)
```

#### Scrape Integration

During `patina scrape`, add FTS5 indexing for beliefs:
1. Beliefs are already inserted into the `beliefs` table
2. Beliefs are already embedded in USearch index at `BELIEF_ID_OFFSET + rowid`
3. NEW: Create and populate `beliefs_fts` table with statement + evidence_summary + tags

#### Module Layout

```
src/retrieval/oracles/
├── mod.rs          # pub use exports
├── semantic.rs     # existing
├── lexical.rs      # existing
├── temporal.rs     # existing
├── persona.rs      # existing
└── belief.rs       # NEW — BeliefOracle (A+B)
```

#### Cross-Project (Federation)

During graph routing, the BeliefOracle runs against each related project's belief table. A query in `cairo-game` that routes to `patina` via LEARNS_FROM also searches patina's beliefs. Reuses existing `collect_oracle_results_in_context()` pattern — create fresh oracles in target repo's context, collect results, tag with repo name.

#### Measurement

Re-run A/B eval from session `20260202-151214`:
- **Control:** Current system (beliefs as annotations only)
- **Treatment:** BeliefOracle wired into default query flow
- **Target:** Task-oriented delta ≥ 0.0. Stretch: ≥ +0.5

### D2: Three-Layer Delivery (Description → Response → Graph Breadcrumbs)

**Current:** `context` reads static markdown files. `scry` returns full content with belief annotations as ignorable metadata. No recall instruction anywhere.

**Change:** Three delivery layers, consistent across both MCP and CLI. Belt-and-suspenders — knowledge delivered at multiple touch points so the LLM encounters it regardless of which tool it calls first.

OpenClaw puts recall instructions in both tool description AND system prompt. Patina adapts this: both tool/command description AND tool/command response carry delivery, but without adapter-specific system prompts.

#### Layer 1: Tool/Command Description (the nudge)

The description text is the first thing the LLM sees — before it even calls the tool.

**MCP tool descriptions:**
```
scry: "Search codebase knowledge — USE THIS FIRST for any question about the code.
       Fast hybrid search over indexed symbols, functions, types, git history,
       and session learnings."

context: "Get project patterns and conventions — USE THIS to understand design rules
          before making architectural changes. Returns core patterns (eternal
          principles) and surface patterns (active architecture)."
```

**CLI `--help` text (consumed by LLM adapters calling via Bash/exec):**
Same content in `patina scry --help` and `patina context --help`. Claude Code reads `--help` output; OpenCode reads it via exec. The help text IS the tool description for CLI consumers.

#### Layer 2: Response-Level Recall (in the output)

**In `context` response (MCP and CLI):**
```
## Core Patterns
[existing — layer/core/ principles]

## Active Beliefs (top N by relevance to topic)
  B-12: "Error handling should use thiserror derive macros" (entrenchment: 0.8)
  B-07: "Prefer explicit Result<T,E> over panics" (entrenchment: 0.9)
  B-23: "MCP tools should be adapter-agnostic" (entrenchment: 0.7)

## Cross-Project Beliefs (from Mother graph)
[beliefs from related projects via graph traversal]

## Recall Directive
Before answering questions about project conventions, design decisions, or
architectural patterns: search for relevant beliefs.
  CLI:  patina scry --belief <id>
  MCP:  scry(content_type="beliefs")
Project knowledge accumulates in beliefs — check them before assuming defaults.
```

The recall directive appears in the **tool/command response**, not in any adapter file. Every LLM that calls `context` (via MCP or CLI) sees it. Includes both CLI and MCP syntax so the LLM uses whichever interface it's connected through.

**In `scry` response (when beliefs are present):**
```
--- Belief Impact ---
3 beliefs matched — dig deeper:
  CLI:  patina scry --belief <id> --content-type code
  MCP:  scry(belief="<id>", content_type="code")
```

Lightweight hint, not a full directive. Only appears when belief results are present.

#### Layer 3: Graph Breadcrumbs (link tracing in every result)

Every result is a node in the knowledge graph. The output shows its edges — what it links to, why, and how to follow. The LLM sees breadcrumbs and can self-direct exploration.

**Belief results include graph edges:**
```
2. [belief] error-handling-thiserror                       (0.83)
            "Use thiserror derive macros" (entrenchment: 0.8)
            Links:
              → attacks: panic-for-prototyping (defeated)
              → supports: explicit-result-types
              → reaches: src/retrieval/engine.rs, src/mcp/server.rs (+4 files)
            Dig deeper:
              patina scry --belief error-handling-thiserror --content-type code
```

**Code results include belief impact + structural edges:**
```
1. [code]   src/retrieval/engine.rs::query_with_options    (0.87)
            Belief impact:
              ← error-handling-thiserror (reach: 0.9)
              ← onnx-runtime-for-ml (reach: 0.7)
            Graph:
              → imports: src/retrieval/fusion.rs, src/retrieval/oracle.rs
              → co-changes: src/mcp/server.rs (28 times)
            Dig deeper:
              patina scry why src/retrieval/engine.rs::query_with_options
              patina assay --query-type callers --pattern query_with_options
```

The "Dig deeper" commands are CLI syntax (usable by Claude Code via Bash, OpenCode via exec). For MCP responses, the same section uses MCP tool syntax:
```
            Dig deeper:
              scry(mode="why", doc_id="src/retrieval/engine.rs::query_with_options")
              assay(query_type="callers", pattern="query_with_options")
```

The interface adapts to the delivery channel; the content is the same.

**Implementation:**
- Extend `get_project_context()` in `server.rs` to query beliefs by topic relevance
- Add cross-project belief aggregation via graph traversal
- Append recall directive to every context response (MCP and CLI)
- Add graph breadcrumbs to result formatting (belief links, belief impact, structural edges, dig-deeper commands)
- Detect delivery channel (MCP vs CLI) to format dig-deeper commands appropriately
- Rank beliefs by cosine similarity to topic when topic parameter is provided

### D3: Two-Step Retrieval (Search → Fetch) — Both MCP and CLI

**Current:** `scry` returns full content for every result. 10 results × full function bodies + annotations + impact analysis = heavy token load. Both MCP and CLI return the same verbose output.

**Change:** Default scry returns **snippets** in BOTH interfaces. Both MCP and CLI are LLM tools — Claude Code calls CLI via Bash, OpenCode calls via exec, Gemini CLI calls via MCP. All consumers benefit from token-efficient snippets with on-demand detail.

OpenClaw's `memory_search` always returns 700-char snippets — there is no "full content search" mode. Their two-step is fundamental, not conditional. We adopt the same principle: one behavior, two interfaces.

**Step 1 — Search (default, both MCP and CLI):**

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

Each result shows: doc_id, fused score, channel tag, oracle contributions, one-line summary (~120 chars), and graph breadcrumbs (D2 Layer 3).

**Step 2 — Fetch (on demand, both MCP and CLI):**

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

**Implementation:**
- The split happens after RRF fusion, before enrichment
- `enrich_results()` in `enrichment.rs` already reconstructs content from SQLite by ID ranges — this becomes the `detail` path
- `query_id` infrastructure already exists (Phase 3 feedback loop)
- Add `mode=detail` / `--detail` handler to both MCP server and CLI command
- Default mode returns: `doc_id, fused_score, sources[], one_line_summary, graph_breadcrumbs`
- `one_line_summary`: first line of content, truncated to ~120 chars
- Graph breadcrumbs: belief links, belief impact, co-change relationships, dig-deeper commands
- Dig-deeper commands formatted for the delivery channel (CLI syntax for CLI, MCP syntax for MCP)

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
- [ ] **D1: beliefs_fts table** — created during `patina scrape`, FTS5 over statement + evidence_summary + tags
- [ ] **D1: Intent boost** — Rationale and Definition intents boost belief oracle weight
- [ ] **D1 measured:** Re-run task-oriented A/B eval. Target: delta ≥ 0.0 (beliefs no longer hurt). Stretch: delta ≥ +0.5
- [ ] **D2: Three-layer delivery** — tool/command descriptions, response-level recall, and graph breadcrumbs all present
- [ ] **D2: Context returns dynamic beliefs** — `context(topic="error handling")` returns relevant beliefs ranked by topic similarity
- [ ] **D2: Recall directive in context response** — every context response includes recall instruction with both CLI and MCP syntax
- [ ] **D2: Graph breadcrumbs** — belief results show links (attacks/supports/reaches), code results show belief impact + structural edges
- [ ] **D2: Dig-deeper commands** — every result includes actionable commands to follow the graph, formatted for the delivery channel
- [ ] **D3: Snippets by default in BOTH MCP and CLI** — default mode returns doc_id + score + one-line summary + graph breadcrumbs, not full content
- [ ] **D3: --detail / mode=detail fetches single result** — `patina scry --detail <query_id> <rank>` and `scry(mode=detail, ...)` return full content + annotations for one result
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
- **Adapter-specific hooks** — no SessionStart hooks, no CLAUDE.md instructions. All delivery through MCP tools and CLI commands. We steer Claude Code, OpenCode, and Gemini CLI into patina with minimal adapter surface.
- **Windows support** — zero Windows/Microsoft. Mac-focused with Linux for containerized agents.

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

### ADR-1: Why A+B BeliefOracle, not Option C (better tagging)

**Decision:** Separate `BeliefOracle` with internal vector + FTS5, producing one ranked list via weighted sum (0.7 vector + 0.3 text).

**Context (session 20260203-065424):** Three options were considered:
- A: Separate oracle querying USearch index filtered to belief ID range (4B-5B)
- B: FTS5 full-text search against beliefs SQLite table
- C: Better tagging of beliefs that already come back from SemanticOracle

**Why A+B combined, not C:**
Beliefs already appear in SemanticOracle results — they're embedded in the same USearch index. The problem isn't that beliefs CAN'T be found; it's that they're drowned by code (vastly more code entries than belief entries). A separate oracle gives beliefs their own ranking list in RRF, guaranteeing representation in the final output.

Option C would improve tagging but not fix the drowning problem — beliefs would still compete against thousands of code results in the same ranked list.

**Why A+B combined, not A alone or B alone:**
Mirrors the proven dual-channel pattern already in production: SemanticOracle (vector) + LexicalOracle (FTS5) for code. Same principle applied to beliefs. Vector catches semantic matches; FTS5 catches keyword matches. Neither alone is sufficient.

**Anchored in:** [[dependable-rust]] (black-box module implementing Oracle trait), [[unix-philosophy]] (focused tool: "find beliefs relevant to this query"), OpenClaw hybrid pattern (0.7 * vector + 0.3 * text).

### ADR-2: Why two-step applies to BOTH MCP and CLI

**Decision:** Snippets by default in both MCP and CLI. `--detail` / `mode=detail` for full content in both.

**Context:** The CLI is not primarily a human tool. Our target adapters (Claude Code, OpenCode, Gemini CLI) consume both MCP and CLI:
- Claude Code: calls MCP tools directly AND calls `patina scry` via Bash
- OpenCode: calls `patina scry` via exec
- Gemini CLI: calls MCP tools

All three are LLM adapters. All benefit from token-efficient snippets with on-demand detail.

**Why not different defaults per interface:**
One behavior, two interfaces. The [[adapter-pattern]] says: same capability regardless of delivery channel. If snippets are right for MCP (token efficiency), they're right for CLI (same LLM consumer). Different defaults would mean the same LLM gets different information depending on which tool it happens to call — that's an adapter leak.

OpenClaw evidence: `memory_search` always returns 700-char snippets. There is no "full content search" mode. The two-step is fundamental.

**Anchored in:** [[adapter-pattern]] (same behavior regardless of interface), OpenClaw evidence (always snippets).

### ADR-3: Why three-layer delivery (belt and suspenders)

**Decision:** Recall and knowledge delivery through three layers:
1. Tool/command description (the nudge — seen before calling)
2. Response-level recall directive (seen when called)
3. Graph breadcrumbs with dig-deeper commands (actionable links in every result)

**Context:** OpenClaw puts recall in both tool description AND system prompt. Gastown puts behavioral directives in role templates. Both use belt-and-suspenders — multiple delivery points for the same instruction.

**Why three layers, not just one:**
LLMs are probabilistic. A single instruction may be ignored. Multiple reinforcing touchpoints increase compliance:
- Layer 1 (description): LLM sees it before deciding to call the tool
- Layer 2 (response): LLM sees it in the tool output, reinforces Layer 1
- Layer 3 (breadcrumbs): LLM sees actionable commands and can self-direct exploration — not just "you should search beliefs" but "here's the exact command to follow this link"

**Why graph breadcrumbs specifically:**
The "here is small info, if you want bigger info here you go" pattern. Every result is a node with visible edges. The LLM traces links through the knowledge graph, drilling deeper when needed. This turns search results into a navigable knowledge structure, not a flat list.

**Why NOT adapter-specific (no CLAUDE.md, no hooks):**
Both OpenClaw (system prompt) and Gastown (SessionStart hook) are adapter-coupled. Patina's adapters are Claude Code, OpenCode, and Gemini CLI. Any instruction in CLAUDE.md is invisible to OpenCode. Any hook is invisible to Gemini CLI. MCP tools + CLI commands are the shared interface. We minimize adapter surface and steer everything through patina.

**Anchored in:** [[adapter-pattern]] (adapter-agnostic delivery), OpenClaw belt-and-suspenders evidence, [[unix-philosophy]] (composable commands in dig-deeper).

### ADR-4: Why beliefs-as-channel, not intent classifier

The ref repo evidence is unanimous: no production system uses an intent classifier for belief routing. OpenClaw uses a static instruction. Gastown uses role-based templates. Both work. An intent classifier adds complexity (training data, false positives, latency) for a problem that simpler patterns solve.

If beliefs appear as a default search channel and the LLM sees them in results, the LLM handles intent matching naturally — it knows which beliefs are relevant to its current task better than any classifier we'd build.

### ADR-5: Why remove "All" routing

G0 measurement proved brute-force fails: 0% repo recall. The "All" strategy exists as a measured baseline and fallback. The measurement is complete — graph won definitively (100% recall). Keeping "All" adds complexity (3 strategies, --routing flag, user confusion) for a path that's proven inferior.

If a project has no graph edges, returning local-only results is better than searching 19 repos and drowning signal in noise. The user can add edges with `patina mother link` if they want cross-project results.

---

## Resolved Design Questions (session 20260203-065424)

Three open questions from session [[20260202-202802]] were resolved by reading ref repo code and anchoring in [[layer/core]] values:

1. **D1 BeliefOracle approach:** A+B combined (vector + FTS5). Mirrors SemanticOracle + LexicalOracle dual-channel pattern. See ADR-1.
2. **D3 Two-step scope:** Both MCP and CLI get snippets by default. CLI is an LLM tool (Claude Code via Bash, OpenCode via exec), not primarily human. See ADR-2.
3. **Recall directive placement:** Three-layer delivery (description + response + graph breadcrumbs). Belt-and-suspenders for both interfaces. See ADR-3.

**Ref repo code reviewed:**
- OpenClaw: `memory-tool.ts` (tool definitions), `system-prompt.ts` (recall placement), `hybrid.ts` (scoring), `manager.ts` (search architecture)
- Gastown: `prime.go` (context injection), `mayor.md.tmpl` / `polecat.md.tmpl` (role templates)

---

## Related

- **Parent spec:** [[mother]] (`layer/surface/build/feat/mother/SPEC.md`)
- **Naming cleanup:** [[mother-naming]] (`layer/surface/build/refactor/mother-naming/SPEC.md`)
- **Epistemic layer (complete):** [[epistemic-layer]] (`layer/surface/build/feat/epistemic-layer/SPEC.md`)
- **v1 release:** [[v1-release]] (`layer/surface/build/feat/v1-release/SPEC.md`)
- **Ref repos:** [[openclaw/openclaw]], [[steveyegge/gastown]]
- **A/B eval session:** [[20260202-151214]]
- **Design resolution session:** [[20260203-065424]]
