---
id: oxidized-knowledge
status: active
created: 2025-08-11
references: []
tags: [architecture, metaphor, core]
---

# Patina - Oxidized Knowledge

**Purpose:** Knowledge accumulation through oxidation - how patterns form, evolve, and persist.

---

## Structure

- **Core** - base metal, immutable and strong (proven patterns)
- **Surface** - active oxidation (evolving work) 
- **Dust** - patina that flaked off (archived wisdom)

## System

- **User** - Oxidizer (adds the oxygen of creativity and vision)
- **LLMs** - Smith (reads and shapes the patina to manifest the oxidizer's vision)
- **Sessions** - Chemical Reactions (data and knowledge accumulation and transformation)
- **Git** - Time (threads that weave together providing patterns)
- **Containers** - Isolation (controlled storage to hold/test/replicate the patina)

## Data Flow

```
User Input → LLM Processing → Session Capture → Git Commit → Pattern Extraction
     ↓            ↓                ↓                ↓              ↓
   Vision    Code Generation   Context Log    History Proof   Layer Update
```

## Layer Management

### Promotion Path
- Surface (new) → Core (proven via repeated success)
- Surface (new) → Dust (failed or deprecated)

### Storage
- **Core**: `layer/core/*.md` - Version controlled, immutable patterns
- **Surface**: `layer/surface/*.md` - Active development, mutable
- **Dust**: `layer/dust/*.md` - Historical reference, searchable

## Integration Points

### Session ↔ Workspace
- Session ID maps to workspace ID
- Git worktree per workspace
- Isolated container per worktree

### Git ↔ Patterns
- Successful merges → Pattern candidates
- Commit messages → Pattern descriptions
- Diff analysis → Pattern content

### LLM ↔ Layers
- Reads: All layers for context
- Writes: Only to surface/
- Enforces: Core patterns in generated code

## Pattern Lifecycle

### Pattern Recognition
- Git diff + Session context → Pattern extraction → Surface storage

### Pattern Validation
- Used in ≥3 successful contexts → Core candidate
- Failed in any context → Dust candidate  
- Explicitly deprecated → Move to dust

## System Properties

- **Isolation**: Each workspace is containerized
- **Reproducibility**: Consistent environments across contexts
- **Traceability**: Git history links to session context
- **Discoverability**: Navigate searches all layers
- **Evolution**: Patterns move between layers based on usage