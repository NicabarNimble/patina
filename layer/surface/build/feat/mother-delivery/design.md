# Mother Delivery — Design Context

> Research evidence, architectural decisions, and resolved design questions.

---

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

**Implementation note (session 20260203-120615):** Channel B requires no new table — `belief_fts` already exists (`scrape/beliefs/mod.rs:103`) with columns `(id, statement, facets, content)` and porter tokenizer. Created during scrape, populated, but never queried at retrieval time. The BeliefOracle becomes its first consumer. See also ADR-6 for the over-fetch strategy needed by Channel A.

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

**Implementation note (session 20260203-120615):** This is a breaking change to the MCP response shape. The D3 spec adds a `--full` / `mode=full` escape hatch that preserves current full-content behavior during transition. The decision stands — snippets are the right default — but migration requires an opt-in path to the old behavior until adapters are confirmed working with snippets + detail.

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

**Implementation note (session 20260203-120615):** The three layers touch different parts of the codebase and can land independently. Layer 1 (tool descriptions) is a small self-contained change. Layer 2 (dynamic beliefs in context) requires wiring `get_project_context()` to SQLite + embeddings — it currently reads `layer/` files from disk only. Layer 3 (graph breadcrumbs) is shared with D3's snippet format. Implementation order: D3 ships snippets without breadcrumbs first, D2 Layer 3 adds the breadcrumb section to the established snippet skeleton.

**Anchored in:** [[adapter-pattern]] (adapter-agnostic delivery), OpenClaw belt-and-suspenders evidence, [[unix-philosophy]] (composable commands in dig-deeper).

### ADR-4: Why beliefs-as-channel, not intent classifier

The ref repo evidence is unanimous: no production system uses an intent classifier for belief routing. OpenClaw uses a static instruction. Gastown uses role-based templates. Both work. An intent classifier adds complexity (training data, false positives, latency) for a problem that simpler patterns solve.

If beliefs appear as a default search channel and the LLM sees them in results, the LLM handles intent matching naturally — it knows which beliefs are relevant to its current task better than any classifier we'd build.

### ADR-5: Why remove "All" routing

G0 measurement proved brute-force fails: 0% repo recall. The "All" strategy exists as a measured baseline and fallback. The measurement is complete — graph won definitively (100% recall). Keeping "All" adds complexity (3 strategies, --routing flag, user confusion) for a path that's proven inferior.

If a project has no graph edges, returning local-only results is better than searching 19 repos and drowning signal in noise. The user can add edges with `patina mother link` if they want cross-project results.

### ADR-6: Why over-fetch before dedicated index

**Decision:** BeliefOracle Channel A uses aggressive over-fetch (`min(limit * 50, total_index_size / 2)`) from the shared USearch index, with a dedicated belief USearch index as fallback if over-fetch proves unreliable.

**Context (session 20260203-120615):** With ~48 beliefs in an index of thousands of code entries, a standard `limit * 2` search (the SemanticOracle default) will likely return zero beliefs in the top results. The beliefs exist in the index but are statistically rare.

**Why over-fetch first, not dedicated index from the start:**
- Over-fetch is zero infrastructure cost — no new index to build, no new scrape step, no new file to manage. It's a parameter change in the oracle's `query()` method.
- USearch ANN search is sub-millisecond even at high limits (thousands of results). The post-filter to the 4B-5B range reduces to ~48 candidates. Performance cost is negligible.
- If over-fetch works, we avoid maintaining a second index that duplicates data already in the main index.

**When to fall back to dedicated index:**
- If empirical testing shows over-fetch at `limit * 50` still misses beliefs for common queries (ANN approximation might skip the belief region of the vector space entirely)
- The dedicated index would be tiny (~48 entries), fast to build during scrape, and eliminates the filtering problem entirely
- This is a measurable decision — run the A/B eval and check belief recall

**Anchored in:** [[dependable-rust]] (start simple, measure, escalate only if needed), pragmatism over premature optimization.

### ADR-7: Why unify CLI search path (D0)

**Decision:** Make QueryEngine (oracles + RRF fusion) the default for all CLI scry queries. Remove `--hybrid`, `--lexical`, and `--dimension` flags. One pipeline, two interfaces.

**Context (session 20260203-120615):** Grounding the delivery specs against the actual codebase revealed CLI and MCP scry are different pipelines. CLI default uses heuristic auto-detection between direct vector search and FTS5 — no oracles, no RRF. MCP always uses QueryEngine. This means D1 (BeliefOracle) would only work for MCP, D3 (snippets) would need two implementations, and the adapter pattern is violated.

**Why not keep the bifurcation and implement D1/D3 twice:**
- Duplicates oracle logic in the direct search path
- Maintains two pipelines that must produce consistent results
- Fights the architecture — the oracle system was designed to be the unified search interface
- None of the ref repos have dual search paths: OpenClaw has one pipeline (hybrid scoring), Gastown has one delivery path per role, OpenCode calls CLI via exec expecting consistent behavior

**Why the "experimental needs feedback" concern is resolved:**
`--hybrid` was introduced Dec 2025 as experimental, pending Phase 3 feedback loop. MCP was built later and used the oracle path from day one — it has been the default for every MCP query since January 2026. The oracle system is production-proven by MCP usage. The CLI default simply never completed the transition.

**What stays as-is:**
- `--belief` and `--file` modes are specialized entity-neighbor queries, not default search. They remain unchanged.
- `scry_text()` and `scry_lexical()` survive as oracle internals — SemanticOracle and LexicalOracle wrap equivalent logic.
- `--legacy` escape hatch provides the old direct-search behavior during transition.

**Anchored in:** [[adapter-pattern]] (same behavior regardless of interface), ref repo evidence (one search path), [[dependable-rust]] (completing the architecture, not building around it).

---

## Design History

**Session [[20260202-202802]]:** Initial design with 5 open questions.

**Session [[20260203-065424]]:** Resolved D1 (A+B oracle), D3 (two-step scope), and recall placement by reading ref repo code. See ADR-1, ADR-2, ADR-3.

**Session [[20260203-120615]]:** Grounded specs against actual codebase. Discovered `belief_fts` already exists, identified over-fetch risk, added migration strategy for breaking change, clarified D2/D3 dependency ordering. Discovered CLI/MCP bifurcation — CLI default bypasses oracle system entirely. Added D0 (unified search) as foundation change. See implementation notes on ADR-1/2/3, ADR-6, and ADR-7.

**Ref repo code reviewed:**
- OpenClaw: `memory-tool.ts` (tool definitions), `system-prompt.ts` (recall placement), `hybrid.ts` (scoring), `manager.ts` (search architecture)
- Gastown: `prime.go` (context injection), `mayor.md.tmpl` / `polecat.md.tmpl` (role templates)

---

## Related

- **Parent spec:** [[mother-delivery]] (`SPEC.md`)
- **A/B eval session:** [[20260202-151214]]
- **Design resolution session:** [[20260203-065424]]
- **Ref repos:** [[openclaw/openclaw]], [[steveyegge/gastown]]
