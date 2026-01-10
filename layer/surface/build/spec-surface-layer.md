# Spec: Surface Layer

**Status:** Design
**Created:** 2026-01-08
**Updated:** 2026-01-08
**Origin:** Sessions 20260108-124107, 20260108-200725

---

## North Star

> When I start a new project, my accumulated wisdom should be visible and usable from day 1.

Not queryable. Not "run scry and hope." **Visible. In files I can read.**

---

## Position in the Stack

Surface is the **distillation layer** above scry/assay/oxidize.

```
                      ┌─────────────────────┐
                      │   FUTURE TOOLS      │  ← Higher insights
                      │   (not yet built)   │
                      └──────────┬──────────┘
                                 │
                      ┌──────────▼──────────┐
                      │      SURFACE        │  ← The patina
                      │  (git, portable,    │  ← Federation interface
                      │   queryable)        │  ← Other projects query this
                      └──────────┬──────────┘
                                 │
           ┌─────────────────────┼─────────────────────┐
           │                     │                     │
  ┌────────▼────────┐   ┌────────▼────────┐   ┌────────▼────────┐
  │    SCRY         │   │    ASSAY        │   │   OXIDIZE       │
  │  (query)        │   │  (structure)    │   │  (embeddings)   │
  └────────┬────────┘   └────────┬────────┘   └────────┬────────┘
           │                     │                     │
           └─────────────────────┼─────────────────────┘
                                 │
                      ┌──────────▼──────────┐
                      │     EVENTLOG        │  ← Local, rebuilt
                      └──────────┬──────────┘
                                 │
                      ┌──────────▼──────────┐
                      │        GIT          │  ← Source of truth
                      │   (sessions, code,  │
                      │    surface, core)   │
                      └─────────────────────┘
```

Surface is **output** of scry/assay/oxidize, not a parallel input. It's the crystallization layer.

---

## Three Raw File Types

| Type | Captures | Written When |
|------|----------|--------------|
| **Sessions** | User ↔ LLM conversation | During/after interaction |
| **Git** | User/LLM ↔ Code | When code changes |
| **Surface** | Distilled understanding | When knowledge is extracted |

- Sessions are the raw log of *talking*
- Commits are the raw log of *doing*
- Surface is the raw log of *understanding*

We built the extraction layer for code and git. Sessions got added to that pipeline. Surface is the **next level up** - distilled understanding extracted from querying the stack.

---

## The Cycle

Surface is both derived and committed:

```
Git → eventlog → scry/assay → SURFACE → Git (cycle)
```

- **Derived**: Extracted by querying the stack below
- **Committed**: Becomes part of source of truth
- **Queryable**: Future scry can search surface too
- **Portable**: Travels via git, enables federation

---

## What Surface Captures

| Type | Example | Source |
|------|---------|--------|
| **Decision** | why-rouille, why-sqlite | Session decisions, commit rationale |
| **Pattern** | measure-first, scalpel-not-shotgun | Session patterns observed |
| **Concept** | sync-first, borrow-checker | Recurring ideas across sessions/commits |
| **Component** | scry, eventlog, oxidize | Code structure (assay) |

---

## Structure

Flat namespace. Let links carry structure, not folders.

```
layer/surface/
├── sync-first.md
├── rouille.md
├── measure-first.md
├── eventlog.md
├── scry.md
└── ...
```

No subdirectories. Importance emerges from connectivity, not hierarchy.

---

## Node Format

Minimum viable surface node:

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
- [[borrow-checker]] - key enabler
```

**Key elements:**
- **Frontmatter**: type, extraction date, sources
- **Title**: node name (matches filename)
- **Description**: one sentence (or brief paragraph)
- **Why**: rationale (optional but valuable)
- **Links**: wikilinks to related nodes

---

## The Graph

Wikilinks ARE the graph:

```
sync-first ────────> rouille
    │
    ├──────────────> tokio (avoided)
    │
    └──────────────> borrow-checker
```

No graph database. Just files linking to files.

**Backlinks emerge**: When you open `rouille.md`, tools can show what links TO it.

---

## Generation

### Command: `patina surface`

```bash
patina surface              # Generate/update surface from stack
patina surface --dry-run    # Preview what would be created/changed
```

### How It Works

`patina surface` queries the tools we already built:

1. **Query scry**: "What decisions were made?" → decision nodes
2. **Query scry**: "What patterns recur?" → pattern nodes
3. **Query assay**: "What are the key modules?" → component nodes
4. **Query scry**: "What concepts appear frequently?" → concept nodes
5. **Extract links**: Co-occurrence in same session/commit → wikilinks
6. **Write files**: Atomic markdown to `layer/surface/`
7. **Commit**: Surface goes into git

The extraction is automated. The tools do the heavy lifting. Surface is the **materialized view** of what they know.

### What Gets Extracted

**From sessions (via scry):**
- "Key Decisions" sections
- "Patterns Observed" sections
- Concepts that recur across sessions

**From commits (via scry):**
- Messages with decision language ("because", "instead of", "prefer")
- Conventional commit types as context

**From code (via assay):**
- Key modules/components
- Architectural relationships (imports, callers)

---

## The LLM as Driver

Surface exists to make LLMs smarter. The flow:

```
User: "How should I structure entities in dojo?"
                │
                ▼
Claude calls: mcp__patina__scry("dojo entity structure")
                │
                ▼
Scry queries: local surface + eventlog + (federated mothership)
                │
                ▼
Returns: Decisions, patterns, concepts with provenance
                │
                ▼
Claude: "Based on your past patterns, you typically..."
```

Surface is queryable via the existing MCP tools. No new interface needed.

---

## Federation

### Surface vs Persona

| Aspect | Project Surface | Mothership Persona |
|--------|-----------------|-------------------|
| Scope | This project | All projects |
| Location | `layer/surface/` (in git) | `~/.patina/persona/` (local) |
| Content | Project-specific knowledge | User beliefs across projects |
| Federation | Exports to persona | Imports to new projects |

**Surface → Persona**: Project learnings bubble up to user beliefs
**Persona → Surface**: User beliefs seed new project surface

### Cross-Project Flow

```
Project A surface ──┐
                    ├──merge──> Mothership ──bootstrap──> New Project
Project B surface ──┘
```

Surface is the **federation interface**. Other projects can't query your:
- eventlog (not in git)
- embeddings (local, machine-specific)

Other projects CAN query your:
- surface (in git, portable, readable)

### Transfer Commands

```bash
# Bootstrap new project from past project's surface
patina init new-game --surface-from ~/projects/past-game

# Merge multiple surfaces
patina surface merge --from game-1 --from game-2

# Generate surface from ref repo
patina surface --from-ref dojo
```

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
# Any LLM can read and understand
```

**3. Bootstrap Test**
```bash
patina init test-project --surface-from ~/projects/past-project
ls layer/surface/
# Has content from day 1
```

**4. Query Test**
```bash
patina scry "why did we choose rouille?"
# Returns surface node with decision rationale
```

---

## Implementation Phases

### Phase 1: Basic Generation

**Scope:** Generate surface nodes by querying scry/assay.

| Task | Description |
|------|-------------|
| Create `src/commands/surface/mod.rs` | Command scaffolding |
| Query scry for decisions | Extract from sessions/commits |
| Query assay for components | Extract key modules |
| Generate atomic markdown | One file per node |
| Extract links from co-occurrence | Same session = related |

**Exit criteria:**
- `patina surface` creates node files
- Files have wikilinks from co-occurrence

### Phase 2: Incremental Updates

**Scope:** Update surface without full regeneration.

| Task | Description |
|------|-------------|
| Track extraction state | What's already processed |
| Incremental extraction | Only new sessions/commits |
| Merge into existing nodes | Update sources, add links |

**Exit criteria:**
- `patina surface` is incremental
- Existing nodes grow with new sources

### Phase 3: Federation

**Scope:** Enable surface transfer between projects.

| Task | Description |
|------|-------------|
| `--surface-from` on init | Copy surface from path |
| `surface merge` command | Combine multiple surfaces |
| Mothership project registry | Track project surfaces |
| Federated scry | Query across project surfaces |

**Exit criteria:**
- New project bootstraps from past project
- Scry queries federated surface

---

## Design Principles

- **Distilled over raw** - Surface is extracted, not logged
- **Atomic over comprehensive** - One idea per file
- **Links over prose** - Wikilinks carry meaning
- **Queryable over readable** - Optimized for LLM access
- **Portable over powerful** - Plain markdown, works anywhere
- **Flat over hierarchical** - Links create structure

---

## What Surface Is

- **Distillation layer** above scry/assay/oxidize
- **Materialized view** of extracted knowledge
- **Federation interface** for cross-project queries
- **LLM memory** - externalized understanding

## What Surface Is NOT

- **Not a log** - That's sessions
- **Not a query system** - That's scry
- **Not a database** - That's eventlog
- **Not manual docs** - It's generated
- **Not persona** - That's user-level, surface is project-level

---

## Open Questions

1. **Extraction quality**: How to extract meaningful nodes vs noise?
2. **Link typing**: Should links be typed (implements, avoids) or just connected?
3. **Manual edits**: Can users edit surface? Do edits survive regeneration?
4. **Staleness**: How to identify nodes no longer relevant?

---

## References

- [Obsidian](https://obsidian.md) - Knowledge garden model
- [spec-three-layers.md](./spec-three-layers.md) - mother/patina/awaken separation
- [spec-pipeline.md](./spec-pipeline.md) - scrape/oxidize/scry pipeline
- Session 20260108-124107 - Initial design exploration
- Session 20260108-200725 - Refined as distillation layer

---

## Session Context

**Journey to this spec:**

1. "Why rouille?" wasn't answerable → fixed with 23 lines (commits in semantic index)
2. Built distill (500 lines) → realized it duplicated indexed content
3. Explored fancy typed graph → grounded back to simplicity
4. Realized: we built great extraction for code/git, now need same for knowledge
5. Key insight: Surface is the **next level up** in the distillation path

**The pattern:**
- Git + code → scrape → eventlog → scry (we built this)
- Sessions → scrape → eventlog → scry (we added this)
- Knowledge → **surface** → scry (this is next)

Surface is to understanding what eventlog is to facts.
