# Surface Layer - Design Document

Detailed architecture and implementation for [[SPEC.md]].

---

## Two Functions

### 1. Capture

**What:** Extract knowledge buried in raw data and generate atomic facts to `layer/surface/`.

**Source:** scry queries, assay queries, sessions, commits, patterns observed.

**Output:** Atomic markdown files with wikilinks that connect related concepts.

### 2. Curate

**What:** Manage the lifecycle of knowledge on the surface - score importance, promote evergreen facts to core, archive stale facts to dust, cull slop.

**Key insight:** Importance = user belief + proof in code/context. Something is important not just because it's mentioned, but because:
- The user believes it matters
- There's evidence in the codebase that it matters
- It has temporal weight (old but still referenced = evergreen)

---

## Current Architecture (Hub & Spoke)

```
                         GIT (source of truth)
                                │
                                ▼
                             SCRAPE
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          PATINA.DB (Hub)                                    │
│                                                                             │
│  eventlog ──────────────────────────────────────────────────────────────►  │
│       │                                                                     │
│       ├──► commits, commit_files        (materialized from git.commit)     │
│       ├──► function_facts, import_facts (materialized from code.*)         │
│       ├──► patterns                     (materialized from pattern.*)      │
│       ├──► forge_prs, forge_issues      (materialized from forge.*)        │
│       ├──► sessions, goals, observations (materialized from session.*)     │
│       │                                                                     │
│       └──► FTS5: code_fts, commits_fts, pattern_fts                        │
│                                                                             │
│  call_graph, co_changes, module_signals (structural)                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                │
          ┌─────────────────────┼─────────────────────┐
          │                     │                     │
          ▼                     ▼                     ▼
    ┌───────────┐        ┌───────────┐        ┌───────────┐
    │  OXIDIZE  │        │   SCRY    │        │   ASSAY   │
    │           │        │           │        │           │
    │ reads db  │        │ reads db  │        │ reads db  │
    │ writes    │        │ reads     │        │           │
    │ embeddings│───────►│ embeddings│        │           │
    └───────────┘        └───────────┘        └───────────┘
```

### Layer Structure (The Lifecycle)

```
layer/
├── core/       → Evergreen. Proven over time. Fundamental truths.
├── surface/    → Active. Current. In-use knowledge.
└── dust/       → Archived. Historical. No longer active.
```

Today this lifecycle is **manual**. Nothing automates promotion or archival.

---

## Capture: Implementation

### Node Format

Atomic markdown with frontmatter:

```markdown
---
type: decision|pattern|concept|component
extracted: 2026-01-08
sources: [session:20250804, commit:7885441]
---

# sync-first

Prefer synchronous, blocking code over async.

## Why
- Borrow checker works better without async lifetimes
- LLMs understand synchronous code better

## Links
- [[rouille]] - chosen because of this
- [[tokio]] - explicitly avoided
```

### What Gets Captured

| Type | Example | Source | Approach |
|------|---------|--------|----------|
| **Component** | scry, eventlog, oxidize | assay inventory | Deterministic |
| **Concept** | sync-first, borrow-checker | Recurring terms in sessions/commits | Deterministic (frequency) |
| **Decision** | why-rouille, why-sqlite | Session "Key Decisions", commit rationale | Adapter LLM synthesis |
| **Pattern** | measure-first, scalpel-not-shotgun | Session "Patterns Observed" | Adapter LLM synthesis |

### Link Generation

Wikilinks ARE the graph. No graph database needed.

**Links from:**
- **Imports** - component A imports component B → `[[B]]` in A's node
- **Co-occurrence** - concepts appear in same session/commit → linked
- **Explicit reference** - commit mentions "because of X" → link to X

### Connection Scoring

How do we identify that an idea in a session maps to evidence in code?

**Using existing infrastructure:**

| Tool | Provides | Already Exists? |
|------|----------|-----------------|
| Session embeddings | Vector for session text | ✅ oxidize |
| Commit embeddings | Vector for commit messages | ✅ oxidize |
| Semantic similarity | "Is A similar to B?" | ✅ scry semantic oracle |
| Temporal correlation | "Did A happen near B?" | ✅ timestamp comparison |
| Lexical overlap | "Do A and B share keywords?" | ✅ scry lexical oracle |

**What's new:**

| Need | Description |
|------|-------------|
| Connection query | Join session → commit by similarity + time |
| Connection storage | Table to store validated connections |
| Confidence threshold | Cutoff for "confident enough to surface" |
| Uncertainty queue | Low-confidence items for adapter review |

**The flow:**
```
Session idea (embedded)
        │
        ▼
Compare to recent commits (similarity score)
        │
        ▼
Check temporal correlation (session before commit?)
        │
        ▼
Confidence score = f(similarity, temporal, lexical)
        │
        ├── HIGH (>0.8) → Surface directly
        ├── MEDIUM (0.5-0.8) → Queue for adapter review
        └── LOW (<0.5) → Discard
```

### Mother's Role

**Mother orchestrates surface capture.** Mother is deterministic daemon code, NOT a local LLM.

```
┌─────────────────────────────────────────────────────────────┐
│                    MOTHER (Daemon)                          │
│                                                             │
│  Orchestrates existing tools:                               │
│    • oxidize embeddings (session → vector)                  │
│    • scry similarity (is A related to B?)                   │
│    • temporal correlation (A happened, then B happened)     │
│    • threshold rules (confidence > X → surface it)          │
│                                                             │
│  Outputs:                                                   │
│    • Connection candidates (high confidence → surface)      │
│    • Uncertainty queue (low confidence → adapter review)    │
│                                                             │
│  Does NOT:                                                  │
│    • Run inference (no local LLM in Phase 1-2)              │
│    • Make strategic decisions (adapters do this)            │
│    • Generate prose (adapters do this)                      │
└─────────────────────────────────────────────────────────────┘
```

**Key insight:** 90% of connection scoring already exists in oxidize/scry. Mother just orchestrates and stores validated connections.

---

## Curate: Implementation

### Importance Signals

| Signal | Description | Weight |
|--------|-------------|--------|
| **User endorsement** | User explicitly marks as important | High |
| **Code evidence** | Referenced in actual codebase | High |
| **Commit correlation** | Commits mention this concept | Medium |
| **Session frequency** | Appears across multiple sessions | Medium |
| **Recency** | Mentioned recently | Medium |
| **Age + still referenced** | Old but still appears in new work | High (evergreen signal) |

### Lifecycle States

```
                    ┌─────────────┐
                    │   Captured  │  (new node generated)
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
              ┌─────│   Surface   │─────┐
              │     └─────────────┘     │
              │                         │
              ▼                         ▼
       ┌─────────────┐          ┌─────────────┐
       │    Core     │          │    Dust     │
       │ (evergreen) │          │ (archived)  │
       └─────────────┘          └─────────────┘
```

**Promotion to Core:**
- High importance score sustained over time
- User endorsement
- Evidence of continued relevance (still referenced in new commits/sessions)

**Archival to Dust:**
- Low importance score
- No recent references
- Superseded by newer knowledge

---

## L2 Eventlog: Surface Decisions

### The Regeneration Problem

Surface data crosses a **non-deterministic boundary**:
- Adapter LLM synthesis (tokens, model version, non-reproducible)
- User curation (human judgment)

Unlike L1 data (git/code → eventlog → patina.db), surface cannot be regenerated identically from source.

**Helland's principle:** When you cross a non-deterministic boundary, the output becomes a new source of truth. You can't derive it - you must capture it.

### L1 vs L2 Eventlog

```
L1 EVENTLOG (patina.db source)
────────────────────────────────
git.commit, git.diff           → commits, commit_files
code.function, code.import     → function_facts, import_facts
session.start, session.end     → sessions, goals, observations
pattern.detected               → patterns
forge.issue, forge.pr          → forge_issues, forge_prs


L2 EVENTLOG (layer/surface/ source)
────────────────────────────────
surface.extract.*              → deterministic nodes (component, concept)
surface.synthesize.*           → LLM-generated nodes (decision, pattern)
surface.connection.*           → validated connections
surface.curate.*               → lifecycle transitions
```

### L2 Event Types

**Capture Events:**
```
surface.extract.component    {node_id, source: "assay:inventory", path}
surface.extract.concept      {node_id, source: "session:co-occurrence", terms}
surface.synthesize.decision  {node_id, prompt, response, model, tokens, sources}
surface.synthesize.pattern   {node_id, prompt, response, model, tokens, sources}
```

**Connection Events:**
```
surface.connection.scored    {session_id, commit_sha, score, method}
surface.connection.validated {connection_id, validator: "llm"|"user", confidence}
surface.connection.rejected  {connection_id, reason}
```

**Curate Events:**
```
surface.node.promoted  {node_id, from: "surface", to: "core", reason}
surface.node.archived  {node_id, from: "surface", to: "dust", reason}
surface.node.edited    {node_id, before_hash, after_hash, editor: "user"|"llm"}
surface.node.culled    {node_id, reason: "duplicate"|"slop"|"superseded"}
```

### Architecture with L2

```
                        GIT (source of truth)
                               │
                               ▼
                        L1 EVENTLOG
                        ┌─────────────────────────────────────┐
  git/code ────────────►│ git.*, code.*                       │
  sessions/layer ──────►│ session.*, pattern.*                │──► patina.db
  forge ───────────────►│ forge.*                             │
                        └─────────────────────────────────────┘
                                       │
                                       ▼
                                  scry/assay
                                       │
                        ┌──────────────┼──────────────┐
                        ▼              ▼              ▼
                  deterministic   adapter LLM      user
                  (extract)       (synthesize)   (curate)
                        │              │              │
                        └──────────────┴──────────────┘
                                       │
                                       ▼
                        L2 EVENTLOG (surface decisions)
                        ┌─────────────────────────────────────┐
                        │ surface.extract.*                   │
                        │ surface.synthesize.*                │──► layer/surface/
                        │ surface.connection.*                │    (documents)
                        │ surface.curate.*                    │
                        └─────────────────────────────────────┘
```

---

## Success Metrics Detail

### Baseline Measurements

**Baseline 1: Manual Session Audit**

Before implementing capture, manually audit 10 recent sessions:

1. Read session file completely
2. List all key decisions (should become decision nodes)
3. List all patterns observed (should become pattern nodes)
4. List all concepts introduced (should become concept nodes)
5. Note any session→commit connections

**Output:** `eval/surface-ground-truth.json`

**Baseline 2: Existing Surface Content**

```bash
# Component count
find layer/surface -name "*.md" | wc -l

# By type (from frontmatter)
grep -r "^type:" layer/surface/*.md | cut -d: -f3 | sort | uniq -c
```

**Baseline 3: Scry Coverage**

How often does scry currently return session/pattern content vs code?

### Evaluation Methodology

**Sample-Based Evaluation:**

1. Random sample: 50 nodes per evaluation
2. Stratified: 20 decisions, 15 patterns, 10 concepts, 5 components
3. Human rating: 3-point scale (useful / marginal / noise)
4. Inter-rater reliability: 2 raters, measure agreement

**Calibration Test:**

```python
bins = [(0.5, 0.6), (0.6, 0.7), (0.7, 0.8), (0.8, 0.9), (0.9, 1.0)]
for low, high in bins:
    connections = get_connections_in_range(low, high)
    actual_correct = human_label(connections)
    expected = (low + high) / 2
    calibration_error = abs(actual_correct - expected)
    # Target: calibration_error < 0.1
```

---

## Infrastructure

### What Exists (Use It)

| Component | Status | Location |
|-----------|--------|----------|
| Session embeddings | ✅ Working | `oxidize` → semantic index |
| Commit embeddings | ✅ Working | `oxidize` → semantic index |
| Semantic similarity | ✅ Working | `scry` semantic oracle |
| Lexical search | ✅ Working | `scry` lexical oracle (FTS5) |
| Temporal data | ✅ Working | `commits` table timestamps |
| Import relationships | ✅ Working | `import_facts` table |
| Mother daemon | ✅ Working | `patina serve` |
| Layer structure | ✅ Exists | `layer/core/`, `layer/surface/`, `layer/dust/` |

### What's New (Build It)

| Component | Description |
|-----------|-------------|
| L2 eventlog | Surface decisions (synthesize, curate, connect) |
| Connection query | Join session → commit by similarity |
| Connection storage | New table for validated connections |
| Confidence scoring | Algorithm combining similarity + temporal + lexical |
| Uncertainty queue | Storage for adapter review items |
| Surface command | `patina surface capture`, `patina surface status` |

---

## Design Principles

- **Distilled over raw** - Surface is extracted, not logged
- **Atomic over comprehensive** - One idea per file
- **Links over prose** - Wikilinks carry meaning
- **Portable over powerful** - Plain markdown, works anywhere
- **Flat over hierarchical** - Links create structure, not folders
- **Deterministic first** - Add LLM synthesis only where needed
- **Curated over accumulated** - Quality over quantity
- **Smart model in the room** - Use adapter LLMs for synthesis, not local models

---

## Open Questions

### Capture
1. What frequency threshold makes a term worth capturing?
2. What decision language patterns reliably indicate rationale?
3. What confidence threshold for automatic surfacing vs adapter review?

### Curate
1. How to weight user endorsement vs automated signals?
2. What threshold triggers promotion to core?
3. How to handle the existing manual docs in `layer/surface/`?

### Metrics
1. Who rates nodes? User? Automated heuristics?
2. How often to measure? Per release? Per session?
3. What's "useful"? Need clear rubric for human raters

---

## Session Notes

### 20260115-053944: Mother Architecture Crystallized

**Key Insight:** Mother is NOT a local LLM model - she's deterministic daemon code.

- Phase 1-2: Deterministic (existing embeddings + similarity + threshold rules)
- Phase 3: Adapter LLM synthesis (Claude/Gemini/OpenCode - smartest model in room)
- Phase 6+: Local model optimization (only after proven patterns exist)

### 20260115-121358: L2 Eventlog Design

**Design partners:** Helland (primary), Hickey (secondary), Fowler (tertiary)

**Key Insight:** Surface data crosses a non-deterministic boundary (LLM synthesis, user curation). Once crossed, the output becomes a new source of truth.

### 20260122-061519: Success Metrics

Added Andrew Ng-style measurement framework. Key principle: measure before you build.

---

## References

- [Obsidian](https://obsidian.md) - Knowledge garden model
- [spec-pipeline.md](../../spec-pipeline.md) - scrape/oxidize/scry pipeline
- [spec-mothership.md](../../spec-mothership.md) - Mother architecture
