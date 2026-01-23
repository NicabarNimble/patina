# Spec Review: Q4 2025 - Q1 2026

**Period:** October 15, 2025 → January 15, 2026
**Author:** Generated from session archaeology and git history
**Purpose:** Tell the story of 3 months of Patina development

---

## Executive Summary

**765 commits. 39 archived specs. ~180 sessions. One clear arc: from ambitious vision to grounded implementation.**

What started as a "neuro-symbolic knowledge system" with Prolog reasoning and distributed Turso databases evolved into a practical, local-first RAG network. The journey is marked by deliberate over-engineering attempts, honest self-critique, aggressive pruning, and ultimately, working software that aligns with its stated values.

---

## The Numbers

| Metric | Value |
|--------|-------|
| Total commits | 765 |
| Feature/fix/refactor commits | 358 |
| Archived spec tags | 39 |
| Sessions documented | ~180 |
| Lines removed (cleanup) | ~3,000+ |
| Baseline MRR achieved | 0.624 |

---

## Act I: The Neuro-Symbolic Dream (October 2025)

### The Vision

October began on a branch called `neuro-symbolic-knowledge-system`. The ambition was enormous:

- **Prolog as reasoning engine** - SQLite for storage, Prolog for inference
- **Three-layer persona architecture** - Observations → Beliefs → Intelligent Agent
- **Distributed belief system** - Master persona at `~/.patina/`, project-specific exceptions
- **Turso integration** - Cloud sync, multi-device, team knowledge bases

From session 20251026-072236:
> "The Core Model: Projects are fixed reality. Your Persona adapts to reality - you learn exceptions."

The design was beautiful on paper: atomic yes/no questions generated from observed behavior, beliefs codified with evidence citations, Prolog rules detecting conflicts.

### The Pivot

The ONNX embeddings work succeeded (spec/oxidize archived). Pure Rust, cross-platform, Twitter-scale proven via `ort` crate. This became foundation.

But the Turso dream hit reality. Session 20251101-180301 contains a brutally honest self-review:

> **Grade: B-**
>
> Good refactoring work, but questionable architecture and incomplete delivery. The code is cleaner and more consistent, but we're not actually closer to Turso integration - we've just rearranged the furniture.

The critique identified that the "database abstraction" rejected the very mechanism (traits) needed for backend swapping. The decision to use concrete types over traits was called a "post-hoc rationalization."

**Key lesson documented:** "Documentation debt compounds. Initial optimistic framing in docs became harder to correct later - better to be honest from start."

### What Survived

- ONNX embeddings infrastructure (pure Rust, INT8 quantized)
- SqliteDatabase wrapper pattern
- Persona session commands (`/persona-start`, `/persona-end`)
- The philosophical grounding: layer/core values as decision framework

### What Was Deferred

- Prolog reasoning engine
- Turso integration
- Distributed sync
- Team knowledge bases

---

## Act II: Infrastructure & Measurement (November 2025)

### The Mother Emerges

November shifted focus from distributed dreams to local-first reality. The "mother" concept transformed from "Turso cloud sync" to something more practical: `~/.patina/` as a local knowledge hub.

Key commits:
- `a609d1546` feat(repo): add mothership MVP for cross-project knowledge
- `44a43c5c` feat(scry): add auto-detecting lexical search via FTS5
- `9642ef3e` feat(oxidize): integrate temporal dimension into pipeline

The `patina repo add <url>` command enabled reference repos - external codebases indexed locally for cross-project queries.

### Measurement Culture Begins

The evaluation framework arrived:
- `fa9c9f25` feat(eval): add evaluation framework for retrieval quality
- Ground truth queryset (`eval/retrieval-queryset.json`)
- MRR, Recall@k benchmarking
- Baseline established: MRR 0.624, Recall@10 67.5%

From session notes: "Andrew Ng's measurement-first approach" became a guiding principle.

### Scry MVP Complete

The query command went from idea to working:
- Semantic search (embedding-based)
- Lexical search (FTS5)
- Temporal queries (co-change relationships)
- Hybrid retrieval with RRF fusion

### The Big Archive

55 outdated docs were moved to `layer/dust/`. The surface documentation got a major cleanup. This was the first major pruning pass.

---

## Act III: Quality Gates & Architectural Alignment (December 2025)

### The Regression Crisis

December began with a crisis: MRR had regressed from 0.624 to 0.427 without anyone noticing. Session 20251227-122051 documents the investigation:

**Root causes:**
1. Stale database entries (154 from deleted commands polluting index)
2. Outdated ground truth paths (files had been refactored)

**Fix:**
- Updated queryset paths
- Rebuilt database and indices
- Result: MRR 0.427 → 0.588 (+37.7%)

**Lesson learned:** "Stale data is silent" - without regular benchmarks, quality degrades invisibly.

### The CI Quality Gate

A quality gate was added to CI - then immediately changed from blocking to warning-only after discussion:

> "Prevents blocking PRs due to ground truth staleness, file moves/renames. Balances quality visibility with development velocity."

This became a pattern: measure everything, but don't let metrics become obstacles.

### The Great Refactoring

The scry command was identified as violating layer/core values: 2,141 lines in a single file, 30 functions, no module decomposition.

The refactor:
```
Before: scry/mod.rs (2,141 lines)
After:  scry/mod.rs (235 lines) + internal/*.rs (2,627 lines in 8 modules)
```

Same pattern applied to assay:
```
Before: assay/mod.rs (997 lines)
After:  assay/mod.rs (135 lines) + internal/*.rs
```

### Legacy Removal

Four commands were archived and removed (session 20251227-122051):
- query (177 lines) - superseded by scry
- belief (198 lines) - experimental, unused
- embeddings (190 lines) - superseded by oxidize
- ask (357 lines) - low usage, superseded by scry

Later, more cleanup:
- audit.rs removed
- layer/dust/repos system removed
- ~1,100 lines total in spec/remove-legacy-repos-and-audit

### Specs Completed in December

| Spec | What It Delivered |
|------|-------------------|
| spec/quality-gates | MRR regression fix, CI gate |
| spec/command-refactoring | scry/assay decomposition |
| spec/remove-legacy-repos-and-audit | ~1,100 lines removed |
| spec/observable-scry | Structured response, feedback logging |
| spec/secrets-v2 | age-encrypted vault with Touch ID |
| spec/agentic-rag | Oracle abstraction, hybrid retrieval, MCP server |
| spec/mcp-retrieval-polish | MCP tool rename, temporal oracle |
| spec/model-management | Base model download, caching |
| spec/feedback-loop | Measure and learn from retrieval quality |
| spec/assay | Structural queries + signals |

---

## Act IV: Forge & Federation (January 2026)

### The Adapter Unification

January's first major push unified the "LLM frontend" concept into "adapters":

- `98d21bcb` WIP: refactor frontend → adapter terminology
- `454cb71f` refactor(adapters): remove Codex from adapter system
- `e262fb97` feat(adapters): add select_adapter() function
- `53b5f2ff` feat(launch): implement two-flow adapter selection

**Key distinction discovered:** Codex is an *agent* (spawned BY adapters), not an adapter (launched WITH patina). This conceptual clarity drove the removal.

OpenCode was wired in as a strategic multi-provider option.

### The Forge Journey

The forge (GitHub integration) went through a complete arc:

**Phase 1: Abstraction** (spec/forge-abstraction)
- ForgeReader + ForgeWriter traits
- GitHub implementation
- Conventional commit parsing

**Phase 2: Sync v2** (spec/forge-sync-v2)
- Background sync via fork
- PID guards
- 750ms API pacing
- --sync/--log/--limit flags

**Phase 3: Bulk Fetch** (spec/forge-bulk-fetch)
The breakthrough. Session 20260114-114833 documents:

> Changing `limit: 500` → `limit: 50000` reduced claude-code sync from **3.7 hours to 3:20 minutes** (fetching 17,509 issues in one call vs 18k individual API calls).

The wasteful `discover_all_issues()` function was deleted. 100x performance improvement.

### Lean Storage for Ref Repos

spec/ref-repo-storage introduced a key insight:

> "Don't store what you can compute" - git data is derived (rebuild from source), but API responses are captured knowledge worth preserving.

Results across ref repos:
| Repo | Before | After | Reduction |
|------|--------|-------|-----------|
| claude-code | 227MB | 202MB | 11% |
| opencode | 106MB | 78MB | 26% |
| dust | 51MB | 20MB | 60% |
| livestore | 96MB | 49MB | 44% |

### Self-Healing Infrastructure

spec/preflight added automatic cleanup of stale processes (>24h), preventing OOM conflicts and zombie processes.

### January Session Volume

**68 sessions in January alone** - nearly one every 5 hours. The development velocity was intense.

---

## Cross-Cutting Themes

### 1. Measurement-First Culture

Every major decision involved benchmarks:
- MRR regression caught and fixed
- CI quality gate (warning mode)
- Retrieval baselines: MRR 0.624, Recall@10 67.5%, Latency ~135ms

### 2. Spec-Driven Development

The pattern became ritualized:
1. Design exploration in sessions
2. Write spec with phases
3. Implement phase by phase
4. Archive completed spec as git tag

**39 archived spec tags** prove this wasn't just documentation theater.

### 3. Architectural Alignment

layer/core values (dependable-rust, unix-philosophy, adapter-pattern) became the decision framework:

From session 20251209-100946:
> "Slowing down to review before coding catches architectural issues early. The spec was mostly right, implementation deviated from it."

Commands that violated dependable-rust (monolithic files) were refactored. Code that didn't align was removed.

### 4. Aggressive Pruning

- 4 commands removed (922 lines)
- 55 docs archived to dust
- layer/dust/repos system removed (~1,100 lines)
- audit.rs deleted
- Codex adapter removed

**If it doesn't fit, delete it.**

### 5. Honest Self-Critique

The sessions contain remarkable intellectual honesty:

- Grade B- for database abstraction work
- "We've just rearranged the furniture"
- "Post-hoc rationalization of an implementation limitation"
- "Over-engineering" acknowledged repeatedly

This honesty prevented technical debt accumulation.

### 6. Vision → Reality Evolution

| Vision | Reality |
|--------|---------|
| Turso distributed sync | Local-first SQLite |
| Prolog reasoning engine | Deferred |
| Team knowledge bases | Single-user mother |
| Typed knowledge graph | Wikilinks in markdown |
| Cloud-synced beliefs | Local persona.db |

The vision shrank, but what remained *actually works*.

---

## The Architecture Today

```
                            GIT (source of truth)
                                    │
                   ┌────────────────┼────────────────┐
                   ▼                ▼                ▼
             scrape git      scrape code      scrape forge
           (commits+parsed)   (symbols)      (issues, PRs)
                   │                │                │
                   └────────────────┴────────────────┘
                                    │
                                    ▼
                               SQLite DB
                                    │
                   ┌────────────────┴────────────────┐
                   ▼                                 ▼
               oxidize                            assay
           (→ embeddings)                      (→ signals)
                   │                                 │
                   └────────────┬────────────────────┘
                                ▼
                              scry
                       (unified oracle)
                                │
                                ▼
                          LLM Frontend
                    (Claude, Gemini, OpenCode)
```

**Core insight from build.md:** "scry is the API between LLM and codebase knowledge. Everything else prepares for that moment."

---

## What's On Deck

### Active Specs

| Spec | Focus |
|------|-------|
| spec-surface-layer | Distill knowledge into atomic markdown with wikilinks |
| spec-report | Self-analysis reports using patina's own tools |
| spec-vocabulary-gap | LLM query expansion for terminology mismatch |
| spec-mother | Phase 1: Federated query |
| spec-database-identity | UIDs for databases, federation graph |

### Deferred Work

- spec-retrieval-optimization (Phase 2-4 need 100+ queries)
- spec-persona-fusion (Phase 2)
- spec-model-runtime (local LLM inference)

---

## Lessons for Future Sessions

1. **Write the spec first, implement second** - prevents "rearranging furniture"
2. **Benchmark before AND after** - silent regression is real
3. **Warning mode > blocking mode** for quality gates
4. **Delete aggressively** - code that doesn't align becomes debt
5. **Honest self-critique** - Grade B- is valuable feedback
6. **One commit per logical change** - 16 commits > 1 batch commit
7. **Ground fancy ideas in reality** - ask "what does this DO for me?"
8. **Prune early, prune often** - 55 docs archived was healthy

---

## Conclusion

Three months, 765 commits, 39 completed specs. The neuro-symbolic dream became a practical RAG network. Prolog was deferred but ONNX embeddings shipped. Turso sync became local mother. The vision contracted, but coherence emerged.

The codebase now aligns with its stated values. Commands follow dependable-rust. Adapters follow adapter-pattern. Each tool does one thing (unix-philosophy).

**What remains:** Surface layer (federation), reports (dogfooding), vocabulary gap (search quality). The foundation is solid. The measurement culture is established. The next phase is about building *on* the foundation, not *rebuilding* it.

---

*"Patina accumulates knowledge like the protective layer that forms on metal - your development wisdom builds up over time."*

The last three months were the forging. Now comes the patina.
