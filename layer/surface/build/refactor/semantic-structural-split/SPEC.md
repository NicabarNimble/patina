---
type: refactor
id: semantic-structural-split
status: active
created: 2026-02-08
sessions:
  origin: 20260208-070221
related:
- layer/surface/build/feat/mother-v2/SPEC.md
- layer/surface/build/fix/eval-phase4-divergences/SPEC.md
beliefs:
- dependable-rust
- unix-philosophy
- andrew-ng-over-shoulder
- never-tune-on-eval
- error-analysis-over-architecture
---

# refactor: Semantic-Structural Split

> Separate scry (semantic/meaning) from assay (structural/factual) along their
> intended design boundary. Scry became a monolith holding 5 oracles, 3 FTS5
> tables, temporal queries, belief lookup, and vector search. This refactor
> restores the original intent: scry for meaning, assay for facts.

## Problem

### The Mixing Problem

Scry currently owns five oracles that mix two fundamentally different query types:

| Oracle | Query Type | Data Source | Belongs In |
|--------|-----------|-------------|------------|
| Semantic | **Meaning** — vector similarity | USearch index | scry |
| Lexical | **Factual** — BM25 text match | 3 FTS5 tables | assay |
| Temporal | **Factual** — co-change frequency | co_changes table | assay |
| Persona | **Factual** — developer practices | persona files | assay |
| Belief | **Factual** — belief relationships | beliefs table | assay |

All five produce ranked lists that get RRF-fused in one step. This coupling
caused compounding problems across 6 sessions of retrieval tuning:

1. **BM25 scale mismatch** (root cause 2): Three FTS5 tables with different
   column counts produce incomparable BM25 scores. Required min-max
   normalization as a patch. In assay, tables can be queried independently
   or normalized within a single factual layer — no cross-type comparison.

2. **P@5 outer fusion dilution**: Non-semantic oracles (temporal, belief)
   insert low-quality results into top-5 via RRF, pushing correct lexical
   hits to positions 6-10. A fusion problem created by mixing query types.

3. **25-parameter tuning trap**: 5 oracles x 5 intents = 25 weights. Most
   complexity exists because one fusion step balances meaning and facts.
   Splitting reduces this to combining two signals: structural + semantic.

4. **Semantic index pollution**: `query_session_events()` in oxidize/mod.rs
   embeds 6 source types into one vector space (~27K items, 88% session
   events). The semantic projection trained on session co-occurrence maps
   everything to session-space, not meaning-space. Semantic oracle returns
   0% useful results for NL queries.

5. **Non-isolable debugging**: Every fix exposed the next layer of mixing
   complexity. Fixing scraper bugs shifted BM25 distributions. Fixing BM25
   with inner RRF created RRF-of-RRF. Each problem was local but effects
   cascaded through the shared fusion pipeline.

### What We Learned (6 Sessions of Evidence)

The retrieval-tuning spec documents the full history. Key outcomes:

- Phase 2.5 shipped with 3/4 targets met (P@10 40.7%, test P@10 44.2%, MRR 0.433)
- P@5 miss (26% vs 28% target) proven to be outer fusion dilution, not normalization
- Min-max normalization is a correct patch but exists because of the mixing problem
- Every measurement fix revealed another mixing consequence
- The eval measured end-to-end but tuning happened at component level — mismatch

### Design Anchors (layer/core)

**[[dependable-rust]]**: Each command is a black-box module. Scry's public
interface (`scry()`, `scry_text()`, `scry_lexical()`) currently exposes both
semantic and structural concerns. After split, scry's interface is purely
semantic. Assay's interface is purely structural/factual. Internal changes
to either don't cascade.

**[[unix-philosophy]]**: One tool, one job. Scry's job: "find what's
conceptually related." Assay's job: "find what's factually relevant."
These are different questions with different algorithms, different data
structures, and different failure modes. Mixing them violates the principle.

**[[adapter-pattern]]**: The Oracle trait is an adapter interface. After
split, scry has semantic oracles (vector similarity, conceptual matching).
Assay has factual oracles (FTS5, temporal, relational). Each set can evolve
independently.

## Design

### Principle: One Cut, Clean Boundary

The split follows a single principle: **meaning vs. facts**.

- **Scry** answers: "What is conceptually related to this query?"
  Algorithm: vector similarity across one or more semantic domains.

- **Assay** answers: "What facts are relevant to this query?"
  Algorithm: FTS5 text search, temporal co-change, belief relationships,
  call graphs, import relationships.

Fusion of meaning + facts happens at the consumer level (the command or
tool that calls both), not inside either system.

### Phase 1: Move Factual Oracles to Assay

Move lexical, temporal, persona, and belief oracles from scry to assay.
Assay gains a new query mode: ranked factual search (alongside its
existing exact structural queries).

**Assay before:**
```
assay inventory    — module metadata
assay imports      — import relationships
assay importers    — importer relationships
assay functions    — function listing
assay callers      — call graph (callers)
assay callees      — call graph (callees)
assay derive       — structural signals
assay derive-moments — temporal signals
```

**Assay after (additive):**
```
assay search <query>  — ranked factual search (FTS5 + temporal + belief)
assay belief <id>     — belief grounding (evidence for/against)
assay cochange <file> — co-change analysis
```

The existing exact-query subcommands don't change. New ranked-search
capability is additive.

**Scry after (reductive):**
```
scry <query>        — semantic vector search (core mode)
scry orient <path>  — structural ranking for a directory
scry recent [days]  — temporal ranking of recent changes
scry --belief <id>  — belief grounding (vector neighbors)
scry --detail/--why — result inspection convenience modes
```

Scry's `QueryEngine` reduces from 5 oracles to semantic only. The
`src/retrieval/` module simplifies significantly. The convenience modes
(orient, recent, why, detail, use) query structural data for context
but don't participate in fusion — they're separate access patterns.
The split principle is about *fusion*, not *CLI surface area*.

**What moves:**
```
src/retrieval/oracles/lexical.rs   → src/commands/assay/internal/
src/retrieval/oracles/temporal.rs  → src/commands/assay/internal/
src/retrieval/oracles/persona.rs   → src/commands/assay/internal/
src/retrieval/oracles/belief.rs    → src/commands/assay/internal/
src/commands/scry/internal/search.rs (scry_lexical, normalize_table) → assay
```

**What stays in scry:**
```
src/retrieval/oracles/semantic.rs  — vector similarity oracle
src/retrieval/engine.rs            — simplified, semantic-only
src/commands/scry/internal/search.rs (scry_text, scry) — semantic search
```

**What gets removed:**
```
src/retrieval/intent.rs            — intent weights tuned cross-oracle balance
```

**What gets repurposed (not removed):**
```
src/retrieval/fusion.rs            — RRF algorithm removed; result type definitions
                                     (FusedResult, OracleContribution, StructuralAnnotations)
                                     retained for QueryEngine, MCP, and eval
src/commands/scry/internal/hybrid.rs — renamed to semantic.rs; thin wrapper around
                                       QueryEngine (no longer a 5-oracle orchestrator)
```

### Phase 2: First Semantic Domain — Knowledge

The current semantic index is broken: 27K items, 88% session events,
trained on session co-occurrence. Before adding domains, build ONE that
works and prove it adds value over assay's factual search alone.

**Domain: knowledge** (beliefs + patterns + commits)

Why this domain first:
- Beliefs and patterns are natural language — semantic matching adds value
  over keyword search (FTS5 can find "dependable-rust" but not "how should
  I structure my modules?")
- Commit messages capture the "why" behind changes — conceptual queries
  like "what changed about error handling?" benefit from meaning, not keywords
- ~2K items total — small, clean, fast to iterate on
- Directly measurable: does scry find the right belief when you ask a
  conceptual question that assay can't answer with keywords?

**What gets embedded:**
- Beliefs (~74 items) — natural language decisions
- Patterns (~80 items) — natural language documents about principles
- Commit messages (~1800 items) — natural language "why" behind changes

**What does NOT get embedded (yet):**
- Session events — deferred, not deleted. Sessions capture development
  intent ("what was the user thinking about?") which has real semantic
  value. But sessions are the largest corpus (88% of current index) and
  need their own training signal. A session domain must earn its place
  by proving retrieval value — see Phase 5.
- Code function signatures — structural data, assay handles this
- Forge events — deferred until a domain proves valuable

**oxidize changes:**
- `query_session_events()` refactored to `query_knowledge_corpus()`
- Builds a single `knowledge.usearch` index
- Training pairs: belief↔pattern co-reference, commit↔code co-change
  (start with commit-based pairs as baseline, iterate on training signal)

**Validation:** Run scry with knowledge domain against a small query set
(~20 conceptual questions). Compare scry results vs assay-only results.
If scry finds correct answers that assay misses → domain proven. If
scry adds nothing over assay → investigate training signal before adding
more domains.

### Phase 3: Consumer-Level Fusion

With scry and assay as independent tools, the consumer decides how to
combine their answers. Two consumers matter:

**`patina context`** (CLAUDE.md generation):
- Calls assay for factual context (what files, what beliefs, what changed)
- Calls scry for semantic context (what's conceptually relevant)
- Merges results with simple priority (facts first, then meaning for gaps)

**MCP tool** (live session queries):
- Currently exposes `scry` tool. After split, expose both:
  - `scry` — "what's related to this concept?"
  - `assay search` — "what facts match this query?"
- Or expose a combined `query` tool that calls both and merges
- Fusion is simple: two ranked lists, interleave or weighted merge
- No 25-parameter tuning — just "how much meaning vs. facts?"

### Phase 4: Eval Redesign

With the split, eval becomes independently testable. Three divergences from
the original plan below were authorized by [[eval-phase4-divergences]].

**Assay eval:**
- "Given this query, does assay find the right files?" (FTS5 ranked search)
- New 25-query set (`resources/eval/assay-queries.json`) designed for
  post-split architecture. Original 52-query set had 8 stale paths from
  deleted oracle files; fresh queries per [[never-tune-on-eval]].
- Co-change temporal analysis NOT tested here — different query type
  (file-path input, not NL). Separate eval if needed per [[unix-philosophy]].

**Scry eval:**
- "Given this concept, does scry find related items?" (vector similarity)
- New 20-query set (`resources/eval/scry-queries.json`) with intentional
  vocabulary gaps testing semantic bridging:
  - "ripple effects from changes" should find [[dependable-rust]]
  - "baselines before optimization" should find [[andrew-ng-over-shoulder]]
- Includes scry-vs-assay comparison (exit criterion: ≥5/20 scry-only hits)

**Combined eval:**
- Runs both query sets through both systems with facts-first interleaving
- Measures cross-system contribution: what each system adds to the other
- Pipeline-level P@K, hit rate, and per-query-type delta from single-system
- Richer "development context → right context" product metric is Phase 5

### Phase 5: Actively Discover Semantic Domains

The knowledge domain is the foundation, not the ceiling. Patina's value
scales with every semantic dimension that helps the user get better
context. Phase 5 is an ongoing effort to discover, test, and ship
domains that add retrieval value. Each domain must prove it helps through
measurement — but the posture is exploration, not gatekeeping.

#### Phase 5a: Knowledge Domain Corpus Optimization

Before adding new domains, fix the existing knowledge domain's corpus
composition. Phase 4 diagnostics reveal the root cause of the 4/20
scry-vs-assay gap: commit dominance (1,824 commits = 92% of index vs
77 beliefs + 79 patterns = 8%). Conceptual queries hit the dense commit
cluster instead of the sparse high-value beliefs/patterns.

**Diagnostic evidence (Phase 4 session 20260208-171005):**
- All 4 scry-only hits are beliefs where query vocabulary overlaps belief text
- 7 of 9 both-miss queries expect core pattern docs — commits crowd them out
- 1 both-miss is a ground-truth issue: query about "AI assistants understanding
  code" returns `llm-readable-code` belief (rank 1, score 0.850) — genuinely
  correct answer, but not in the expected set
- Score compression: all results in 0.79-0.87 range, no differentiation

**Three-part fix (per [[corpus-composition-over-model]]):**

1. **Fix eval ground truth** — add `llm-readable-code` to query 6's expected
   set ("making code easy for AI assistants to understand and modify"). This
   belief literally says "Code should be self-documenting for AI readers."
   Honest measurement per [[andrew-ng-over-shoulder]], not metric gaming.

2. **Enrich belief/pattern embedding text** — give the embedder more semantic
   signal per item. Beliefs currently embed ~100 chars ("Belief: {id} -
   {statement}. Persona: ..."). Add evidence text, related belief references,
   and contextual vocabulary. For patterns, remove 500-char content truncation.
   Richer embeddings create more differentiated vectors that bridge wider
   vocabulary gaps.

3. **Filter commit corpus to significant subset** — reduce from 1,824 to
   ~300-400 commits. The 31-50 char bucket (572 commits) contains low-signal
   messages like "fix: typo" and "docs: update". Keep commits that:
   - Reference belief/pattern IDs in the message
   - Have messages above median length (>56 chars)
   - Mark releases, sessions, or significant features
   - Touch >3 files (structural significance)

   This shifts the ratio from 92% commits to ~50-60%, letting beliefs/patterns
   occupy proportional vector space.

**Process:** Fix ground truth → enrich text → filter commits → re-train
projection → rebuild index → measure (`patina eval --scry`). Target: ≥5/20
scry-only hits (Phase 4 exit criterion). If met → move to Phase 5b domain
discovery. If not → investigate training signal or model before adding domains.

#### Phase 5b: Session-Semantic Domain

**Hypothesis:** Semantic search over session events (decisions, patterns,
context, work) finds development reasoning and rationale that no current
system can surface. Sessions are invisible today — no eventlog_fts table
exists, so assay can't keyword-search session content, and scry's knowledge
domain doesn't include sessions. This is a net-new retrieval capability.

**Corpus analysis:**
- 38,181 total session events, but ~8x duplicated (each scrape re-inserts)
- ~2,744 unique events with >50 char content after dedup
- High-value types: decision (896), pattern (1,072), work (593), context (587)
- Content is natural language about WHY decisions were made — different
  vocabulary than WHAT was decided

**Validation note:** Standard scry-vs-assay comparison is not meaningful here
because assay has 0% coverage of session content (no eventlog_fts). Instead:
1. Measure scry hit rate on session queries (any hit = value, since nothing
   searches sessions today)
2. Additionally simulate what FTS5 WOULD find on the same content to assess
   whether semantic adds value over hypothetical keyword search
3. If scry-only hit rate is high AND semantic finds things keywords miss →
   strong evidence for the domain

**Infrastructure requirements:**
- Corpus builder: `query_session_corpus()` in oxidize/mod.rs (deduped)
- New `sessions` projection in oxidize.yaml
- Multi-domain SemanticOracle: load knowledge + sessions indices
- Semantic RRF fusion across domains in QueryEngine
- Session event enrichment in enrichment.rs (already partially exists)

**Model:** E5-base-v2 (same as knowledge domain, test first before trying
dialogue-tuned models — per [[corpus-composition-over-model]], corpus
matters more than model choice)

**Candidate: code-semantic**
- Content: function signatures, code documentation, module descriptions
- Training signal: call graph adjacency, co-change frequency
- Value hypothesis: "how does patina handle X?" finds related code across
  modules even when naming conventions differ
- Validation: compare code-semantic vs assay FTS5 on code queries.
  Code has strong keyword signal (function names are descriptive in Rust) —
  semantic may add little over good FTS5. Must prove otherwise.
- Model: may benefit from a code-specific model (CodeBERT, StarCoder
  embeddings) — test, don't assume

**Domain discovery process (fast iteration, not bureaucracy):**
1. Notice a gap: "the system missed X because keywords weren't enough"
2. Hypothesize: "semantic search over [corpus] finds answers that
   assay FTS5 misses for [query type]"
3. Build a small eval set (~15-20 queries) with expected results
4. Run assay-only baseline, measure P@K
5. Build the domain, run scry, measure P@K
6. If scry adds measurable value → ship it, keep iterating on quality
7. If scry adds nothing → investigate training signal or model choice
   before killing — the hypothesis may be right but the implementation wrong
8. Each domain gets its own model choice — test which model works best
   for that content type, don't default to one-size-fits-all

**Where to look for new domains:**
- Session feedback: when the user says "why didn't you find X?" — that's
  a domain signal. What kind of content was X? What meaning did the query
  carry that keywords missed?
- Error analysis: every eval run that shows assay finding results scry
  misses (or vice versa) reveals what each layer is good at
- Cross-project patterns: when mother connects projects, what semantic
  dimensions carry across? Those are high-value domains.
- New content types: as patina indexes more (forge events, CI logs,
  documentation sites), each content type is a candidate domain

**Multi-model support in oxidize:**
- `oxidize.yaml` already supports per-projection config
- Extend to allow per-domain model specification:
  ```yaml
  domains:
    knowledge:
      model: e5-base-v2
      corpus: [beliefs, patterns, commits]
      training: belief-pattern-coref
    sessions:          # earned after Phase 5 validation
      model: bge-base  # or same model — test decides
      corpus: [session_events]
      training: session-cooccurrence
  ```
- Infrastructure supports multiple models; domains activate only when earned

**Scry with multiple earned domains:**
When scry has multiple validated domains, fusion within scry is semantic
RRF — combining different views of meaning. This is fundamentally different
from today's mixed fusion (meaning + facts). RRF across semantic domains
makes sense: each domain answers "what's conceptually related?" from a
different perspective. The question being fused is the same type.

### Future: Mother's Semantic Layer

Once per-project scry works with multi-domain semantics, mother can build
a cross-project concept layer on top:

- Mother already has the graph infrastructure (USES, LEARNS_FROM, SIBLING)
- Today those edges are manually declared
- With per-project semantic results, mother could discover relationships:
  "Project A's [[dependable-rust]] is semantically close to Project B's
  'encapsulation' pattern — shared architectural philosophy"
- Mother operates at concept-level, not document-level — she understands
  how projects relate through their beliefs and patterns
- This is NOT designed here — it follows naturally from the split once
  per-project semantic domains are validated

This is what gives patina unique value: not just "find similar text" but
"find related ideas across projects, grounded in beliefs, validated by
evidence." No basic RAG system does this.

### The Value Imperative

Patina's competitive advantage IS the semantic layer. Assay (factual
search) is table stakes — any FTS5 wrapper can do keyword search. What
makes patina worth using is that scry understands meaning in ways that
keyword search cannot: finding beliefs that apply to unfamiliar code,
surfacing design rationale when naming conventions differ, connecting
decisions across sessions that happened months apart.

**The goal is to discover as many value-adding semantic domains as
possible**, not to minimize domains. Each domain that proves it helps
the user retrieve better context is a compounding advantage. The
measurement discipline ([[andrew-ng-over-shoulder]]) exists to ensure
we're adding real value, not to slow us down — it's a quality gate, not
a speed limit. We should be actively exploring what dimensions of meaning
help retrieval, proposing new domain candidates whenever we notice the
system missing something, and investing in the domains that prove out.

A single-domain scry is a stepping stone, not the destination.

## Exit Criteria

### Phase 1: Move Factual Oracles — COMPLETE (2026-02-08)
- [x] Lexical, temporal, persona, belief oracles moved to assay
- [x] `assay search <query>` returns ranked FTS5 + temporal + belief results
- [x] `assay belief <id>` returns evidence grounding for a belief
- [x] `assay cochange <file>` returns co-change analysis
- [x] Scry reduced to semantic oracle only (vector search)
- [x] `src/retrieval/` simplified: no RRF fusion, no intent weights
- [x] All existing tests pass (`cargo test --workspace`) — 328 tests
- [x] `cargo clippy --workspace` clean
- [x] MCP server wired: assay tool routes search/cochange/belief query types
- [x] Unused oracle files deleted (lexical.rs, temporal.rs, persona.rs, belief.rs, intent.rs)
- [x] Eval baseline documented (see below)

**Post-split eval baseline (semantic-only, `patina eval --nl`):**
```
Mean P@5:  0.0%    Mean P@10: 0.0%    MRR: 0.000
```
All zeros expected. The NL eval was designed for multi-oracle ablation.
Semantic oracle returns embedding results (sessions, commits, patterns)
that don't resolve to file paths the test set expects. The eval framework
itself needs redesigning (Phase 4) to separately test scry semantic
retrieval and assay factual retrieval. The 0% number is honest — the
semantic index is polluted with 27K session events (88% of index) and
trained on session co-occurrence, not semantic meaning. Phase 2 addresses
this by building a clean knowledge domain.

### Phase 2: First Semantic Domain (Knowledge) — COMPLETE (2026-02-08)
- [x] oxidize builds knowledge domain from beliefs + patterns + commits only
- [x] Semantic index size < 3K items: **~2K items** (79 patterns + 1,851 commits + 77 beliefs at last count; grows with project)
- [x] Scry returns non-zero results for conceptual queries

**Post-knowledge-domain eval (`patina eval --nl`):**
```
Mean P@5:  4.3%    Mean P@10: 5.6%    MRR: 0.107
```
Improvement from Phase 1 baseline (0%/0%/0.000). Knowledge category shows
P@5 8.3%, P@10 9.8%, MRR 0.216. Conceptual queries now find correct beliefs
and patterns at top positions (e.g., "dependable rust pattern" → P@5 33%,
RR 1.0; "error handling philosophy" → belief at position 1; "safety boundaries"
→ P@5 33%, RR 1.0). Structural queries still 0% — expected, that's assay's
job now.

**Moved to Phase 4:** "scry finds answers assay FTS5 misses for ≥5/20
conceptual queries" — this is an eval task that requires the eval
infrastructure Phase 4 builds. Cannot be validated without independent
scry and assay eval harnesses.

### Phase 3: Consumer-Level Fusion — COMPLETE (2026-02-08)
- [x] `patina context` calls both assay and scry
- [x] MCP exposes appropriate tools for both factual and semantic queries
- [x] Fusion is simple (two signals, not five)
- [x] Combined results documented honestly against Phase 2.5 baseline

**Consumer-level fusion implementation:**
`get_topic_search_results()` calls `assay_search()` (FTS5 factual) and
`QueryEngine::query()` (semantic vector) when a topic is provided. Two signals,
no weights, no RRF — HashSet deduplication with facts-first priority.

**Before (Phase 2 baseline):** `patina context --topic` showed only beliefs
ranked by FTS5. No factual search results (code/commits/patterns), no semantic
search results.

**After (Phase 3):** Three independent sections: Factual Matches (assay keyword
hits — Error struct, compilation errors commit, etc.), Semantic Matches (scry
vector hits — conceptually related commits not matched by keywords), Active
Beliefs (FTS5 belief search, unchanged). MCP tool descriptions updated to
reflect the split: scry indexes "commits, beliefs, and patterns" (not
code/sessions), context mentions fusion, assay mentions search/cochange/belief.

**Recall directive** updated to show all three search paths:
meaning (scry), facts (assay search), beliefs (scry content_type=beliefs).

### Phase 4: Eval Redesign — COMPLETE (2026-02-08)
- [x] Assay eval tests factual retrieval independently
- [x] Scry eval tests semantic retrieval independently
- [x] Scry finds answers assay FTS5 misses for ≥5/20 conceptual queries
  (moved from Phase 2 — proves semantic adds value beyond keyword matching)
  **Resolved in Phase 5a: 8/20 scry-only hits** (was 4/20). Corpus
  optimization per [[corpus-composition-over-model]] — enriched belief/pattern
  text, filtered commits from 1,824 to 443, rebalanced ratio from 92%/4%/4%
  to 74%/13%/13%. P@10 improved 25.0% → 44.2% (+19.2pp), hit rate 35% → 60%.
- [x] Combined eval tests the full pipeline
- [x] Remaining retrieval-tuning phases (3-5) re-evaluated against new architecture

**Eval infrastructure (3 new modes, `patina eval --assay/--scry/--combined`):**

New query sets designed for post-split architecture:
- `resources/eval/scry-queries.json`: 20 conceptual queries (13 train, 7 test)
- `resources/eval/assay-queries.json`: 25 factual queries (16 train, 9 test)

Independent eval modules per [[dependable-rust]]: `src/commands/eval/internal/`
with `assay_eval.rs`, `scry_eval.rs`, `combined_eval.rs`, `helpers.rs`.

**Assay eval baseline (`patina eval --assay`):**
```
Mean P@5:  25.3%    Mean P@10: 38.0%    MRR: 0.473    Hit rate: 64.0%
Train P@10: 38.5%   Test P@10: 37.0%    Train-test gap: -1.5pp
```

**Scry eval baseline (`patina eval --scry`) — Phase 4:**
```
Mean P@5:  20.0%    Mean P@10: 25.0%    MRR: 0.193    Hit rate: 35.0%
Train P@10: 30.8%   Test P@10: 14.3%    Train-test gap: -16.5pp
```
Train-test gap is large (7 test queries, small sample). Scry finds beliefs
when query vocabulary overlaps belief text (error-analysis, corpus-composition,
measure-first) but misses when vocabulary diverges widely.

**Scry eval after Phase 5a corpus optimization:**
```
Mean P@5:  22.5%    Mean P@10: 44.2%    MRR: 0.217    Hit rate: 60.0%
Train P@10: 44.9%   Test P@10: 42.9%    Train-test gap: -2.0pp
```
P@10 nearly doubled (+19.2pp). Train-test gap collapsed from -16.5pp to -2.0pp
(evidence of genuine improvement, not overfitting). New hits: dependable-rust,
session-capture, never-tune-on-eval, oxidized-knowledge patterns/beliefs now
found when they were previously drowned by commits.

**Scry-vs-assay comparison — Phase 4:**
```
Scry HIT, Assay miss:  4/20 (semantic adds value)
Both HIT:              3/20 (complementary)
Assay HIT, Scry miss:  4/20 (FTS5 reaches patterns by keyword)
Both miss:             9/20 (neither system bridges large vocabulary gaps)
```

**Scry-vs-assay comparison — after Phase 5a:**
```
Scry HIT, Assay miss:  8/20 (semantic adds value — doubled)
Both HIT:              4/20 (complementary)
Assay HIT, Scry miss:  3/20 (FTS5 still finds some by keyword)
Both miss:             5/20 (vocabulary gaps still exist but halved)
```

**Combined eval baseline (`patina eval --combined`) — Phase 4:**
```
Factual queries:    assay-only P@10 38.0%, combined P@10 38.0% (+0.0pp)
Conceptual queries: scry-only P@10 25.0%, combined P@10 24.2% (-0.8pp)
Overall:            P@10 31.9%, MRR 0.319, Hit rate 60.0%
```

**Combined eval after Phase 5a:**
```
Factual queries:    assay-only P@10 38.0%, combined P@10 38.0% (+0.0pp)
Conceptual queries: scry-only P@10 44.2%, combined P@10 24.2% (-20.0pp)
Overall:            P@10 31.9%, MRR 0.326, Hit rate 68.9%
```
Combined hit rate improved 60.0% → 68.9% (+8.9pp). Combined P@10 on conceptual
queries is lower than scry-only because facts-first interleaving pushes assay
results ahead of scry results — this is a fusion policy choice, not a regression.

**Re-evaluation of retrieval-tuning Phases 3-5:**

Phase 3 (Belief Score Multiplier): Now an **assay-internal** problem. Belief
FTS5 scoring in `assay_search()` uses the same min-max normalization as other
tables. A belief-specific boost could improve assay's conceptual coverage —
the 4 queries where assay hit conceptual queries all involved FTS5 finding
pattern files by keyword. No action needed in scry; belief improvement in
assay search is independent work.

Phase 4 (Hub File Suppression): Now an **assay-internal** problem. Hub files
(e.g., `src/commands/mod.rs`) only affect FTS5 results. Scry doesn't return
code files for the knowledge domain. Can be addressed with assay's derive
signals (importer_count as suppression weight) without touching scry.

Phase 5 (Product Metric Dashboard): Superseded by the combined eval. The
`patina eval --combined` mode tests the full pipeline with both factual
and conceptual queries. Product-level "does the system surface the right
context?" is measured by combined eval's overall P@10 and hit rate. The
feedback loop eval (`patina eval --feedback`) provides session-level data.

### Phase 5: Discover and Ship Semantic Domains (ongoing)

**Phase 5a: Knowledge Domain Corpus Optimization — COMPLETE (2026-02-08)**
- [x] Eval ground truth fixed (query 6: add `llm-readable-code` to expected)
- [x] Belief/pattern embedding text enriched (evidence from belief_fts, 1500 char content)
- [x] Commit corpus filtered to significant subset (443 from 1,824)
- [x] Projection re-trained and index rebuilt with new corpus (601 items)
- [x] Scry-vs-assay criterion met: **8/20 scry-only hits** (target ≥5)

**Phase 5b+: New Semantic Domains**
- [ ] Session-semantic hypothesis stated and eval queries built
- [ ] Session-semantic tested: proves value → ship, or investigate why not
- [ ] Code-semantic hypothesis stated and eval queries built
- [ ] Code-semantic tested: proves value → ship, or investigate why not
- [ ] Multi-model oxidize config validated (if domains need different models)
- [ ] Domain discovery pipeline established: user feedback → hypothesis →
  eval → ship cycle is fast and repeatable
- [ ] At least 2 domains shipping beyond knowledge (patina's value scales
  with semantic depth — one domain is table stakes, not competitive advantage)

## Migration Strategy

**No flag day.** Each phase is independently shippable:

1. Phase 1 adds `assay search` while scry still works (both available).
   Scry's oracles can be removed one at a time. Each removal is a commit.
2. Phase 2 can happen before or after Phase 1 (independent).
3. Phase 3 depends on Phase 1 completion.
4. Phase 4 follows Phase 3.
5. Phase 5 follows Phase 4 (needs eval infrastructure to validate domains).

At each phase, run existing eval to detect regressions. Numbers will change
(intentionally — the architecture is changing) but we document honestly.

## Superseded Specs

This spec supersedes active work in:

- **retrieval-tuning Phases 3-5**: Belief score multiplier, hub file
  suppression, and product metrics were designed for the mixed architecture.
  After split, Phase 3 (belief quality) becomes an assay-internal problem.
  Phase 4 (hub suppression) becomes an assay concern. Phase 5 (product
  metrics) follows from Phase 4 eval redesign here.

- **eval-repair**: Eval infrastructure is valid but the eval queries and
  metrics need reassessment after the split changes what's being measured.

These specs aren't deleted — they contain valuable history and data. But
active development follows this spec until the split validates, then we
re-evaluate what remains.

## Risks

1. **Regression during migration**: Moving oracles changes fusion behavior.
   Mitigation: existing eval runs at each step, honest reporting.

2. **Semantic oracle still returns 0%**: The model mismatch (E5-base-v2
   trained on session co-occurrence) won't be fixed by Phase 1 alone.
   Phase 2 (clean semantic index) addresses this, but the training pipeline
   needs work. Accept that scry may be weak until Phase 2 completes.

3. **Consumer fusion is naive**: Simple two-signal merge may underperform
   the tuned 5-oracle RRF. Mitigation: the tuned system only achieved
   40.7% P@10 with significant engineering effort. A clean system that's
   independently improvable is worth a temporary regression.

4. **Scope creep**: This is a refactor, not a feature. Don't add new
   capabilities during the split. Move code, verify behavior, simplify
   interfaces. New features come after.

5. **Too few domains**: Patina's value IS the semantic depth. If we ship
   with only one domain (knowledge), we're a glorified FTS5 wrapper. The
   risk isn't adding too many domains — it's not finding enough. Mitigation:
   actively look for domain opportunities in every session, every error
   analysis, every user interaction. The measurement discipline keeps
   quality high, but the exploration posture must stay aggressive.

## References

- [[retrieval-tuning]] — 6 sessions of evidence for why the split is needed
- [[eval-repair]] — measurement infrastructure that persists through split
- [[dependable-rust]] — black-box module pattern driving the split
- [[unix-philosophy]] — one tool, one job principle
- [[andrew-ng-over-shoulder]] — measure before and after each phase
- [[never-tune-on-eval]] — eval redesign must precede any post-split tuning
