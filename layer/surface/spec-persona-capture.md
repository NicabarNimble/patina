# Spec: Persona Capture

## Overview
Persona capture records cross-project beliefs and patterns that belong to YOU, not to any specific project. These live in `~/.patina/persona/` and are queryable by all projects.

## Persona vs Session
| Aspect | Session | Persona |
|--------|---------|---------|
| Location | `<project>/.patina/events/` | `~/.patina/persona/events/` |
| Scope | Project-specific | Cross-project |
| Example | "This project uses ECS" | "I prefer ECS for game engines" |
| Ownership | Git-tracked, portable | Personal, machine-local |
| Triggers | `/session-note`, `/session-end` | `patina persona note` |

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
        event_id: format!("persona_{}_{:03}", date_str(), next_seq),
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
  "event_id": "persona_20251121_001",
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
│   │   ├── persona_20251121_001-belief-captured.json
│   │   └── ...
│   ├── beliefs.db              # Materialized state
│   └── beliefs.usearch         # Vector index
└── ...
```

## Integration with Projects

Projects can query persona when local knowledge is insufficient:

```rust
// In project query flow
let local_results = query_local(&query)?;
if local_results.is_empty() || local_results[0].score < 0.7 {
    let persona_results = query_persona_service(&query)?;
    // Tag results as [PERSONA]
}
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
- [ ] Beliefs are queryable via mothership `/persona/query` endpoint
