# Spec: Persona

**Status:** Complete (2025-12-08)
**Phase:** 4d (Core Infrastructure)
**Location:** `src/commands/persona/mod.rs`

---

## What Persona IS

A learned model of the user that enables LLMs to respond as the user would want.

**Goal:** When user says "build me a website", LLM already knows their stack, style, and approach—starts informed, asks about unknowns.

**Includes:**
- Beliefs (preferences, principles)
- Knowledge (facts about user, their history)
- Style (how they code, communicate, think)
- Context (what they're working on, what they've done)

**Key principle:** Persona is personal. Lives in `~/.patina/personas/` and is **never git-tracked**. Designed for multi-persona future (one user, multiple contexts).

**Key principle:** Persona is continuously refined, not captured once. Every session potentially adds knowledge.

**Key principle:** Supersedes enables evolution. Without it, old wrong knowledge pollutes forever; with it, refined understanding replaces the old.

## Persona vs Project Knowledge

| Aspect | Project Knowledge | Persona |
|--------|-------------------|---------|
| Location | `<project>/.patina/` | `~/.patina/personas/default/` |
| Scope | Project-specific | Cross-project |
| Example | "This project uses ECS" | "I prefer ECS for game engines" |
| Shared | Yes (via layer/) | **No (personal, machine-local)** |
| Queried via | `patina scry` | `patina scry` ([PERSONA] tag) |

---

## Capture Paths

Persona is continuously refined via multiple capture mechanisms. All paths feed the same store, tagged by source.

### 1. Reflection Flow (`/persona-start`)
Dedicated Q&A distillation session. LLM reviews observations, asks strategic questions, codifies validated knowledge.
- Highest signal, highest friction
- **WHY separate:** Work flow is chaotic pair-programming—don't interrupt it. Reflection is a different mental mode.
- Uses existing Prolog validation infrastructure (`src/reasoning/engine.rs`)

### 2. Session Observation
Patterns captured during `/session-*` work flow.
- Raw observations during work (decisions, challenges, patterns)
- Tagged for later distillation
- Low friction, doesn't break flow

### 3. Session Distillation
Post-session processing: scrape sessions → extract persona-worthy knowledge.
- Automated extraction from session history
- Can run batch (nightly, on-demand)
- Bridges session observations → persona

### 4. Direct Capture
Explicit user statements via CLI.
```bash
patina persona note "prefer Result<T,E> over panics" --domains rust
patina persona note "worked at Company X on distributed systems"
```
- Lowest friction for explicit knowledge
- No validation required

---

## Storage

```
~/.patina/
├── personas/
│   └── default/                    # Default persona (multi-persona future)
│       ├── events/                 # Append-only capture
│       │   ├── manifest.json
│       │   └── *.json              # Tagged by source
│       │
│       ├── materialized/           # Processed views
│       │   ├── persona.db          # SQLite
│       │   └── persona.usearch     # Vector index
│       │
│       └── config.yaml             # Persona settings
│
├── registry.yaml
└── repos/
```

**Event Schema:**
```json
{
  "id": "evt_a1b2c3d4e5f6",
  "event_type": "knowledge_captured",
  "timestamp": 1733676600,
  "source": "direct",
  "content": "prefer Result<T,E> over panics",
  "domains": ["rust", "error-handling"],
  "working_project": "patina",
  "supersedes": null
}
```

**Source tags:** `reflection`, `session`, `distillation`, `direct`

---

## Commands

### Capture
```bash
patina persona note "content" [--domains X,Y] [--supersedes ID]
```

### Query
```bash
patina persona query "search term"
patina persona query --domains rust "patterns"
patina persona list
patina persona list --domains rust --recent 10
```

### Processing
```bash
patina persona materialize          # Process events → db + vectors
patina persona distill              # Extract from recent sessions
```

---

## Integration

### Scry Integration
```bash
patina scry "error handling"
# [PROJECT] src/error.rs - custom Result type
# [PERSONA] prefer Result<T,E> over panics (rust)
```

Persona results tagged `[PERSONA]`, can be filtered with `--no-persona`.

### Mothership Integration
```bash
curl -X POST localhost:50051/api/scry \
  -d '{"query": "error handling", "include_persona": true}'
```

### LLM Integration
**Layer 1 (current):** Adapters (CLAUDE.md, etc.) tell the LLM about persona tools:
- "Use `patina persona query` for user preferences on this domain"
- "Query persona before making architectural decisions"

**Layer 2 (future):** Orchestration agent that knows patina system, makes intelligent routing decisions about what to query.

---

## Implementation

**Core:** `src/commands/persona/mod.rs` - follows scrape/sessions pattern (single file, private internals, public results).

**Available for reflection flow (future):**
- `src/reasoning/engine.rs` - Scryer Prolog integration
- `src/storage/beliefs.rs` - structured belief storage

---

## Acceptance Criteria

- [x] `~/.patina/personas/default/` structure created
- [x] `patina persona note` captures events (tagged `direct`)
- [x] `patina persona materialize` processes events → SQLite + USearch
- [x] `patina persona query` returns semantic search results
- [x] `patina persona list` shows captured knowledge
- [x] `patina scry` includes persona results tagged `[PERSONA]`
- [x] `/api/scry` supports `include_persona` option
- [x] Persona data never appears in git

---

## Future: Multi-Persona

Design supports future multi-persona capability:
```bash
patina persona create work-gamedev
patina persona switch work-gamedev
# Project config can specify persona
```

Not implemented now, but storage structure accommodates it.
