# Spec: Surface Layer

**Status:** Active (Next on deck)
**Created:** 2026-01-08
**Updated:** 2026-01-15
**Origin:** Sessions 20260108-124107, 20260108-200725, 20260109-063849, 20260110-154224, 20260115-053944

---

## North Star

> When I start a new project, my accumulated wisdom should be visible and usable from day 1.

Not queryable. Not "run scry and hope." **Visible. In files I can read.**

---

## Two Functions

Surface layer has two distinct responsibilities:

### 1. Capture

**What:** Extract knowledge buried in raw data and generate atomic facts to `layer/surface/`.

**Source:** scry queries, assay queries, sessions, commits, patterns observed.

**Output:** Atomic markdown files with wikilinks that connect related concepts.

### 2. Curate

**What:** Manage the lifecycle of knowledge on the surface - score importance, promote evergreen facts to core, archive stale facts to dust, cull slop.

**Goal:** The surface should contain actionable, relevant knowledge - not an ever-growing pile of generated docs.

**Key insight:** Importance = user belief + proof in code/context. Something is important not just because it's mentioned, but because:
- The user believes it matters
- There's evidence in the codebase that it matters
- It has temporal weight (old but still referenced = evergreen)

---

## Current State

### The Architecture (Hub & Spoke)

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

### The Gap

| Aspect | Today | Vision |
|--------|-------|--------|
| Content | Manual markdown | Auto-generated + manually curated |
| Capture | Human writes everything | patina extracts from scry/assay |
| Curation | Manual file moves | patina scores, promotes, archives |
| Lifecycle | Ad-hoc | Systematic (surface → core or dust) |

---

## Capture: The How

### Intent

Extract knowledge from the database and generate atomic markdown files that:
- Are **visible** (plain files, readable without patina)
- Are **linked** (wikilinks create a graph)
- Are **sourced** (frontmatter cites where knowledge came from)
- Are **portable** (work in Obsidian, viewable by any LLM)

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

### Capture Phases

**Phase 1: Deterministic Components**
- Query `assay inventory` for key modules
- Generate component nodes with stats (lines, functions)
- Extract links from `import_facts`

**Phase 2: Deterministic Concepts + Connection Scoring**
- Query sessions for recurring terms (frequency analysis)
- Query commits for decision language ("because", "prefer", "instead of")
- Implement connection scoring (session → commit similarity)
- Generate concept nodes with co-occurrence links
- Store validated connections

**Phase 3: Adapter LLM Synthesis**
- Adapter LLM (Claude/Gemini/OpenCode) reviews uncertainty queue
- Adapter generates decision and pattern nodes with rationale
- Adapter validates/refines Mother's connection candidates
- Higher quality, uses the smartest model in the room

### Open Questions (Capture)

1. What frequency threshold makes a term worth capturing?
2. What decision language patterns reliably indicate rationale?
3. What confidence threshold for automatic surfacing vs adapter review?
4. How to measure connection scoring quality? (precision metric needed)

---

## Curate: The How

### Intent

Manage knowledge lifecycle so the surface remains **useful, not bloated**:
- Score importance of each node
- Promote proven knowledge to `layer/core/`
- Archive stale knowledge to `layer/dust/`
- Cull over-generation before it becomes slop

### Importance Signals

What makes an atomic fact "important"?

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

### Self-Managing Documentation

Patina should help manage its own build/spec system:

- **Detect completed specs** - all phases done, archive to git tag
- **Detect stale docs** - not referenced, not updated, suggest archival
- **Keep build.md current** - surface changes to roadmap automatically
- **Cull redundant nodes** - merge near-duplicates, remove noise

### Curate Phases

**Phase 1: Importance Scoring**
- Define scoring algorithm
- Score existing surface nodes
- Surface scores in node frontmatter or separate index

**Phase 2: Manual Curation Assist**
- `patina surface status` - show nodes ranked by importance
- `patina surface stale` - identify candidates for archival
- User confirms promotions/archival

**Phase 3: Automated Lifecycle**
- Automatic promotion suggestions
- Automatic archival of clearly stale nodes
- Integration with spec/build system

### Open Questions (Curate)

1. How to weight user endorsement vs automated signals?
2. What threshold triggers promotion to core?
3. What threshold triggers archival to dust?
4. How to handle the existing manual docs in `layer/surface/`?
5. Should curate run automatically or only on command?

---

## Implementation Path

### Milestone 1: Capture Foundation
- `patina surface capture` command
- Component nodes from assay
- Basic wikilinks from imports
- **Exit:** Can generate component nodes

### Milestone 2: Connection Scoring
- Session → commit similarity query
- Connection storage table
- Confidence threshold + uncertainty queue
- Concept nodes with validated connections
- **Exit:** Mother can score connections, queue uncertainties

### Milestone 3: Curate Foundation
- Importance scoring algorithm
- `patina surface status` command
- Manual promote/archive workflow
- **Exit:** Can see importance rankings, manually curate

### Milestone 4: Adapter LLM Synthesis
- Adapter LLM reviews uncertainty queue during sessions
- Generates decision/pattern nodes with rationale
- Validates Mother's connection candidates
- **Exit:** Can generate high-quality decision/pattern nodes

### Milestone 5: Automated Curation
- Automatic staleness detection
- Promotion/archival suggestions
- Self-managing spec system
- **Exit:** patina helps manage its own documentation

### Milestone 6 (Future): Local Model Optimization
- Local model (Gemma) handles routine synthesis offline
- Only when proven patterns exist from Milestone 4
- Reduces dependency on adapter LLM for common cases
- **Exit:** Offline mode for routine surface maintenance

---

## Success Criteria

**1. Visibility Test**
```bash
cat layer/surface/sync-first.md
# Shows: what it is, why, related concepts, sources
# No patina query needed to read
```

**2. Portability Test**
```bash
# Open in Obsidian
open layer/surface/ -a Obsidian
# Graph view shows connected concepts
```

**3. Capture Test**
```bash
patina surface capture
# Creates nodes from scry/assay queries
git status
# Shows new/modified files in layer/surface/
```

**4. Connection Test**
```bash
patina surface connections
# Shows: session X → commit Y (confidence: 0.87)
# Queued for review: 3 uncertain connections
```

**5. Curate Test**
```bash
patina surface status
# Shows nodes ranked by importance
# Identifies stale candidates
```

**6. Query Test**
```bash
patina scry "why did we choose rouille?"
# Returns surface node with decision rationale
```

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
| Connection query | Join session → commit by similarity |
| Connection storage | New table for validated connections |
| Confidence scoring | Algorithm combining similarity + temporal + lexical |
| Uncertainty queue | Storage for adapter review items |
| Surface command | `patina surface capture`, `patina surface status` |

---

## Appendix: Session Notes

### Session 20260110-181504: Ref Repo Exploration

For ref repos (external repos without `layer/`), what data can serve as their "surface layer"?

**Key Insight:** For ref repos, **issues/PRs ARE their surface layer**:
- Bug discussions → why things broke
- Feature rationale → why things were added
- PR descriptions → design decisions
- Comments → community knowledge

**Issues Found:**
1. Forge data not in semantic index (oxidize doesn't embed issues)
2. Ref repos don't scrape documentation (README, STYLE_GUIDE, etc.)
3. Rate limiting needed for PR fetching

### Session 20260115-053944: Mother Architecture Crystallized

**Key Insight:** Mother is NOT a local LLM model - she's deterministic daemon code.

- Phase 1-2: Deterministic (existing embeddings + similarity + threshold rules)
- Phase 3: Adapter LLM synthesis (Claude/Gemini/OpenCode - smartest model in room)
- Phase 6+: Local model optimization (only after proven patterns exist)

**90% of connection scoring already exists** in oxidize/scry. Mother orchestrates, stores connections, queues uncertainties.

---

## References

- [Obsidian](https://obsidian.md) - Knowledge garden model
- [spec-pipeline.md](./spec-pipeline.md) - scrape/oxidize/scry pipeline
- [spec-mothership.md](./spec-mothership.md) - Mother architecture
- Session 20260108-124107 - Initial design exploration
- Session 20260108-200725 - Refined as distillation layer
- Session 20260109-063849 - LLM synthesis and model cartridge design
- Session 20260110-154224 - Corrected to hub & spoke architecture
- Session 20260110-181504 - Ref repo exploration, forge data gaps
- Session 20260115-053944 - Two functions: Capture & Curate, Mother crystallized
