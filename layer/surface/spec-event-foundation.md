# Spec: Event Foundation

## Overview
Events are the source of truth for all Patina knowledge. They live in git, are portable, and materialize into queryable state (SQLite + USearch).

## Current State
- Test events exist: `.patina/events/*.json`
- Manifest exists: `.patina/events/manifest.json`
- Schema follows LiveStore sequence model: `{ global, client, rebase_generation }`

## Components

### 1. Event Emitter
**Location:** `.claude/bin/session-end.sh` (calls Rust) or `src/commands/events/emit.rs`

**Triggers:**
- `/session-end` → emits session summary event
- `/session-note` → emits observation event
- Future: automatic git commit hooks

**Event Schema:**
```json
{
  "schema_version": "1.0.0",
  "event_id": "evt_20251121_004",
  "event_type": "observation_captured",
  "timestamp": "2025-11-21T06:32:00Z",
  "sequence": {
    "global": 4,
    "client": 0,
    "rebase_generation": 0
  },
  "source": {
    "session_id": "20251121-063228",
    "git_commit": "bf22318e",
    "git_branch": "neuro-symbolic-knowledge-system"
  },
  "payload": {
    "content": "...",
    "observation_type": "pattern|decision|challenge|technology",
    "domains": ["architecture", "embeddings"],
    "code_refs": [{ "file": "src/main.rs", "line": 42, "context": "..." }],
    "reliability": 0.95
  }
}
```

**Event Types:**
- `observation_captured` - insights, patterns, decisions
- `session_started` - session boundary marker
- `session_ended` - session summary with git stats

**Implementation:**
```rust
// src/commands/events/mod.rs
pub mod emit;

// src/commands/events/emit.rs
pub fn emit_event(event_type: EventType, payload: Value) -> Result<()> {
    let manifest = read_manifest()?;
    let next_seq = manifest.last_sequence.global + 1;

    let event = Event {
        event_id: format!("evt_{}_{:03}", date_str(), next_seq),
        event_type,
        sequence: Sequence { global: next_seq, client: 0, rebase_generation: 0 },
        source: current_source()?,
        payload,
        ..
    };

    write_event(&event)?;
    update_manifest(next_seq)?;
    Ok(())
}
```

### 2. Manifest Tracker
**Location:** `.patina/events/manifest.json`

**Fields:**
- `schema_version` - for migrations
- `project` - project name
- `last_sequence` - LiveStore sequence triple
- `last_materialized_sequence` - for incremental materialization
- `event_count` - total events
- `created_at`, `updated_at` - timestamps

**Operations:**
- Read on emit to get next sequence
- Update after each emit
- Update after materialization

### 3. Materializer Command
**Command:** `patina materialize`

**Location:** `src/commands/materialize.rs`

**What it does:**
1. Read manifest to find `last_materialized_sequence`
2. Read all events since that sequence
3. For each event:
   - Insert into SQLite (`observations` table)
   - Generate embedding via E5-base-v2
   - Insert into USearch index
4. Update `last_materialized_sequence` in manifest

**Integration with existing code:**
- Uses `src/embeddings/onnx.rs` for embeddings
- Uses `src/embeddings/database.rs` for SQLite
- Uses `src/storage/` for USearch

**CLI:**
```bash
patina materialize              # Incremental (only new events)
patina materialize --full       # Rebuild from scratch
patina materialize --dry-run    # Show what would be materialized
```

### 4. Session Backfill
**Command:** `patina backfill sessions`

**Location:** `src/commands/backfill.rs`

**What it does:**
1. Scan `layer/sessions/*.md`
2. Parse frontmatter + content
3. Generate events for each session
4. Emit with synthetic timestamps preserving order

**Considerations:**
- ~290 existing sessions
- Preserve original timestamps from session files
- Extract key sections: Goals, Activity Log, Key Decisions
- Generate `session_ended` events with summary

## File Structure
```
.patina/
├── events/
│   ├── manifest.json
│   ├── 2025-11-21-001-observation-captured.json
│   ├── 2025-11-21-002-observation-captured.json
│   └── ...
└── data/
    ├── observations.db      # Materialized SQLite
    └── observations.usearch # Materialized vectors
```

## Acceptance Criteria
- [ ] `patina emit observation "insight here"` creates event file
- [ ] manifest.json updates with new sequence
- [ ] `patina materialize` processes events into SQLite + USearch
- [ ] `patina query "search term"` finds materialized events
- [ ] Backfill processes all 290 sessions without errors
