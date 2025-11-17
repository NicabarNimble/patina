# Phase 1 Implementation Plan: Event-Sourced Knowledge System

**Status**: Ready for Implementation  
**Created**: 2025-11-12  
**Reviewer**: Claude (Sonnet 4.5)  
**Goal**: Transform Patina from direct-write SQLite to event-sourced architecture with domains as emergent tags

---

## Executive Summary

### What We're Building
An event-sourced knowledge system where observations flow through immutable event files (JSON in git) that get materialized into queryable databases. Domains emerge automatically through LLM tagging during scrape, eliminating rigid ontological hierarchies.

### Why This Matters
1. **Time Travel**: Replay events to any point, change validation rules, rebuild beliefs
2. **Auditability**: Git log shows complete provenance chain
3. **Flexibility**: Schema evolution without data migration
4. **Collaboration**: Events in git = reviewable via PRs, mergeable via git
5. **Organic Growth**: Domains emerge from actual work, not upfront design

### Current State vs Target
```
NOW: Work → observations.db (direct write) → vectors
TARGET: Work → events/ (JSON) → observations.db (materialized) → vectors
```

---

## Architectural Review & Findings

### ✅ What's Already Working (Don't Touch)

1. **Neuro-Symbolic Reasoning** (`src/reasoning/`)
   - Scryer Prolog embedded in Rust
   - Dynamic fact injection working
   - Validation rules in `.patina/validation-rules.pl`
   - 94 tests passing
   - **Action**: Keep as-is, use in Phase 1D

2. **Embeddings & Vector Search** (`src/embeddings/`, `src/storage/`)
   - ONNX Runtime + all-MiniLM-L6-v2 (INT8 quantized)
   - USearch HNSW indices working
   - Metal GPU acceleration
   - **Action**: Rename `embeddings` command → `oxidize` in Phase 1D

3. **Code Indexing** (`src/commands/scrape/code.rs`)
   - Tree-sitter parsing via `patina-metal`
   - SQLite structure indexing
   - Incremental updates working
   - **Action**: Keep separate from observations (different concern)

4. **Session Format** (`layer/sessions/*.md`)
   - 266 Obsidian-compatible markdown files
   - Structured activity logs
   - **Action**: Parse in Phase 1B

### 🔧 What Needs Transformation

1. **Storage Architecture**
   ```
   Current: .patina/db/observations.db (direct write)
   Target:  .patina/shared/events/*.json → .patina/shared/project.db (materialized)
   ```

2. **Scrape Commands**
   ```
   Current: embeddings generate does extraction + vectorization
   Target:  scrape (extraction → events) + oxidize (vectorization) separate
   ```

3. **Schema** (observations.db)
   ```sql
   Current: No domains field, no extraction tracking
   Target:  Add domains JSON field, add extraction_state table, add domain_relationships
   ```

---

## Phase 1 Implementation: 4-Week Breakdown

### Week 1: Event Foundation (Phase 1A)

**Goal**: Establish event-sourced architecture with working materialize command

#### Tasks

**1A.1: Design Event Schema** (2 hours)
```json
{
  "event_id": "evt_001",
  "event_type": "observation_captured",
  "timestamp": "2025-11-12T10:30:00Z",
  "author": "nicabar",
  "sequence": 1,
  "payload": {
    "content": "Extracted environment detection to module",
    "observation_type": "pattern",
    "source_type": "session",
    "source_id": "20251107-124740",
    "domains": ["rust", "modularity", "architecture"],
    "reliability": 0.85,
    "metadata": {}
  }
}
```

**Event Types**:
- `observation_captured` - Pattern, decision, challenge captured
- `belief_formed` - Validated belief created
- `belief_strengthened` - Additional evidence added
- `belief_contextualized` - Context-specific override

**Naming Convention**: `YYYY-MM-DD-NNN-type.json`
- Example: `2025-11-12-001-observation_captured.json`
- Lexicographically ordered for sequential replay

**Files to Create**:
- `src/storage/events.rs` - Event file handling
- `docs/event-schema.md` - Full specification
- `tests/fixtures/events/` - Test event files

---

**1A.2: Implement Materialize Command** (8 hours)

**Core Algorithm**:
```rust
// src/commands/materialize/mod.rs

pub fn execute(force: bool) -> Result<()> {
    let events_dir = Path::new(".patina/shared/events");
    let db_path = ".patina/shared/project.db";
    
    // Read last materialized event
    let last_event = if !force {
        read_last_materialized(db_path)?
    } else {
        None
    };
    
    // Get all events in order
    let events = read_events_since(events_dir, last_event)?;
    
    // Open/create database
    let db = SqliteDatabase::open(db_path)?;
    
    // Materialize each event
    for event in events {
        match event.event_type.as_str() {
            "observation_captured" => {
                materialize_observation(&db, &event)?;
            }
            "belief_formed" => {
                materialize_belief(&db, &event)?;
            }
            _ => eprintln!("Unknown event type: {}", event.event_type),
        }
        
        // Update last materialized marker
        db.execute(
            "INSERT OR REPLACE INTO materialize_state (key, value) 
             VALUES ('last_event', ?1)",
            &[&event.event_id]
        )?;
    }
    
    Ok(())
}
```

**Schema Updates** (`src/db/schema.sql`):
```sql
-- Add domains support
ALTER TABLE observations ADD COLUMN domains TEXT; -- JSON array

-- Track materialization progress
CREATE TABLE materialize_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Domain catalog (auto-populated during materialize)
CREATE TABLE domains (
    name TEXT PRIMARY KEY,
    first_seen TIMESTAMP NOT NULL,
    observation_count INTEGER DEFAULT 0,
    project_count INTEGER DEFAULT 0
);

-- Domain relationships (populated during oxidize)
CREATE TABLE domain_relationships (
    domain_a TEXT NOT NULL,
    domain_b TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    strength REAL NOT NULL,
    discovered_at TIMESTAMP NOT NULL,
    PRIMARY KEY (domain_a, domain_b, relationship_type)
);

-- Track what's been scraped (avoid re-extraction)
CREATE TABLE extraction_state (
    source_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    source_mtime BIGINT,
    extracted_at TIMESTAMP NOT NULL,
    observation_count INTEGER NOT NULL,
    PRIMARY KEY (source_type, source_id)
);
```

**Files to Create**:
- `src/commands/materialize/mod.rs` - Main command
- `src/storage/events.rs` - Event reading/writing
- `src/db/schema_migrations.rs` - Schema evolution support

**Success Criteria**:
- ✅ `patina materialize` runs without errors
- ✅ `patina materialize --force` does full rebuild
- ✅ Test with 10 sample event files
- ✅ Incremental materialize only processes new events

---

**1A.3: Backup Existing Data** (1 hour)

**Script**: `scripts/backup-phase0.sh`
```bash
#!/bin/bash
set -e

BACKUP_DIR=".patina/backups/phase0-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

# Backup databases
cp .patina/db/observations.db "$BACKUP_DIR/"
cp .patina/db/facts.db "$BACKUP_DIR/"
cp .patina/db/code.db "$BACKUP_DIR/"

# Export observations to JSON
sqlite3 .patina/db/observations.db \
  "SELECT json_object(
     'id', id,
     'content', content,
     'observation_type', observation_type,
     'source_type', source_type,
     'source_id', source_id,
     'reliability', reliability,
     'created_at', created_at
   ) FROM observations" > "$BACKUP_DIR/observations.json"

echo "✅ Backup complete: $BACKUP_DIR"
echo "   • observations.db ($(du -h $BACKUP_DIR/observations.db | cut -f1))"
echo "   • facts.db ($(du -h $BACKUP_DIR/facts.db | cut -f1))"
echo "   • observations.json ($(wc -l < $BACKUP_DIR/observations.json) observations)"
```

**Action**: Run before starting Phase 1B

---

**1A.4: Initialize Fresh Structure** (2 hours)

**Script**: `scripts/init-phase1.sh`
```bash
#!/bin/bash
set -e

# Create new structure
mkdir -p .patina/shared/events
mkdir -p .patina/shared/vectors
mkdir -p .patina/local

# Initialize fresh database with new schema
rm -f .patina/shared/project.db
sqlite3 .patina/shared/project.db < src/db/schema.sql

# Update .gitignore
cat >> .gitignore << 'EOF'

# Phase 1: Event-sourced structure
.patina/local/
.patina/shared/project.db
.patina/shared/vectors/
EOF

echo "✅ Phase 1 structure initialized"
echo "   • .patina/shared/events/ (git-tracked)"
echo "   • .patina/shared/project.db (gitignored)"
echo "   • .patina/local/ (gitignored)"
```

---

**Week 1 Deliverables**:
- [ ] Event schema documented (`docs/event-schema.md`)
- [ ] `patina materialize` command working
- [ ] Existing data backed up
- [ ] Fresh structure created with new schema
- [ ] 10 test events materialize correctly

---

### Week 2: Session Scraping (Phase 1B)

**Goal**: Extract all 266 sessions as event files with auto-tagged domains

#### Tasks

**1B.1: Session Markdown Parser** (6 hours)

**Current Format Analysis** (from `layer/sessions/*.md`):
```markdown
# Session 20251107-124740

## Activity Log
- Implemented X
- Refactored Y to use Z pattern
- Decided against approach A because B

## Observations
- Pattern: Always validate inputs before processing
- Challenge: Performance degraded with large datasets
- Decision: Use async for I/O-bound operations
```

**Parser Implementation**:
```rust
// src/parsers/session_markdown.rs

use anyhow::Result;
use std::path::Path;

pub struct SessionObservation {
    pub content: String,
    pub observation_type: String, // pattern, decision, challenge
    pub source_id: String,         // session timestamp
    pub reliability: f32,
}

pub fn parse_session(path: &Path) -> Result<Vec<SessionObservation>> {
    let content = std::fs::read_to_string(path)?;
    let mut observations = Vec::new();
    
    // Extract session ID from filename
    let session_id = extract_session_id(path)?;
    
    // Parse activity log for implicit observations
    observations.extend(parse_activity_log(&content, &session_id)?);
    
    // Parse explicit observation sections
    observations.extend(parse_observations_section(&content, &session_id)?);
    
    Ok(observations)
}

fn parse_activity_log(content: &str, session_id: &str) -> Result<Vec<SessionObservation>> {
    // Look for ## Activity Log section
    // Extract bullet points
    // Classify based on keywords: "Decided" → decision, "Pattern" → pattern, etc.
    // Return observations with reliability 0.75 (derived from activity)
    todo!()
}

fn parse_observations_section(content: &str, session_id: &str) -> Result<Vec<SessionObservation>> {
    // Look for ## Observations section
    // Parse "Pattern:", "Decision:", "Challenge:" prefixed lines
    // Return observations with reliability 0.85 (explicit)
    todo!()
}
```

**Files to Create**:
- `src/parsers/session_markdown.rs` - Parser implementation
- `tests/parsers/session_markdown_test.rs` - Parser tests with fixtures

---

**1B.2: Domain Tagging via LLM** (4 hours)

**Implementation**:
```rust
// src/adapters/domain_tagger.rs

use anyhow::Result;

pub trait DomainTagger {
    fn tag_domains(&self, observation: &str, context: &ProjectContext) -> Result<Vec<String>>;
}

pub struct ProjectContext {
    pub languages: Vec<String>,  // From code.db
    pub frameworks: Vec<String>, // From code.db
    pub recent_domains: Vec<String>, // From observations.db
}

// Claude implementation
impl DomainTagger for ClaudeAdapter {
    fn tag_domains(&self, observation: &str, context: &ProjectContext) -> Result<Vec<String>> {
        let prompt = format!(r#"
Given this observation: "{}"

Project context:
- Languages: {}
- Frameworks: {}
- Recent domains: {}

Return 2-5 domain tags (lowercase, hyphenated). 
Examples: rust, modularity, error-handling, performance, security

Return ONLY a JSON array: ["domain1", "domain2", ...]
"#, 
            observation,
            context.languages.join(", "),
            context.frameworks.join(", "),
            context.recent_domains.join(", ")
        );
        
        let response = self.query(&prompt)?;
        let domains: Vec<String> = serde_json::from_str(&response)?;
        Ok(domains)
    }
}
```

**Files to Create**:
- `src/adapters/domain_tagger.rs` - Trait + implementations
- `tests/adapters/domain_tagger_test.rs` - Mocked tests

---

**1B.3: Session Scrape Command** (6 hours)

**Implementation**:
```rust
// src/commands/scrape/sessions.rs

use anyhow::Result;
use std::path::Path;

pub fn execute(force: bool) -> Result<ScrapeStats> {
    let sessions_dir = Path::new("layer/sessions");
    let events_dir = Path::new(".patina/shared/events");
    let db_path = ".patina/shared/project.db";
    
    let db = SqliteDatabase::open(db_path)?;
    let tagger = create_domain_tagger()?; // Uses current LLM adapter
    
    let start = std::time::Instant::now();
    let mut extracted_count = 0;
    
    // Get project context for domain tagging
    let context = build_project_context(&db)?;
    
    // Iterate over session files
    for entry in std::fs::read_dir(sessions_dir)? {
        let path = entry?.path();
        if !path.extension().map_or(false, |e| e == "md") {
            continue;
        }
        
        // Check extraction state (skip if already extracted and not modified)
        if !force && is_already_extracted(&db, &path)? {
            continue;
        }
        
        // Parse session
        let observations = parse_session(&path)?;
        
        // Create event file for each observation
        for obs in observations {
            // Auto-tag domains
            let domains = tagger.tag_domains(&obs.content, &context)?;
            
            // Create event
            let event = Event {
                event_id: generate_event_id(),
                event_type: "observation_captured".to_string(),
                timestamp: chrono::Utc::now(),
                author: get_git_author()?,
                sequence: get_next_sequence()?,
                payload: ObservationPayload {
                    content: obs.content,
                    observation_type: obs.observation_type,
                    source_type: "session".to_string(),
                    source_id: obs.source_id,
                    domains,
                    reliability: obs.reliability,
                    metadata: serde_json::json!({}),
                },
            };
            
            // Write event file
            write_event_file(events_dir, &event)?;
            extracted_count += 1;
        }
        
        // Update extraction state
        mark_as_extracted(&db, &path, extracted_count)?;
    }
    
    Ok(ScrapeStats {
        items_processed: extracted_count,
        time_elapsed: start.elapsed(),
        database_size_kb: 0, // Not materialized yet
    })
}
```

**Files to Create/Modify**:
- `src/commands/scrape/sessions.rs` - New scraper
- `src/commands/scrape/mod.rs` - Add sessions subcommand
- `src/main.rs` - Wire up `patina scrape sessions`

---

**1B.4: Extract All Sessions** (2 hours)

**Process**:
```bash
# Run session scrape
patina scrape sessions

# Expected output:
# 🔍 Scanning layer/sessions/...
# [1/266] Extracting 20241101-103045.md... 3 observations
# [2/266] Extracting 20241102-141522.md... 2 observations
# ...
# ✅ Extracted 266 sessions → 542 observations → 542 event files
```

**Verification**:
```bash
# Check event files created
ls -1 .patina/shared/events/ | wc -l  # Should be ~500+

# Check domains applied
sqlite3 .patina/shared/project.db \
  "SELECT domains FROM observations LIMIT 10"

# Sample domain distribution
sqlite3 .patina/shared/project.db \
  "SELECT name, observation_count FROM domains ORDER BY observation_count DESC LIMIT 20"
```

---

**Week 2 Deliverables**:
- [ ] Session parser handles current markdown format
- [ ] Domain auto-tagging working via LLM adapter
- [ ] `patina scrape sessions` command complete
- [ ] All 266 sessions extracted as events
- [ ] Domains catalog populated with 50-100 domains
- [ ] Extraction tracking prevents re-scraping

---

### Week 3: Git Scraping (Phase 1C)

**Goal**: Extract git commit history as event files with deduplication

#### Tasks

**1C.1: Git Commit Parser** (4 hours)

**Implementation**:
```rust
// src/commands/scrape/git.rs

use anyhow::Result;
use std::process::Command;

pub struct GitObservation {
    pub content: String,
    pub observation_type: String,
    pub commit_hash: String,
    pub commit_date: chrono::DateTime<chrono::Utc>,
    pub author: String,
}

pub fn extract_commits() -> Result<Vec<GitObservation>> {
    // Get all commits (not just last 90 days)
    let output = Command::new("git")
        .args(&["log", "--all", "--pretty=format:%H|%an|%ai|%s|%b"])
        .output()?;
    
    let log = String::from_utf8(output.stdout)?;
    let mut observations = Vec::new();
    
    for line in log.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 5 {
            continue;
        }
        
        let commit_hash = parts[0].to_string();
        let author = parts[1].to_string();
        let date_str = parts[2];
        let subject = parts[3];
        let body = parts[4];
        
        // Parse conventional commit format
        let obs_type = classify_commit(subject);
        let content = format_commit_observation(subject, body);
        
        // Skip merge commits and trivial changes
        if should_skip_commit(subject) {
            continue;
        }
        
        observations.push(GitObservation {
            content,
            observation_type: obs_type,
            commit_hash,
            commit_date: parse_git_date(date_str)?,
            author,
        });
    }
    
    Ok(observations)
}

fn classify_commit(subject: &str) -> String {
    // feat:, fix: → decision
    // refactor:, perf: → pattern
    // docs:, test: → skip
    // build:, ci: → decision
    
    if subject.starts_with("feat:") || subject.starts_with("fix:") {
        "decision".to_string()
    } else if subject.starts_with("refactor:") || subject.starts_with("perf:") {
        "pattern".to_string()
    } else {
        "decision".to_string() // Default
    }
}

fn should_skip_commit(subject: &str) -> bool {
    // Skip merges, docs, formatting, generated code
    subject.starts_with("Merge ")
        || subject.starts_with("docs:")
        || subject.starts_with("chore:")
        || subject.contains("formatting")
        || subject.contains("Generated with Claude Code")
}
```

**Files to Create**:
- `src/commands/scrape/git.rs` - Git scraper
- `tests/commands/scrape/git_test.rs` - Parser tests

---

**1C.2: Content Deduplication** (4 hours)

**Strategy**: Hash normalized content to detect duplicates across sources

**Implementation**:
```rust
// src/storage/deduplication.rs

use sha2::{Sha256, Digest};

pub fn compute_content_hash(content: &str) -> String {
    let normalized = normalize_content(content);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn normalize_content(content: &str) -> String {
    content
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// In observations schema
CREATE TABLE observations (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL, -- For deduplication
    -- ... other fields
    UNIQUE(content_hash, source_id) -- Allow same content from different sources
);
```

**Deduplication Logic**:
- Same content from same source → Skip (duplicate)
- Same content from different sources → Keep both (corroboration)
- Similar but not identical → Keep both (variation)

---

**1C.3: Git Scrape Command** (6 hours)

**Implementation**:
```rust
// src/commands/scrape/git.rs (continued)

pub fn execute(force: bool) -> Result<ScrapeStats> {
    let events_dir = Path::new(".patina/shared/events");
    let db_path = ".patina/shared/project.db";
    
    let db = SqliteDatabase::open(db_path)?;
    let tagger = create_domain_tagger()?;
    
    let start = std::time::Instant::now();
    let mut extracted_count = 0;
    let mut skipped_count = 0;
    
    // Get project context
    let context = build_project_context(&db)?;
    
    // Extract all commits
    let observations = extract_commits()?;
    
    for obs in observations {
        // Check if already extracted (by commit hash)
        if !force && is_commit_extracted(&db, &obs.commit_hash)? {
            skipped_count += 1;
            continue;
        }
        
        // Compute content hash for deduplication
        let content_hash = compute_content_hash(&obs.content);
        
        // Check if duplicate (same content hash from same source)
        if content_exists(&db, &content_hash, &obs.commit_hash)? {
            skipped_count += 1;
            continue;
        }
        
        // Auto-tag domains
        let domains = tagger.tag_domains(&obs.content, &context)?;
        
        // Create event
        let event = Event {
            event_id: generate_event_id(),
            event_type: "observation_captured".to_string(),
            timestamp: obs.commit_date,
            author: obs.author,
            sequence: get_next_sequence()?,
            payload: ObservationPayload {
                content: obs.content,
                observation_type: obs.observation_type,
                source_type: "commit".to_string(),
                source_id: obs.commit_hash.clone(),
                domains,
                reliability: 0.70, // Git commits slightly less reliable than sessions
                metadata: serde_json::json!({
                    "content_hash": content_hash
                }),
            },
        };
        
        // Write event file
        write_event_file(events_dir, &event)?;
        extracted_count += 1;
        
        // Mark as extracted
        mark_commit_extracted(&db, &obs.commit_hash)?;
    }
    
    println!("✅ Git scrape complete:");
    println!("  • Extracted: {}", extracted_count);
    println!("  • Skipped: {}", skipped_count);
    
    Ok(ScrapeStats {
        items_processed: extracted_count,
        time_elapsed: start.elapsed(),
        database_size_kb: 0,
    })
}
```

---

**1C.4: Integration Testing** (2 hours)

**Test End-to-End Flow**:
```bash
# 1. Scrape sessions
patina scrape sessions
# → Creates ~542 event files

# 2. Materialize
patina materialize
# → Builds observations.db with ~542 observations

# 3. Scrape git
patina scrape git
# → Creates ~300 event files (deduplicated)

# 4. Materialize again
patina materialize
# → Incrementally adds ~300 observations

# 5. Verify no duplicates
sqlite3 .patina/shared/project.db \
  "SELECT COUNT(*), COUNT(DISTINCT content_hash) FROM observations"
# Should match (no duplicates)

# 6. Check domain distribution
sqlite3 .patina/shared/project.db \
  "SELECT name, observation_count FROM domains ORDER BY observation_count DESC LIMIT 20"
```

---

**Week 3 Deliverables**:
- [ ] Git commit parser working
- [ ] Content hash deduplication implemented
- [ ] `patina scrape git` command complete
- [ ] All git history extracted as events
- [ ] No duplicate observations
- [ ] Sessions + git observations coexist in observations.db

---

### Week 4: Oxidize & Integration (Phase 1D)

**Goal**: Complete the event-sourced flow with vectorization and domain relationships

#### Tasks

**1D.1: Rename Embeddings → Oxidize** (2 hours)

**Changes**:
```rust
// src/commands/oxidize/mod.rs (renamed from embeddings)

// Remove extraction logic (moved to scrape)
// Keep only vectorization logic

pub fn execute(force: bool) -> Result<()> {
    let db_path = ".patina/shared/project.db";
    let vectors_dir = ".patina/shared/vectors";
    
    // Read observations from materialized DB
    let observations = load_observations(db_path)?;
    
    // Generate embeddings
    let embeddings = generate_embeddings(&observations, force)?;
    
    // Build USearch indices
    build_vector_index(vectors_dir, &embeddings)?;
    
    // Discover domain relationships
    discover_domain_relationships(db_path, &embeddings)?;
    
    Ok(())
}
```

**File Changes**:
- Rename: `src/commands/embeddings/` → `src/commands/oxidize/`
- Update: `src/main.rs` - Change command name
- Update: `src/commands/mod.rs` - Update module name

---

**1D.2: Domain Relationship Discovery** (6 hours)

**Algorithm**:
```rust
// src/commands/oxidize/domain_relationships.rs

use anyhow::Result;

pub fn discover_relationships(
    db: &SqliteDatabase,
    observations: &[Observation],
    embeddings: &[Vec<f32>],
) -> Result<()> {
    // 1. Semantic clustering
    let clusters = cluster_observations(embeddings, 0.75)?; // 75% similarity threshold
    
    // 2. Analyze each cluster for domain co-occurrence
    for cluster in clusters {
        let cluster_obs: Vec<&Observation> = cluster.indices
            .iter()
            .map(|&i| &observations[i])
            .collect();
        
        // Count domain co-occurrences
        let co_occurrences = compute_co_occurrence(&cluster_obs);
        
        // Calculate strength (0.0-1.0)
        for ((domain_a, domain_b), count) in co_occurrences {
            let strength = count as f32 / cluster_obs.len() as f32;
            
            if strength >= 0.70 { // 70% co-occurrence threshold
                insert_relationship(
                    db,
                    &domain_a,
                    &domain_b,
                    "co_occurs_with",
                    strength,
                )?;
            }
        }
    }
    
    // 3. Detect universal patterns (appear across many domains)
    let universal_domains = detect_universal_domains(observations)?;
    for domain in universal_domains {
        insert_relationship(
            db,
            &domain,
            "universal_pattern",
            "is_type",
            1.0,
        )?;
    }
    
    Ok(())
}

fn cluster_observations(embeddings: &[Vec<f32>], threshold: f32) -> Result<Vec<Cluster>> {
    // Use cosine similarity + hierarchical clustering
    // Group observations with similarity >= threshold
    todo!()
}

fn compute_co_occurrence(observations: &[&Observation]) -> HashMap<(String, String), usize> {
    let mut co_occur = HashMap::new();
    
    for obs in observations {
        let domains = parse_domains(&obs.domains)?;
        
        // All pairs of domains in this observation co-occur
        for i in 0..domains.len() {
            for j in (i + 1)..domains.len() {
                let pair = if domains[i] < domains[j] {
                    (domains[i].clone(), domains[j].clone())
                } else {
                    (domains[j].clone(), domains[i].clone())
                };
                
                *co_occur.entry(pair).or_insert(0) += 1;
            }
        }
    }
    
    co_occur
}
```

**Files to Create**:
- `src/commands/oxidize/domain_relationships.rs` - Relationship discovery
- `src/commands/oxidize/clustering.rs` - Semantic clustering

---

**1D.3: Shared/Local Split** (4 hours)

**Directory Structure**:
```
.patina/
├── shared/              # Git-tracked (team knowledge)
│   ├── events/          # Immutable event log (JSON files)
│   ├── project.db       # Materialized from events (gitignored)
│   └── vectors/         # USearch indices (gitignored)
├── local/               # Gitignored (personal workspace)
│   ├── observations.db  # Scratch space for drafts
│   └── vectors/         # Local indices
└── code.db              # Code structure (separate concern)
```

**Update .gitignore**:
```gitignore
# Phase 1: Event-sourced structure
.patina/local/
.patina/shared/project.db
.patina/shared/vectors/

# Keep events tracked
!.patina/shared/events/
```

**Update Commands**:
```rust
// All commands need to check both shared + local

// Query command
pub fn search(query: &str) -> Result<Vec<Observation>> {
    let mut results = Vec::new();
    
    // Search shared database
    results.extend(search_db(".patina/shared/project.db", query)?);
    
    // Search local database (if exists)
    if Path::new(".patina/local/observations.db").exists() {
        results.extend(search_db(".patina/local/observations.db", query)?);
    }
    
    // Deduplicate and sort by relevance
    results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
    results.dedup_by(|a, b| a.id == b.id);
    
    Ok(results)
}
```

---

**1D.4: End-to-End Testing** (4 hours)

**Complete Flow Test**:
```bash
#!/bin/bash
# tests/integration/phase1_e2e.sh

set -e

echo "🧪 Phase 1 End-to-End Test"
echo

# 1. Clean slate
rm -rf .patina/shared .patina/local
./scripts/init-phase1.sh

# 2. Scrape sessions
echo "📝 Scraping sessions..."
patina scrape sessions
SESSION_EVENTS=$(ls -1 .patina/shared/events/ | wc -l)
echo "  ✓ Created $SESSION_EVENTS event files"

# 3. Materialize
echo "🔨 Materializing..."
patina materialize
SESSION_OBS=$(sqlite3 .patina/shared/project.db "SELECT COUNT(*) FROM observations")
echo "  ✓ Materialized $SESSION_OBS observations"

# 4. Scrape git
echo "📦 Scraping git..."
patina scrape git
TOTAL_EVENTS=$(ls -1 .patina/shared/events/ | wc -l)
echo "  ✓ Created $TOTAL_EVENTS total event files"

# 5. Materialize incrementally
echo "🔨 Materializing git..."
patina materialize
TOTAL_OBS=$(sqlite3 .patina/shared/project.db "SELECT COUNT(*) FROM observations")
echo "  ✓ Total observations: $TOTAL_OBS"

# 6. Oxidize (vectorize + domain relationships)
echo "🔥 Oxidizing..."
patina oxidize
DOMAIN_COUNT=$(sqlite3 .patina/shared/project.db "SELECT COUNT(*) FROM domains")
REL_COUNT=$(sqlite3 .patina/shared/project.db "SELECT COUNT(*) FROM domain_relationships")
echo "  ✓ Domains: $DOMAIN_COUNT"
echo "  ✓ Relationships: $REL_COUNT"

# 7. Query test
echo "🔍 Testing query..."
patina query semantic "error handling" --limit 5
echo "  ✓ Query working"

# 8. Belief validation test
echo "✅ Testing belief validation..."
patina belief validate "We use Result<T,E> for error handling" --min-score 0.5
echo "  ✓ Validation working"

echo
echo "🎉 Phase 1 Complete!"
echo "   • Events: $TOTAL_EVENTS"
echo "   • Observations: $TOTAL_OBS"
echo "   • Domains: $DOMAIN_COUNT"
echo "   • Relationships: $REL_COUNT"
```

---

**1D.5: Migration Documentation** (2 hours)

**Create**: `docs/migration-phase1.md`

**Contents**:
1. **Why Migrate**: Benefits of event-sourced architecture
2. **Backup Instructions**: How to save existing data
3. **Migration Steps**: Step-by-step from old → new structure
4. **Verification**: How to confirm migration success
5. **Rollback**: How to restore backup if needed
6. **FAQ**: Common questions and troubleshooting

---

**Week 4 Deliverables**:
- [ ] `patina oxidize` separate from scrape
- [ ] Domain relationships discovered automatically
- [ ] Shared/local split complete
- [ ] All commands work with new structure
- [ ] End-to-end test passing
- [ ] Migration documentation complete

---

## Success Metrics & Validation

### Phase 1 Complete When:

**Data Quality**:
- [ ] 800+ total observations (sessions + git combined)
- [ ] 50-100 domains in catalog
- [ ] 20+ domain relationships discovered
- [ ] No duplicate observations (verified via content_hash)
- [ ] All observations have domains field (not null)

**Commands Working**:
- [ ] `patina scrape sessions` extracts all 266 sessions
- [ ] `patina scrape git` extracts all git history
- [ ] `patina materialize` builds observations.db from events
- [ ] `patina materialize --force` does full rebuild
- [ ] `patina oxidize` generates vectors + domain relationships
- [ ] `patina query semantic` searches shared + local
- [ ] `patina belief validate` works with neuro-symbolic reasoning

**Structure Correct**:
- [ ] `.patina/shared/events/` contains all event files
- [ ] `.patina/shared/project.db` exists and is materialized
- [ ] `.patina/shared/vectors/` contains USearch indices
- [ ] `.patina/local/` exists for scratch space
- [ ] `.gitignore` properly excludes materialized DBs/vectors
- [ ] Event files committed to git

**Provenance Chain**:
- [ ] Every observation → event file → git commit
- [ ] Can rebuild entire database from events
- [ ] Can query "why do I believe X?" and get full provenance
- [ ] Can time-travel: `git checkout <old-commit>` + `patina materialize`

---

## Critical Path & Dependencies

### Must Complete in Order:

**Week 1 → Week 2**: Can't scrape sessions until event infrastructure exists
**Week 2 → Week 3**: Can scrape git in parallel with session refinement
**Week 3 → Week 4**: Must finish scraping before oxidize (needs observations)

### Parallel Work Opportunities:

**Weeks 2 & 3**: 
- Session scraping (Week 2) + Git scraping (Week 3) can partially overlap
- Start git scraper while sessions are being tested

**Week 4**:
- Domain relationship discovery can happen while shared/local split is implemented

---

## Risk Mitigation

### Risk: LLM Domain Tagging Too Slow

**Mitigation**:
- Cache LLM responses by content hash
- Batch requests (10 observations per API call)
- Fallback to keyword extraction if LLM unavailable

### Risk: Too Many Event Files (Git Bloat)

**Mitigation**:
- Event files are small JSON (~500 bytes each)
- 1000 events = ~500KB total
- Git compresses well (JSON is text)
- If needed: Can batch events into date-based archives

### Risk: Existing 463 Observations Lost

**Mitigation**:
- Backup script runs first (Week 1A.3)
- Export to JSON for reference
- Can manually import critical observations as events if needed

### Risk: Schema Changes Break Existing Code

**Mitigation**:
- Add new columns with defaults (backwards compatible)
- Migration adds `domains TEXT DEFAULT '[]'`
- Existing code continues to work, new code uses domains

---

## Post-Phase 1 Immediate Actions

### Week 5: Polish & Documentation

**Tasks**:
- [ ] Write comprehensive README updates
- [ ] Create video walkthrough of new workflow
- [ ] Add `--help` text for all new commands
- [ ] Performance profiling (scrape/materialize/oxidize)
- [ ] Fix any bugs discovered during testing

### Week 6: Phase 2 Planning

**Start designing**:
- Cross-project persona (`~/.patina/persona/`)
- Belief promotion workflow (project → persona)
- Context-dependent belief system

---

## Appendix: Command Reference

### New Commands (Phase 1)

```bash
# Event sourcing
patina materialize              # Rebuild observations.db from events
patina materialize --force      # Full rebuild (ignore state)

# Scraping
patina scrape sessions          # Extract layer/sessions/*.md → events
patina scrape git               # Extract git history → events

# Oxidization (renamed from embeddings)
patina oxidize                  # Generate vectors + discover domain relationships
patina oxidize --force          # Regenerate all vectors

# Existing commands (still work)
patina query semantic "..."     # Semantic search (uses shared + local)
patina belief validate "..."    # Neuro-symbolic validation
patina ask "..."                # Q&A over observations
```

### Modified Commands

```bash
# Query now searches both shared + local
patina query semantic "error handling" --type pattern

# Belief validate uses shared database
patina belief validate "We avoid global state" --min-score 0.6
```

---

## Implementation Checklist

Use this as your tracking document. Check off each item as completed:

### Week 1: Event Foundation
- [ ] 1A.1: Event schema designed and documented
- [ ] 1A.2: Materialize command implemented
- [ ] 1A.3: Existing data backed up
- [ ] 1A.4: Fresh structure initialized
- [ ] Week 1 test: 10 sample events materialize correctly

### Week 2: Session Scraping
- [ ] 1B.1: Session markdown parser implemented
- [ ] 1B.2: Domain tagging via LLM working
- [ ] 1B.3: Session scrape command complete
- [ ] 1B.4: All 266 sessions extracted
- [ ] Week 2 test: Domains populated in database

### Week 3: Git Scraping
- [ ] 1C.1: Git commit parser implemented
- [ ] 1C.2: Content deduplication working
- [ ] 1C.3: Git scrape command complete
- [ ] 1C.4: Integration testing passing
- [ ] Week 3 test: No duplicate observations

### Week 4: Oxidize & Integration
- [ ] 1D.1: Embeddings renamed to oxidize
- [ ] 1D.2: Domain relationships discovered
- [ ] 1D.3: Shared/local split implemented
- [ ] 1D.4: End-to-end test passing
- [ ] 1D.5: Migration documentation complete

---

**Status**: Ready to begin Week 1  
**First Task**: Design event schema (1A.1)  
**Estimated Completion**: 4 weeks from start date

---

*This plan transforms Patina from a direct-write system to an event-sourced knowledge base where every belief can trace its provenance back through immutable events in git. Domains emerge organically through LLM tagging, relationships form through semantic clustering, and the entire system becomes time-travelable and auditable.*
