# Week 1 Kick-off: Event Foundation

**Goal**: Establish event-sourced architecture in 5 days  
**Outcome**: Working `patina materialize` command that rebuilds observations.db from JSON event files

---

## Day 1: Event Schema Design (Wednesday)

### Morning: Schema Design (2-3 hours)

**Task**: Create comprehensive event schema specification

**File to Create**: `docs/event-schema.md`

**Contents**:

```markdown
# Event Schema Specification v1.0.0

## Event File Structure

### Naming Convention
`YYYY-MM-DD-NNN-type.json`

Examples:
- `2025-11-12-001-observation_captured.json`
- `2025-11-12-002-belief_formed.json`

### File Location
`.patina/shared/events/YYYY-MM-DD-NNN-type.json`

### Base Event Structure

All events share this base structure:

\```json
{
  "schema_version": "1.0.0",
  "event_id": "evt_001",
  "event_type": "observation_captured",
  "timestamp": "2025-11-12T10:30:00Z",
  "author": "nicabar",
  "sequence": 1,
  "payload": { ... }
}
\```

### Event Types

#### 1. observation_captured

Captures a new observation (pattern, decision, challenge).

\```json
{
  "schema_version": "1.0.0",
  "event_id": "evt_001",
  "event_type": "observation_captured",
  "timestamp": "2025-11-12T10:30:00Z",
  "author": "nicabar",
  "sequence": 1,
  "payload": {
    "content": "Extracted environment detection to module when complexity grew",
    "observation_type": "pattern",
    "source_type": "session",
    "source_id": "20251107-124740",
    "domains": ["rust", "modularity", "architecture"],
    "reliability": 0.85,
    "metadata": {}
  }
}
\```

#### 2. belief_formed

Records a validated belief.

\```json
{
  "schema_version": "1.0.0",
  "event_id": "evt_042",
  "event_type": "belief_formed",
  "timestamp": "2025-11-12T14:20:00Z",
  "author": "nicabar",
  "sequence": 42,
  "payload": {
    "belief_statement": "We use Result<T,E> for error handling",
    "supporting_observation_ids": ["obs_001", "obs_017", "obs_023"],
    "confidence": 0.92,
    "validation_method": "prolog",
    "metadata": {
      "evidence_count": 12,
      "cross_project": true
    }
  }
}
\```

### Field Definitions

- `schema_version`: Semantic version (enables schema evolution)
- `event_id`: Unique identifier (e.g., "evt_001")
- `event_type`: Type of event (observation_captured, belief_formed, etc.)
- `timestamp`: ISO 8601 UTC timestamp
- `author`: Git author (from git config user.name)
- `sequence`: Auto-incrementing sequence number (for ordering)
- `payload`: Event-specific data

### Payload Types

#### ObservationPayload
- `content`: The observation text
- `observation_type`: pattern, decision, challenge, technology
- `source_type`: session, commit, manual
- `source_id`: Source identifier (session file, commit hash, etc.)
- `domains`: Array of domain tags (lowercase, hyphenated)
- `reliability`: Confidence in observation (0.0-1.0)
- `metadata`: Optional additional data

#### BeliefPayload
- `belief_statement`: The belief text
- `supporting_observation_ids`: Array of observation IDs
- `confidence`: Belief confidence (0.0-1.0)
- `validation_method`: How belief was validated (prolog, user, consensus)
- `metadata`: Optional additional data
```

**Validation Rules**:
```rust
// Add to docs/event-schema.md

## Validation Rules

1. Event files must be valid JSON
2. All required fields must be present
3. `schema_version` must match supported versions (currently: "1.0.0")
4. `event_id` must be unique across all events
5. `sequence` must be auto-incrementing
6. `timestamp` must be valid ISO 8601
7. `domains` must be lowercase, hyphenated, 2-50 chars
8. `reliability` must be 0.0-1.0
```

**Action**: Create this file, review, commit

---

### Afternoon: Core Event Structures (2-3 hours)

**Task**: Implement Rust types for events

**File to Create**: `src/storage/events.rs`

```rust
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Base event structure (all events share this)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub schema_version: String,
    pub event_id: String,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub author: String,
    pub sequence: u32,
    pub payload: serde_json::Value,
}

/// Typed observation payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationPayload {
    pub content: String,
    pub observation_type: String,
    pub source_type: String,
    pub source_id: String,
    pub domains: Vec<String>,
    pub reliability: f32,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Typed belief payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefPayload {
    pub belief_statement: String,
    pub supporting_observation_ids: Vec<String>,
    pub confidence: f32,
    pub validation_method: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Read all event files from directory
pub fn read_events(events_dir: &Path) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    
    for entry in std::fs::read_dir(events_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let content = std::fs::read_to_string(&path)?;
            let event: Event = serde_json::from_str(&content)?;
            events.push(event);
        }
    }
    
    // Sort by sequence number
    events.sort_by_key(|e| e.sequence);
    
    Ok(events)
}

/// Read events since a given event_id (for incremental processing)
pub fn read_events_since(events_dir: &Path, last_event_id: Option<String>) -> Result<Vec<Event>> {
    let all_events = read_events(events_dir)?;
    
    if let Some(last_id) = last_event_id {
        // Find position of last processed event
        if let Some(pos) = all_events.iter().position(|e| e.event_id == last_id) {
            return Ok(all_events.into_iter().skip(pos + 1).collect());
        }
    }
    
    Ok(all_events)
}

/// Write event file
pub fn write_event_file(events_dir: &Path, event: &Event) -> Result<PathBuf> {
    // Generate filename
    let date = event.timestamp.format("%Y-%m-%d");
    let filename = format!(
        "{}-{:03}-{}.json",
        date, event.sequence, event.event_type
    );
    let path = events_dir.join(filename);
    
    // Write JSON (pretty-printed for git diffs)
    let json = serde_json::to_string_pretty(event)?;
    std::fs::write(&path, json)?;
    
    Ok(path)
}

/// Generate next sequence number
pub fn get_next_sequence(events_dir: &Path) -> Result<u32> {
    let events = read_events(events_dir)?;
    
    if let Some(last_event) = events.last() {
        Ok(last_event.sequence + 1)
    } else {
        Ok(1)
    }
}

/// Validate event structure
pub fn validate_event(event: &Event) -> Result<()> {
    // Check schema version
    if event.schema_version != "1.0.0" {
        anyhow::bail!("Unsupported schema version: {}", event.schema_version);
    }
    
    // Check required fields
    if event.event_id.is_empty() {
        anyhow::bail!("event_id cannot be empty");
    }
    
    if event.event_type.is_empty() {
        anyhow::bail!("event_type cannot be empty");
    }
    
    // Validate based on event type
    match event.event_type.as_str() {
        "observation_captured" => {
            let _payload: ObservationPayload = serde_json::from_value(event.payload.clone())?;
            // Could add more specific validation here
        }
        "belief_formed" => {
            let _payload: BeliefPayload = serde_json::from_value(event.payload.clone())?;
        }
        _ => {
            anyhow::bail!("Unknown event type: {}", event.event_type);
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_event_serialization() {
        let event = Event {
            schema_version: "1.0.0".to_string(),
            event_id: "evt_001".to_string(),
            event_type: "observation_captured".to_string(),
            timestamp: Utc::now(),
            author: "test".to_string(),
            sequence: 1,
            payload: serde_json::json!({
                "content": "Test observation",
                "observation_type": "pattern",
                "source_type": "manual",
                "source_id": "test",
                "domains": ["test"],
                "reliability": 0.85,
                "metadata": {}
            }),
        };
        
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        
        assert_eq!(event.event_id, deserialized.event_id);
    }
    
    #[test]
    fn test_write_and_read_event() {
        let temp_dir = TempDir::new().unwrap();
        let events_dir = temp_dir.path();
        
        let event = Event {
            schema_version: "1.0.0".to_string(),
            event_id: "evt_001".to_string(),
            event_type: "observation_captured".to_string(),
            timestamp: Utc::now(),
            author: "test".to_string(),
            sequence: 1,
            payload: serde_json::json!({
                "content": "Test",
                "observation_type": "pattern",
                "source_type": "manual",
                "source_id": "test",
                "domains": ["test"],
                "reliability": 0.85,
                "metadata": {}
            }),
        };
        
        write_event_file(events_dir, &event).unwrap();
        let events = read_events(events_dir).unwrap();
        
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, "evt_001");
    }
}
```

**Action**: Create file, run tests, commit

---

## Day 2: Database Schema Updates (Thursday)

### Morning: Schema Migration (2 hours)

**Task**: Update observations.db schema to support event sourcing

**File to Modify**: `src/db/schema.sql` (or create new migration)

```sql
-- Add schema_version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    version TEXT PRIMARY KEY,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Add domains support to observations
ALTER TABLE observations ADD COLUMN domains TEXT DEFAULT '[]'; -- JSON array
ALTER TABLE observations ADD COLUMN content_hash TEXT; -- For deduplication

-- Materialization state tracking
CREATE TABLE IF NOT EXISTS materialize_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Domain catalog (auto-populated during materialize)
CREATE TABLE IF NOT EXISTS domains (
    name TEXT PRIMARY KEY,
    first_seen TIMESTAMP NOT NULL,
    observation_count INTEGER DEFAULT 0,
    project_count INTEGER DEFAULT 0
);

-- Domain relationships (populated during oxidize)
CREATE TABLE IF NOT EXISTS domain_relationships (
    domain_a TEXT NOT NULL,
    domain_b TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    strength REAL NOT NULL,
    discovered_at TIMESTAMP NOT NULL,
    PRIMARY KEY (domain_a, domain_b, relationship_type)
);

-- Extraction state (avoid re-scraping)
CREATE TABLE IF NOT EXISTS extraction_state (
    source_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    source_mtime BIGINT,
    extracted_at TIMESTAMP NOT NULL,
    observation_count INTEGER NOT NULL,
    PRIMARY KEY (source_type, source_id)
);

-- Add unique constraint for deduplication
CREATE UNIQUE INDEX IF NOT EXISTS idx_observations_content_hash 
ON observations(content_hash, source_id);

-- Insert schema version
INSERT INTO schema_version (version) VALUES ('1.0.0');
```

**Action**: Apply migration to test database, verify schema

---

### Afternoon: Materialize Command Stub (3 hours)

**Task**: Create basic materialize command structure

**File to Create**: `src/commands/materialize/mod.rs`

```rust
use anyhow::Result;
use std::path::Path;
use crate::db::SqliteDatabase;
use crate::storage::events::{read_events_since, Event, ObservationPayload};

pub fn execute(force: bool) -> Result<()> {
    let events_dir = Path::new(".patina/shared/events");
    let db_path = ".patina/shared/project.db";
    
    println!("🔨 Materializing observations from events...");
    
    // Ensure events directory exists
    if !events_dir.exists() {
        anyhow::bail!("Events directory not found: {:?}", events_dir);
    }
    
    // Open database
    let db = SqliteDatabase::open(db_path)?;
    
    // Get last materialized event (unless force rebuild)
    let last_event = if !force {
        get_last_materialized_event(&db)?
    } else {
        println!("  • Force rebuild: processing all events");
        None
    };
    
    // Read events since last materialize
    let events = read_events_since(events_dir, last_event)?;
    
    if events.is_empty() {
        println!("  ✓ Already up to date (no new events)");
        return Ok(());
    }
    
    println!("  • Processing {} events", events.len());
    
    // Materialize each event
    let mut processed = 0;
    for event in events {
        match event.event_type.as_str() {
            "observation_captured" => {
                materialize_observation(&db, &event)?;
                processed += 1;
            }
            "belief_formed" => {
                // TODO: Phase 2
                println!("  ⚠ Skipping belief_formed (not yet implemented)");
            }
            _ => {
                println!("  ⚠ Unknown event type: {}", event.event_type);
            }
        }
        
        // Update last materialized marker
        update_last_materialized(&db, &event.event_id)?;
    }
    
    println!("  ✓ Materialized {} observations", processed);
    
    Ok(())
}

fn get_last_materialized_event(db: &SqliteDatabase) -> Result<Option<String>> {
    // Query materialize_state table
    let result = db.query_one(
        "SELECT value FROM materialize_state WHERE key = 'last_event'",
        &[]
    )?;
    
    Ok(result.map(|row| row.get::<_, String>(0).unwrap()))
}

fn update_last_materialized(db: &SqliteDatabase, event_id: &str) -> Result<()> {
    db.execute(
        "INSERT OR REPLACE INTO materialize_state (key, value) VALUES ('last_event', ?1)",
        &[&event_id]
    )?;
    Ok(())
}

fn materialize_observation(db: &SqliteDatabase, event: &Event) -> Result<()> {
    // Parse payload
    let payload: ObservationPayload = serde_json::from_value(event.payload.clone())?;
    
    // Generate observation ID from event
    let obs_id = format!("obs_{}", event.sequence);
    
    // Compute content hash for deduplication
    let content_hash = compute_content_hash(&payload.content);
    
    // Insert observation (ignore if duplicate content_hash + source_id)
    db.execute(
        "INSERT OR IGNORE INTO observations 
         (id, content, content_hash, observation_type, source_type, source_id, 
          domains, reliability, created_at, event_file)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        &[
            &obs_id,
            &payload.content,
            &content_hash,
            &payload.observation_type,
            &payload.source_type,
            &payload.source_id,
            &serde_json::to_string(&payload.domains)?,
            &payload.reliability.to_string(),
            &event.timestamp.to_rfc3339(),
            &format!("{}-{:03}-{}.json", 
                     event.timestamp.format("%Y-%m-%d"),
                     event.sequence,
                     event.event_type),
        ]
    )?;
    
    // Update domain catalog
    for domain in payload.domains {
        update_domain_catalog(db, &domain)?;
    }
    
    Ok(())
}

fn update_domain_catalog(db: &SqliteDatabase, domain: &str) -> Result<()> {
    db.execute(
        "INSERT INTO domains (name, first_seen, observation_count)
         VALUES (?1, CURRENT_TIMESTAMP, 1)
         ON CONFLICT(name) DO UPDATE SET
         observation_count = observation_count + 1",
        &[&domain]
    )?;
    Ok(())
}

fn compute_content_hash(content: &str) -> String {
    use sha2::{Sha256, Digest};
    
    // Normalize content
    let normalized = content
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    
    // Hash
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

**Action**: Create file, wire up in `src/commands/mod.rs`, test with dummy event

---

## Day 3: Integration & Testing (Friday)

### Morning: Wire Up Command (2 hours)

**Task**: Add materialize to CLI

**File to Modify**: `src/main.rs`

```rust
// Add to Commands enum
Commands::Materialize {
    /// Force full rebuild (ignore state)
    #[arg(long)]
    force: bool,
},

// Add to match statement
Commands::Materialize { force } => {
    commands::materialize::execute(force)?;
}
```

**File to Modify**: `src/commands/mod.rs`

```rust
pub mod materialize;
```

**Action**: Build, test help text

---

### Afternoon: Test with Sample Events (3 hours)

**Task**: Create test fixtures and verify materialize works

**Create Test Fixtures**: `tests/fixtures/events/`

```bash
mkdir -p tests/fixtures/events
```

Create 3 test event files:

**File 1**: `tests/fixtures/events/2025-11-12-001-observation_captured.json`
```json
{
  "schema_version": "1.0.0",
  "event_id": "evt_test_001",
  "event_type": "observation_captured",
  "timestamp": "2025-11-12T10:00:00Z",
  "author": "test",
  "sequence": 1,
  "payload": {
    "content": "Use Result<T, E> for error handling",
    "observation_type": "pattern",
    "source_type": "manual",
    "source_id": "test_001",
    "domains": ["rust", "error-handling"],
    "reliability": 0.85,
    "metadata": {}
  }
}
```

**File 2**: `tests/fixtures/events/2025-11-12-002-observation_captured.json`
```json
{
  "schema_version": "1.0.0",
  "event_id": "evt_test_002",
  "event_type": "observation_captured",
  "timestamp": "2025-11-12T10:30:00Z",
  "author": "test",
  "sequence": 2,
  "payload": {
    "content": "Extract to module when complexity grows",
    "observation_type": "pattern",
    "source_type": "manual",
    "source_id": "test_002",
    "domains": ["rust", "modularity", "architecture"],
    "reliability": 0.90,
    "metadata": {}
  }
}
```

**File 3**: `tests/fixtures/events/2025-11-12-003-observation_captured.json`
```json
{
  "schema_version": "1.0.0",
  "event_id": "evt_test_003",
  "event_type": "observation_captured",
  "timestamp": "2025-11-12T11:00:00Z",
  "author": "test",
  "sequence": 3,
  "payload": {
    "content": "Use async for I/O-bound operations",
    "observation_type": "decision",
    "source_type": "manual",
    "source_id": "test_003",
    "domains": ["rust", "async", "performance"],
    "reliability": 0.80,
    "metadata": {}
  }
}
```

**Integration Test**: `tests/integration/materialize_test.rs`

```rust
use anyhow::Result;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_materialize_from_fixtures() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();
    
    // Setup structure
    let events_dir = project_dir.join(".patina/shared/events");
    std::fs::create_dir_all(&events_dir)?;
    
    // Copy test fixtures
    for i in 1..=3 {
        let fixture = PathBuf::from(format!(
            "tests/fixtures/events/2025-11-12-{:03}-observation_captured.json",
            i
        ));
        let dest = events_dir.join(format!(
            "2025-11-12-{:03}-observation_captured.json",
            i
        ));
        std::fs::copy(&fixture, &dest)?;
    }
    
    // Initialize database
    let db_path = project_dir.join(".patina/shared/project.db");
    // ... init schema ...
    
    // Run materialize
    // ... execute materialize command ...
    
    // Verify
    // ... check observations.db has 3 observations ...
    // ... check domains table has 6 domains ...
    
    Ok(())
}
```

**Manual Test**:
```bash
# Setup test environment
mkdir -p /tmp/patina-test/.patina/shared/events
cp tests/fixtures/events/*.json /tmp/patina-test/.patina/shared/events/

# Run materialize
cd /tmp/patina-test
patina materialize

# Verify
sqlite3 .patina/shared/project.db "SELECT COUNT(*) FROM observations"
# Should output: 3

sqlite3 .patina/shared/project.db "SELECT name, observation_count FROM domains"
# Should show: rust (3), error-handling (1), modularity (1), architecture (1), async (1), performance (1)

# Test incremental materialize
patina materialize
# Should output: "Already up to date (no new events)"
```

---

## Day 4: Backup & Fresh Structure (Monday)

### Morning: Backup Script (2 hours)

**File to Create**: `scripts/backup-phase0.sh`

```bash
#!/bin/bash
set -e

echo "🔒 Backing up Phase 0 data..."

BACKUP_DIR=".patina/backups/phase0-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

# Backup databases
if [ -f .patina/db/observations.db ]; then
    cp .patina/db/observations.db "$BACKUP_DIR/"
    echo "  ✓ observations.db"
fi

if [ -f .patina/db/facts.db ]; then
    cp .patina/db/facts.db "$BACKUP_DIR/"
    echo "  ✓ facts.db"
fi

if [ -f .patina/db/code.db ]; then
    cp .patina/db/code.db "$BACKUP_DIR/"
    echo "  ✓ code.db"
fi

# Export observations to JSON
if [ -f .patina/db/observations.db ]; then
    echo "  • Exporting observations to JSON..."
    sqlite3 .patina/db/observations.db <<'SQL' > "$BACKUP_DIR/observations.json"
SELECT json_group_array(
    json_object(
        'id', id,
        'content', content,
        'observation_type', observation_type,
        'source_type', source_type,
        'source_id', source_id,
        'reliability', reliability,
        'created_at', created_at
    )
)
FROM observations;
SQL
    echo "  ✓ observations.json"
fi

# Summary
echo ""
echo "✅ Backup complete: $BACKUP_DIR"
if [ -f "$BACKUP_DIR/observations.db" ]; then
    SIZE=$(du -h "$BACKUP_DIR/observations.db" | cut -f1)
    COUNT=$(sqlite3 "$BACKUP_DIR/observations.db" "SELECT COUNT(*) FROM observations" 2>/dev/null || echo "0")
    echo "   • observations.db ($SIZE, $COUNT observations)"
fi
echo ""
echo "To restore: cp $BACKUP_DIR/*.db .patina/db/"
```

**Action**: Make executable, run, verify backup

---

### Afternoon: Initialize Fresh Structure (2 hours)

**File to Create**: `scripts/init-phase1.sh`

```bash
#!/bin/bash
set -e

echo "🚀 Initializing Phase 1 structure..."

# Create directories
mkdir -p .patina/shared/events
mkdir -p .patina/shared/vectors
mkdir -p .patina/local
echo "  ✓ Created directory structure"

# Initialize fresh database
if [ -f src/db/schema.sql ]; then
    rm -f .patina/shared/project.db
    sqlite3 .patina/shared/project.db < src/db/schema.sql
    echo "  ✓ Initialized project.db"
else
    echo "  ⚠ Warning: src/db/schema.sql not found"
fi

# Update .gitignore
if ! grep -q ".patina/local/" .gitignore 2>/dev/null; then
    cat >> .gitignore << 'EOF'

# Phase 1: Event-sourced structure
.patina/local/
.patina/shared/project.db
.patina/shared/vectors/
EOF
    echo "  ✓ Updated .gitignore"
else
    echo "  • .gitignore already configured"
fi

echo ""
echo "✅ Phase 1 structure ready"
echo "   • .patina/shared/events/ (git-tracked)"
echo "   • .patina/shared/project.db (gitignored)"
echo "   • .patina/local/ (gitignored)"
echo ""
echo "Next: Run 'patina materialize' after adding events"
```

**Action**: Make executable, run, verify structure

---

## Day 5: Documentation & Review (Tuesday)

### Morning: Command Documentation (2 hours)

**Update**: `README.md`

Add section on event-sourced workflow:

```markdown
## Event-Sourced Workflow (Phase 1)

Patina uses event sourcing for knowledge management:

```
Work → Events (JSON) → Materialize → Database → Oxidize → Vectors
```

### Commands

**Scrape** (extract observations → events):
```bash
patina scrape sessions    # Extract from layer/sessions/*.md
patina scrape git         # Extract from git history
```

**Materialize** (rebuild database from events):
```bash
patina materialize        # Incremental (new events only)
patina materialize --force # Full rebuild
```

**Oxidize** (generate vectors + discover relationships):
```bash
patina oxidize            # Generate embeddings, discover domain relationships
```

**Query** (semantic search):
```bash
patina query semantic "error handling patterns"
```

### File Structure

```
.patina/
├── shared/              # Team knowledge (in git)
│   ├── events/          # Immutable event log (JSON)
│   ├── project.db       # Materialized (gitignored)
│   └── vectors/         # USearch indices (gitignored)
└── local/               # Scratch space (gitignored)
```
```

**Action**: Update README, commit

---

### Afternoon: Week 1 Review (2 hours)

**Checklist**:

- [ ] Event schema documented (`docs/event-schema.md`)
- [ ] Event types implemented (`src/storage/events.rs`)
- [ ] Schema updated (domains, materialize_state, etc.)
- [ ] Materialize command working (`src/commands/materialize/`)
- [ ] CLI integration complete (`src/main.rs`)
- [ ] Test fixtures created (`tests/fixtures/events/`)
- [ ] Manual test passes (3 events → 3 observations)
- [ ] Backup script ready (`scripts/backup-phase0.sh`)
- [ ] Init script ready (`scripts/init-phase1.sh`)
- [ ] Documentation updated (`README.md`, `docs/event-schema.md`)

**Test End-to-End**:
```bash
# 1. Backup existing data
./scripts/backup-phase0.sh

# 2. Initialize Phase 1 structure
./scripts/init-phase1.sh

# 3. Copy test fixtures
cp tests/fixtures/events/*.json .patina/shared/events/

# 4. Materialize
cargo build --release
cargo install --path .
patina materialize

# 5. Verify
sqlite3 .patina/shared/project.db "SELECT COUNT(*) FROM observations"
sqlite3 .patina/shared/project.db "SELECT * FROM domains"

# 6. Test incremental
patina materialize  # Should say "already up to date"
```

**Review Meeting**:
- Demo materialize command
- Show event files → observations.db flow
- Discuss any challenges
- Plan Week 2 (session scraping)

---

## Week 1 Deliverables

By end of Day 5 (Tuesday), you should have:

✅ **Event Schema**:
- `docs/event-schema.md` with v1.0.0 spec
- `src/storage/events.rs` with Rust types

✅ **Materialize Command**:
- `patina materialize` working
- Incremental processing (tracks last event)
- Force rebuild option

✅ **Database**:
- Schema updated with domains, materialize_state
- Test database with 3 sample observations

✅ **Scripts**:
- `scripts/backup-phase0.sh` (backup existing data)
- `scripts/init-phase1.sh` (initialize Phase 1 structure)

✅ **Tests**:
- Test fixtures in `tests/fixtures/events/`
- Manual test passes
- Integration test stub

✅ **Documentation**:
- Event schema documented
- README updated with workflow
- Commands documented

---

## Next: Week 2 Preview

With event foundation in place, Week 2 focuses on **session scraping**:

1. **Parse** `layer/sessions/*.md` files
2. **Extract** observations from activity logs
3. **Tag** domains via LLM (batched)
4. **Create** event files for all 266 sessions
5. **Materialize** → observations.db

By end of Week 2, you'll have ~542 observations extracted from real session data.

---

**Let's get started!** Begin with Day 1 (event schema) and work through methodically. Each day builds on the previous, so don't skip ahead.

Good luck! 🚀
