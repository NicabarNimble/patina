# Spec: Surface Layer

**Status:** Active (Next on deck)
**Created:** 2026-01-08
**Updated:** 2026-01-10
**Origin:** Sessions 20260108-124107, 20260108-200725, 20260109-063849, 20260110-154224

---

## North Star

> When I start a new project, my accumulated wisdom should be visible and usable from day 1.

Not queryable. Not "run scry and hope." **Visible. In files I can read.**

---

## Where We Are Today

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

### Current State of Surface

**What exists:**
- `layer/surface/` contains ~16 **manually written** markdown files
- These get **scraped** into `pattern.surface` events in eventlog
- They're **queryable** via `pattern_fts` (lexical) and semantic index
- They're **peers** to code/git as inputs, not outputs

**What's missing:**
- No `patina surface` command to generate nodes
- No automated extraction from scry/assay queries
- No cycle where surface is both input AND output

### The Gap

| Aspect | Today | Vision |
|--------|-------|--------|
| Content | Manual markdown | Auto-generated nodes |
| Source | Human writes | scry/assay queries |
| Format | Freeform docs | Atomic nodes with wikilinks |
| Cycle | Input only | Input → query → generate → input |

---

## Where We Want To Be

### The Cycle

Surface becomes both derived and committed:

```
Git → scrape → eventlog → scry/assay ──┐
                                       │
                            ┌──────────▼──────────┐
                            │   patina surface    │
                            │   (generates nodes) │
                            └──────────┬──────────┘
                                       │
                                       ▼
                               layer/surface/*.md
                                       │
                                       ▼
                                  git commit
                                       │
                                       └──────────→ (back to top)
```

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

### What Gets Generated

| Type | Example | Source |
|------|---------|--------|
| **Decision** | why-rouille, why-sqlite | Session "Key Decisions", commit rationale |
| **Pattern** | measure-first, scalpel-not-shotgun | Session "Patterns Observed" |
| **Concept** | sync-first, borrow-checker | Recurring ideas across sessions/commits |
| **Component** | scry, eventlog, oxidize | assay inventory (key modules) |

### The Graph

Wikilinks ARE the graph. No graph database needed.

```
sync-first ────────> rouille
    │
    ├──────────────> tokio (avoided)
    │
    └──────────────> borrow-checker
```

**Links from co-occurrence**: If two concepts appear in the same session or commit, they're related.

---

## Options to Get There

Three approaches identified in session 20260109-063849:

### Option A: Deterministic Extraction

Query scry/assay with fixed queries, format results directly.

```
assay inventory → component nodes
scry "decisions" → decision nodes
scry "patterns" → pattern nodes
```

**Pros:**
- Simple to implement
- Reproducible output
- No LLM dependency

**Cons:**
- Raw scry results are noisy (snippets, scores)
- No synthesis - just reformatted query results
- May produce low-quality nodes

**Best for:** Component nodes (assay data is already structured)

### Option B: LLM Synthesis

Use local LLM to transform query results into clean nodes.

```
┌─────────────────┐
│  Query Results  │  ← Raw, noisy
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Local LLM      │  ← Gemma 270M via ort
│  (cartridge)    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Surface Node   │  ← Clean, formatted, linked
└─────────────────┘
```

**Pros:**
- High-quality synthesis
- Can extract meaning from noisy results
- Natural language to structured output

**Cons:**
- Requires local LLM infrastructure
- Non-deterministic (same input → slightly different output)
- Adds complexity (model cartridges, inference)

**Best for:** Decision and pattern nodes (need synthesis)

### Option C: Hybrid (Recommended)

Combine both approaches based on node type.

| Node Type | Approach | Rationale |
|-----------|----------|-----------|
| **Component** | Deterministic | assay data is structured |
| **Decision** | LLM synthesis | needs interpretation |
| **Pattern** | LLM synthesis | needs interpretation |
| **Concept** | Deterministic | frequency-based extraction |

**Phase 1:** Start with deterministic (components + concepts)
**Phase 2:** Add LLM synthesis for decisions/patterns

---

## Infrastructure That Exists

| Component | Status | Location |
|-----------|--------|----------|
| scry queries | ✅ Working | `src/commands/scry/` |
| assay queries | ✅ Working | `src/commands/assay/` |
| ort (ONNX runtime) | ✅ In use | embeddings |
| patina model | ⚠️ Partial | model download/management |
| Gemma ONNX models | ✅ Available | HuggingFace |
| persona (template) | ✅ Working | LiveStore pattern for writes |

### What Needs to Be Built

| Component | Description |
|-----------|-------------|
| `src/commands/surface/mod.rs` | Command scaffolding |
| Surface node schema | Rust struct for node format |
| Deterministic extractors | Query → node for each type |
| (Optional) LLM cartridge | Gemma 270M for synthesis |
| MCP tool: `surface_add` | For LLM-driven surface updates |

---

## Implementation Path

### Phase 1: Deterministic Components

**Scope:** Generate component nodes from assay.

| Task | Description |
|------|-------------|
| Create `src/commands/surface/mod.rs` | Command scaffolding |
| Query `assay inventory` | Get key modules |
| Generate component nodes | One file per module |
| Extract links from imports | `import_facts` → wikilinks |

**Exit criteria:**
- `patina surface` creates component nodes
- Nodes have wikilinks from import relationships

### Phase 2: Concept Extraction

**Scope:** Extract recurring concepts from sessions/commits.

| Task | Description |
|------|-------------|
| Query sessions for recurring terms | Frequency analysis |
| Query commits for decision language | "because", "prefer", "instead of" |
| Generate concept nodes | Co-occurrence → links |

**Exit criteria:**
- Concept nodes generated from session/commit patterns
- Links from co-occurrence in same session

### Phase 3: LLM Synthesis (Optional)

**Scope:** Add local LLM for decision/pattern synthesis.

| Task | Description |
|------|-------------|
| Gemma cartridge setup | manifest + model + tokenizer |
| Synthesis prompts | Query results → structured node |
| Decision node generation | LLM interprets session decisions |
| Pattern node generation | LLM interprets observed patterns |

**Exit criteria:**
- Local LLM synthesizes high-quality nodes
- Decisions and patterns extracted with rationale

### Phase 4: Federation

**Scope:** Enable surface transfer between projects.

| Task | Description |
|------|-------------|
| `--surface-from` on init | Copy surface from path |
| `surface merge` command | Combine multiple surfaces |
| Federated scry | Query across project surfaces |

**Exit criteria:**
- New project bootstraps from past project's surface
- Cross-project queries work

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

**3. Generation Test**
```bash
patina surface
# Creates nodes from scry/assay queries
git status
# Shows new/modified files in layer/surface/
```

**4. Query Test**
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

---

## Open Questions

1. **Extraction quality**: How to filter meaningful nodes vs noise?
2. **Link typing**: Should links be typed (implements, avoids) or just connected?
3. **Manual edits**: Can users edit surface? Do edits survive regeneration?
4. **Staleness**: How to identify nodes no longer relevant?
5. **Incremental updates**: How to update existing nodes vs overwrite?

---

## References

- [Obsidian](https://obsidian.md) - Knowledge garden model
- [spec-pipeline.md](./spec-pipeline.md) - scrape/oxidize/scry pipeline
- Session 20260108-124107 - Initial design exploration
- Session 20260108-200725 - Refined as distillation layer
- Session 20260109-063849 - LLM synthesis and model cartridge design
- Session 20260110-154224 - Corrected to hub & spoke architecture
