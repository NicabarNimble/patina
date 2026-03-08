---
id: analysis-qmd-learnings
status: active
created: 2026-03-02
session: 20260302-061023
tags: [analysis, search, beliefs, retrieval, qmd, reference-repo, grounding, fusion]
references:
  - beliefs-are-the-product
  - beliefs-are-entities-not-documents
  - beliefs-are-where-machine-meets-human
  - patina-is-knowledge-protocol
  - evidence-driven-validation
  - graceful-degradation-over-strict-validation
  - belief-identity-is-slug-not-hash
  - git-is-the-knowledge-substrate
  - data-architecture-v2
  - semantic-structural-split
  - retrieval-tuning
related:
  - layer/surface/reports/data-flow-cheatsheet.md
  - layer/surface/analysis-command-architecture.md
sessions:
  - 20260302-061023  # this analysis
  - 20260208-070221  # semantic-structural split origin
  - 20260208-103844  # split implementation
  - 20260208-144855  # Phase 3 consumer-level fusion
  - 20260215-075638  # knowledge-protocol exploration, beliefs-are-the-product
  - 20260213-055346  # plugin ecosystem, patina-is-knowledge-protocol
  - 20260117-205031  # epistemic layer, beliefs as output
  - 20260226-065302  # data-architecture-v2 origin
  - 20260301-231350  # QMD ref repo indexed
beliefs:
  - beliefs-are-the-product
  - beliefs-are-entities-not-documents
  - beliefs-are-where-machine-meets-human
  - evidence-driven-validation
  - graceful-degradation-over-strict-validation
  - belief-identity-is-slug-not-hash
  - patina-is-knowledge-protocol
---

# QMD Learnings for Patina's Belief System

> What Tobi Lutke's QMD teaches about retrieval engineering,
> evaluated through the lens of Patina's own beliefs and architectural history.

## Context

[QMD](https://github.com/tobi/qmd) is an on-device markdown search engine
(~3500 LOC TypeScript, v1.1.0). It combines BM25 full-text search, vector
semantic search, and LLM reranking — all running locally via node-llama-cpp
with GGUF models. Indexed as reference repo `tobi/qmd` (261 commits, 759
symbols, 635 functions).

This analysis evaluates QMD's retrieval patterns against Patina's belief
system — the core product per [[beliefs-are-the-product]]. Everything here
is anchored in project history: sessions, beliefs, and architectural decisions
that shaped the current system. QMD was indexed as a reference repo in
[[session-20260301-231350]]. See also [[data-flow-cheatsheet]] for how
data moves through Patina's current pipeline.

## QMD's Architecture

9 source files, single SQLite database (FTS5 + sqlite-vec), 3 local GGUF
models (embedding 300MB, reranking 640MB, query expansion 1.1GB), YAML-based
collection config.

**Key abstractions:** Store (facade over DB ops), LlamaCpp (LLM interface
with lifecycle management), Collections (YAML config), Formatter (multi-format
output).

**The crown jewel — the `query` hybrid pipeline:**

```
User Query
  │
  ├─ BM25 probe → strong signal? (≥0.85, gap ≥0.15) → skip expansion
  │
  ├─ Query expansion (fine-tuned 1.7B model)
  │   └─ typed: lex (keyword), vec (semantic), hyde (hypothetical document)
  │
  ├─ Parallel retrieval: FTS5 + vector for each expansion
  │
  ├─ RRF fusion (k=60, original query 2x weight, top-rank bonus)
  │
  ├─ Chunk-then-rerank (best chunk per doc → cross-encoder)
  │
  └─ Position-aware blending
      Top 1-3:  75% retrieval / 25% reranker
      Top 4-10: 60% retrieval / 40% reranker
      Top 11+:  40% retrieval / 60% reranker
```

## Why This Matters for Patina

Patina is not a document search engine. Patina is a knowledge protocol
([[patina-is-knowledge-protocol]]) where the belief system is the product
([[beliefs-are-the-product]]). The [[data-architecture-v2]] spec describes
the architecture:

```
Layer 0: Events     (what happened)       IRREPLACEABLE
Layer 1: Structured (what we parsed)      REBUILDABLE
Layer 2: Semantic   (what things mean)    REBUILDABLE
Layer 3: Beliefs    (what we understand)  REBUILDABLE

    beliefs ground INTO layers 0-2 for evidence
    layers 0-2 feed UP into beliefs
    the loop IS the intelligence
```

The belief↔reality loop has three retrieval problems:

1. **Belief discovery** — finding beliefs relevant to a concept
2. **Belief grounding** — connecting beliefs to code/commits/events
3. **Relationship discovery** — finding implicit support/attack links

QMD's retrieval engineering addresses all three — not as document search,
but as quality improvements to the loop.

---

## Findings

### F-001: The Fusion Gap — Belief Search Has No Hybrid Mode

**Anchored in:** [[beliefs-are-entities-not-documents]], [[session-20260208-070221]]

[[session-20260208-070221]] found beliefs had MRR 0.241 in the mixed scry
pipeline and diagnosed the root cause: semantic and structural concerns mixed
in one pipeline caused cascading bugs across 6 sessions of [[retrieval-tuning]].
The fix — the [[semantic-structural-split]] — was correct: scry for meaning,
assay for facts.

Phase 3 of the split (implemented in [[session-20260208-144855]]) delivered
consumer-level fusion in the `context` command: assay facts first, scry
meaning for gaps. This works.

**The gap:** belief search itself has no fusion. You either `assay belief`
(FTS5 keyword) or `scry --belief` (semantic grounding), never both. An agent
searching for "error handling" beliefs gets different results depending on
which tool it uses. Neither tool alone captures the full picture.

**What QMD teaches:** Fusion belongs at the consumer level, not the pipeline
level. QMD fuses BM25 + vector results via RRF after each pipeline runs
independently. This is exactly the pattern `context` already uses.

**Opportunity:** A fused belief query mode — using the `context` command's
existing fact-first/meaning-for-gaps pattern — would let agents discover
beliefs through both keyword evidence and conceptual relevance in a single
query. The infrastructure exists; the composition is missing.

**Effort:** Medium. Reuse `context` fusion pattern.

### F-002: Grounding Chain Lacks a Quality Gate

**Anchored in:** [[data-architecture-v2]] principle 6, [[evidence-driven-validation]]

[[data-architecture-v2]] principle 6: *"A belief without evidence is an opinion.
Every belief should trace back through a grounding chain: events and code
provide evidence, evidence grounds beliefs."*

Today `compute_belief_grounding` in `src/commands/scrape/beliefs/mod.rs`
(the E4.6a implementation from [[session-20260202-130018]])
uses raw cosine similarity ≥0.85 from USearch. If a belief embedding is
near a commit embedding, that commit "grounds" the belief. No second opinion.

A belief about "safety boundaries" can get grounded to commits about "safe
Rust" (keyword overlap in embedding space) that are actually about memory
safety, not project safety boundaries. The grounding chain is only as
strong as embedding similarity — and embedding similarity is noisy.

**What QMD teaches:** After broad retrieval, send candidates through a
cross-encoder reranker that judges "is this actually relevant?" per item.
QMD chunks documents first, picks the best chunk per doc, then reranks
chunks (not full bodies) — a critical O(tokens) optimization.

**Opportunity:** After grounding retrieves candidate commits/code, add a
quality gate that evaluates "does this commit actually constitute evidence
for this belief statement?" This would improve the evidence chain that
[[evidence-driven-validation]] requires.

**Tension:** Requires an ONNX cross-encoder model, which is compatible with
Rust-first but adds a model dependency (~100-600MB). Worth an explore spec
to evaluate small rerankers (e.g., cross-encoder/ms-marco-MiniLM-L-6-v2
at ~80MB) against the current raw-cosine baseline.

**Effort:** High. Needs ONNX reranker evaluation.

### F-003: The Belief Graph IS a Query Expansion Engine

**Anchored in:** [[beliefs-are-the-product]], [[beliefs-are-where-machine-meets-human]]

QMD uses a fine-tuned 1.7B GGUF model to expand queries into typed variants
(lex/vec/hyde). Effective but contradicts Rust-first and adds a ~1.1GB model.

Patina doesn't need this. Patina's belief graph — with supports/attacks
relationships, facets, and evidence links — already encodes the conceptual
expansion knowledge an LLM would learn.

When searching for "error handling" beliefs:
- **Facets** expand to: `[safety, resilience, validation]`
- **Supports** expand to: [[safety-boundaries]], [[graceful-degradation-over-strict-validation]]
- **Applied-In** expand to: files/specs that implement these beliefs

This is more Patina-native than any LLM-based expansion. The belief graph
is the domain-specific expansion model that QMD's fine-tuned model
approximates for general markdown.

**Current state:** Scry already has an `expanded_terms` parameter for MCP
callers. The missing piece is Patina itself generating those terms from the
belief graph during belief-focused queries.

**Effort:** Medium.

### F-004: Strong Signal Short-Circuit

**Anchored in:** Performance optimization, trivial implementation

QMD probes BM25 first. If the top result scores ≥0.85 with ≥0.15 gap to
second place, it skips the expensive LLM query expansion entirely.

In Patina: when `assay belief dependable-rust` gets an exact FTS5 hit,
there's no reason to run the embedding pipeline. This matters because belief
queries happen during every scrape (for grounding computation) and every
`context` call with a topic.

**Effort:** Low. FTS5 confidence check before embedding.

### F-005: MCP Instructions Should Reflect Belief Landscape

**Anchored in:** [[data-architecture-v2]] principle 8: *"LLMs are first-class consumers."*

QMD's `buildInstructions()` function dynamically injects index state into the
MCP server's system prompt: collection count, document count, capability gaps,
search tool usage patterns. Every LLM connecting to QMD knows immediately
what's searchable and how to search it.

Patina's MCP server doesn't inject belief landscape context. An LLM consuming
Patina tools doesn't know: how many beliefs exist, which are contested, which
are high-entrenchment core values, or what the epistemic health looks like.

**Opportunity:** Dynamic MCP instructions built from `beliefs` table
aggregates — total count, contested count, entrenchment distribution,
recent activity, health warnings. Progressive disclosure: summary in
instructions, detail via tools.

**Effort:** Low.

### F-006: Proto-Belief Discovery Needs Typed Signal Routing

**Anchored in:** [[beliefs-are-where-machine-meets-human]], [[data-architecture-v2]] Layer 3

The [[data-architecture-v2]] vision describes proto-beliefs (emergent patterns)
maturing into named beliefs. This pipeline doesn't exist yet.

QMD's typed query routing (lex → FTS5, vec → embeddings, hyde → hypothetical
documents) isn't directly applicable, but the pattern is: different signals
route through different detectors, results fuse at the consumer level.

Proto-belief discovery will need: pattern anomaly detection, co-occurrence
clustering, session decision density analysis, cross-project belief
convergence. Each is a different signal type requiring different retrieval
infrastructure, same fusion pattern.

**Status:** Not yet actionable. Tied to data-architecture-v2 Layer 3 work.

---

## What QMD Does NOT Teach Patina

### Chunking strategy
QMD's 900-token smart chunking with break point scoring is excellent for
document search. Patina's content-aware composition (enriched 1500-char
belief strings) is the right approach for beliefs — beliefs aren't long
documents. The evidence sections that get truncated are better addressed
by multi-vector indexing (one embedding per evidence entry pointing back
to the same belief) than by document-style chunking.

### LLM-based query expansion
Contradicts Rust-first. Adds a 1.1GB model dependency. The belief graph
(F-003) already encodes the expansion knowledge. Using an LLM to expand
queries about beliefs would be using a general tool when a domain-specific
one exists.

### Position-aware blending
Solves an intra-pipeline problem. Patina's semantic-structural split means
fusion happens at the consumer level (`context`), not inside a pipeline
where retrieval and reranking positions compete. The blending weights are
an artifact of QMD's monolithic query pipeline.

### Content-addressed document storage
QMD stores documents by content hash for deduplication and embedding
invalidation. [[session-20260215-075638]] already explored and rejected
content addressing for beliefs: *"Belief identity is the slug, not the hash.
Beliefs are meant to be mutable."* ([[belief-identity-is-slug-not-hash]],
high entrenchment). See also [[git-is-the-knowledge-substrate]].

---

## Summary: Ranked by Impact x Fit

| # | Finding | Impact | Fit with Patina | Effort | Next Step |
|---|---------|--------|----------------|--------|-----------|
| 1 | Fused belief query | High | High — reuses `context` Phase 3 pattern | Medium | Spec candidate |
| 2 | Belief graph as expansion | High | Highest — uses beliefs to improve beliefs | Medium | Spec candidate |
| 3 | Grounding quality gate | High | High — strengthens evidence chain | High | Explore spec |
| 4 | Strong signal short-circuit | Medium | Clean fit | Low | Small refactor |
| 5 | MCP belief landscape | Medium | Aligns with data-arch-v2 §8 | Low | Small refactor |
| 6 | Proto-belief signal routing | Future | Tied to Layer 3 vision | — | Not yet actionable |

Items 4-5 are small wins deliverable without a spec. Items 1-2 are
spec candidates that improve the belief↔reality loop using existing
infrastructure. Item 3 is the deepest quality improvement but requires
evaluating ONNX cross-encoder models.

## QMD Design Decisions Worth Remembering

These aren't directly applicable but reflect strong engineering worth
referencing if these areas are revisited. Source: `~/.patina/cache/repos/tobi/qmd/src/`
(queryable via `patina scry "topic" --repo tobi/qmd`).

- **Three search tiers** (search/vsearch/query) — users choose latency vs
  quality explicitly. Patina's scry modes (find/detail/full/orient/recent)
  serve a similar purpose.
- **Fine-tuned query expansion** — Tobi trained a 1.7B model via GRPO
  reinforcement learning specifically for query expansion. The investment in
  domain-specific models over general ones is the principle worth noting.
- **Chunk-then-rerank** — reranking full documents is O(tokens). Pick the
  best chunk per doc first. This applies if Patina ever adds reranking.
- **Session lifecycle for LLMs** — `ILLMSession` with scoped access,
  inactivity timeouts, deduplicated model loading. Clean resource management
  pattern for on-device models.
- **YAML config + SQLite data** — collection definitions in human-editable
  YAML, derived data in SQLite. Clean separation of declaration from
  materialization. Parallels Patina's layer/ (declaration) vs .patina/
  (derived).

## Discovery Paths

Future sessions can find this report via:

- `patina scry "QMD learnings belief retrieval"` — semantic match
- `patina assay search "qmd belief grounding fusion"` — FTS5 keyword match
- `patina scry --belief beliefs-are-the-product` — grounding from the core belief
- `patina scry --belief evidence-driven-validation` — grounding from the evidence chain belief
- Session [[session-20260302-061023]] — the session that produced this analysis
- Frontmatter `references` — links back to all anchoring beliefs and specs
- Frontmatter `sessions` — links to all sessions whose findings informed this report
- Frontmatter `related` — links to [[data-flow-cheatsheet]] and [[analysis-command-architecture]]

**QMD source code:** `patina scry "topic" --repo tobi/qmd` or direct read
at `~/.patina/cache/repos/tobi/qmd/src/`
