---
id: oxidized-knowledge
status: active
created: 2025-08-11
updated: 2026-02-12
references: [session-capture, adapter-pattern]
tags: [architecture, metaphor, core]
---

# Patina - Oxidized Knowledge

**Purpose:** Knowledge accumulation through oxidation - how patterns form, evolve, and persist across projects and personas.

---

## Knowledge Separation

Patina distinguishes two types of knowledge:

| Type | Location | Shared? | Contains |
|------|----------|---------|----------|
| **Project** | `layer/` + `.patina/` | Yes (layer/ in git) | Facts, beliefs, and specs about this codebase |
| **Persona** | `~/.patina/layer/` | No (personal) | Cross-project beliefs and preferences |

**Project knowledge:** "TypeScript prefers Result types here" - observation about livestore
**Personal belief:** "I prefer Rust Result<T,E> over exceptions" - your opinion across all projects

Different developers working on the same project share project beliefs but keep separate persona beliefs. Project beliefs live in `layer/surface/epistemic/beliefs/` (git-tracked). Persona beliefs live in `~/.patina/layer/surface/beliefs/` (machine-local).

## Structure

### Project Layers (`layer/`)
- **Core** - base metal, immutable and strong (proven patterns)
- **Surface** - active oxidation (evolving work, specs)
- **Dust** - patina that flaked off (archived wisdom)

### Project Data (`.patina/`)
- **data/** - materialized SQLite + vectors (local, rebuilt from git/sessions)
- **oxidize.yaml** - recipe for building adapters (git-tracked)

### Personal (`~/.patina/`)
- **layer/** - user-level beliefs and preferences (mirror of project layer structure)
  - `layer/surface/beliefs/` - persona beliefs (machine-local, never shared)
- **personas/** - legacy event log (being migrated to layer/beliefs)
- **registry.yaml** - registered projects and repos on this machine
- **cache/repos/** - cloned reference repositories

## System

- **User** - Oxidizer (adds the oxygen of creativity and vision)
- **LLMs** - Smith (reads project + persona knowledge via scry)
- **Sessions** - Chemical Reactions (capture observations → events)
- **Git** - Time (threads that weave together, syncs project knowledge)
- **Containers** - Isolation (controlled storage to hold/test/replicate)

## Data Flow

```
Sources                    Pipeline                      Storage
─────────────────────────────────────────────────────────────────────
.git/                  ┐
layer/sessions/*.md    ├→ scrape → patina.db (eventlog + views)
src/**/*               ┘              │
                                      ↓
                                   oxidize
                                      │
                                      ↓
                       ┌────────── vectors
                       │
                       ↓
~/.patina/layer/   ──→ scry ←── .patina/data/
                       │
                       ↓
              [PROJECT] + [PERSONA] results → LLM context
```

## Layer Management

### Promotion Path (Project Patterns)
- Surface (new) → Core (proven via repeated success)
- Surface (new) → Dust (failed or deprecated)

### Storage
- **Core**: `layer/core/*.md` - Version controlled, immutable patterns
- **Surface**: `layer/surface/*.md` - Active development, specs, mutable
- **Dust**: `layer/dust/*.md` - Historical reference, searchable

## Integration Points

### Session → Events
- Session markdown → scrape sessions → observations table
- Git commits → scrape git → commits + co_changes tables
- Code AST → scrape code → functions + call_graph tables

### Events → Vectors
- oxidize.yaml recipe defines projections
- SQLite tables provide training pairs
- Each user builds vectors locally from shared recipe

### Scry → LLM
- Query searches both project vectors and persona beliefs
- Results tagged [PROJECT] or [PERSONA]
- LLM sees unified context with clear provenance

### Persona (Personal Only)
- `patina persona note "belief"` writes to `~/.patina/layer/surface/beliefs/` and legacy event log
- `patina persona query "topic"` searches persona knowledge
- `patina persona migrate` converts legacy events to belief files (idempotent)
- Never synced via git - machine-local only
- Cross-project: same belief applies to all your work

## Pattern Lifecycle

### Pattern Recognition (Project)
- Git diff + Session context → scrape → Events → Pattern extraction

### Pattern Validation (Project)
- Used in ≥3 successful contexts → Core candidate
- Failed in any context → Dust candidate
- Explicitly deprecated → Move to dust

### Belief Evolution (Persona)
- Personal beliefs accumulate over time
- Not subject to project validation
- Inform how you approach all projects

## System Properties

- **Isolation**: Project knowledge stays in project, persona stays personal
- **Reproducibility**: Same recipe + events = equivalent vectors
- **Traceability**: Git history links to session context
- **Discoverability**: Scry searches project + persona together
- **Evolution**: Project patterns move between layers; persona beliefs persist
