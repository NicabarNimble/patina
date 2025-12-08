# Spec: Persona Capture

**Status:** NOT IMPLEMENTED
**Phase:** 4 (Core Infrastructure)
**Blocked until:** `patina serve` complete

---

## Overview
Persona captures YOUR cross-project beliefs and patterns - knowledge that belongs to you, not to any specific project. Lives in `~/.patina/persona/` and is **never git-tracked**.

**Key principle:** Personas are personal. Different developers have different beliefs, preferences, and mental models. These shouldn't be shared via git.

**Source:** Beliefs flow from **Patina projects** (code you work on). Reference repos are for learning patterns, not capturing beliefs.

## Persona vs Project Knowledge

| Aspect | Project Knowledge | Persona |
|--------|-------------------|---------|
| Location | `<project>/.patina/events/` | `~/.patina/persona/events/` |
| Scope | Project-specific | Cross-project |
| Example | "This project uses ECS" | "I prefer ECS for game engines" |
| Shared | Yes (git-tracked) | **No (personal, machine-local)** |
| Queried via | `patina scry` (project) | `patina scry` ([PERSONA] tag) |
| Triggers | `/session-note`, `scrape` | `patina persona note` |

## Components

### 1. Persona Note Command
**Command:** `patina persona note "insight"`

**Location:** `src/commands/persona/note.rs`

**Usage:**
```bash
# Capture cross-project belief
patina persona note "Always use Result<T,E> over panics in Rust"

# With domain tags
patina persona note --domains rust,error-handling "Prefer explicit error types"

# With reliability score
patina persona note --reliability 0.95 "ECS is better than OOP for games"
```

**Implementation:**
```rust
// src/commands/persona/note.rs
pub fn persona_note(content: &str, domains: Vec<String>, reliability: f32) -> Result<()> {
    let persona_dir = dirs::home_dir()?.join(".patina/persona/events");
    fs::create_dir_all(&persona_dir)?;

    let manifest = read_persona_manifest(&persona_dir)?;
    let next_seq = manifest.last_sequence.global + 1;

    let event = Event {
        event_id: format!("evt_{}_{:03}", date_str(), next_seq),
        event_type: EventType::BeliefCaptured,
        sequence: Sequence { global: next_seq, client: 0, rebase_generation: 0 },
        source: Source {
            session_id: None,  // Not from a session
            git_commit: None,
            git_branch: None,
            capture_context: "cli".to_string(),
        },
        payload: BeliefPayload {
            content: content.to_string(),
            domains,
            reliability,
            supersedes: None,  // For belief updates
        },
        ..
    };

    write_event(&persona_dir, &event)?;
    update_manifest(&persona_dir, next_seq)?;

    println!("Captured persona belief: {}", event.event_id);
    Ok(())
}
```

### 2. Persona Event Types
```rust
pub enum PersonaEventType {
    BeliefCaptured,      // New belief/pattern
    BeliefSuperseded,    // Belief updated/refined
    BeliefRetracted,     // Belief no longer held
    ProjectRegistered,   // New project added
    ProjectUnregistered, // Project removed
}
```

**Event Schema:**
```json
{
  "schema_version": "1.0.0",
  "event_id": "evt_20251121_001",
  "event_type": "belief_captured",
  "timestamp": "2025-11-21T06:32:00Z",
  "sequence": { "global": 1, "client": 0, "rebase_generation": 0 },
  "source": {
    "capture_context": "cli",
    "working_project": "patina"  // Optional: where you were when captured
  },
  "payload": {
    "content": "Always use Result<T,E> over panics in Rust",
    "domains": ["rust", "error-handling"],
    "reliability": 0.95,
    "supersedes": null
  }
}
```

### 3. Persona Materializer
**Command:** `patina persona materialize`

**Location:** `src/commands/persona/materialize.rs`

**What it does:**
1. Read `~/.patina/persona/events/manifest.json`
2. Process events since last materialization
3. Insert into `~/.patina/persona/beliefs.db`
4. Generate embeddings, insert into `~/.patina/persona/beliefs.usearch`

**Handles supersession:**
- When `belief_superseded` event arrives, mark old belief inactive
- When `belief_retracted` event arrives, mark belief retracted
- Query only active beliefs by default

### 4. Persona Query (CLI)
**Command:** `patina persona query "search term"`

**Usage:**
```bash
patina persona query "error handling"
patina persona query --domains rust "patterns"
patina persona query --limit 5 "architecture"
```

**Output:**
```
[0.89] Always use Result<T,E> over panics in Rust
       domains: rust, error-handling
       captured: 2025-11-21

[0.82] Prefer explicit error types over String errors
       domains: rust, error-handling
       captured: 2025-11-20
```

### 5. Persona List
**Command:** `patina persona list`

**Usage:**
```bash
patina persona list                    # All beliefs
patina persona list --domains rust     # Filter by domain
patina persona list --recent 10        # Most recent
```

## Directory Structure
```
~/.patina/
├── persona/
│   ├── events/
│   │   ├── manifest.json
│   │   ├── 2025-11-21-001-belief-captured.json
│   │   └── ...
│   ├── beliefs.db              # Materialized state
│   └── beliefs.usearch         # Vector index
└── ...
```

## Integration with Scry

Persona is queried via `patina scry` which combines project + persona results:

```bash
patina scry "error handling patterns"
# Returns:
# [PROJECT] TypeScript prefers Result types here
# [PERSONA] Always use Result<T,E> over panics in Rust
# [PERSONA] Prefer explicit error types over String
```

Persona results are tagged `[PERSONA]` and ranked slightly lower than project-specific knowledge (0.95x similarity penalty) since project context is more relevant.

**Via mothership API:**
```bash
curl -X POST localhost:50051/scry \
  -d '{"query": "error handling", "include_persona": true}'
```

## CLI Subcommands
```rust
// src/commands/persona/mod.rs
pub enum PersonaCommand {
    Note { content: String, domains: Vec<String>, reliability: Option<f32> },
    Query { query: String, domains: Vec<String>, limit: Option<usize> },
    List { domains: Vec<String>, recent: Option<usize> },
    Materialize { full: bool },
}
```

## Acceptance Criteria
- [ ] `patina persona note "belief"` creates event in `~/.patina/persona/events/`
- [ ] `patina persona materialize` processes events into SQLite + USearch
- [ ] `patina persona query "term"` returns matching beliefs
- [ ] `patina persona list` shows all captured beliefs
- [ ] `patina scry` includes persona results tagged `[PERSONA]`
- [ ] Beliefs queryable via mothership `/scry` and `/persona/query` endpoints
- [ ] Persona data never appears in git (all under ~/.patina/)
