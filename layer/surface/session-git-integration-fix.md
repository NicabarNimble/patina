# Session-Git Integration Fix: Connecting the Plumbing

**Date**: 2025-08-18
**Status**: Active Implementation
**Session**: testing-new-commands

## Problem Statement

Patina has all the pieces for Git-aware memory but they're disconnected:
- Sessions create orphaned branches that accumulate
- Git state detection exists but isn't used
- SQLite tables for tracking exist but aren't populated
- Pattern confidence is always "High" after any commit

## The Disconnected Architecture

```
Current State:
┌─────────────┐     ┌────────────┐     ┌──────────┐
│   Sessions  │  X  │ Git States │  X  │  SQLite  │
│  (branches) │     │ (detection)│     │ (tables) │
└─────────────┘     └────────────┘     └──────────┘
     ↓                    ↓                  ↓
  Orphaned           Unused Code         Empty Tables
```

## The Fix: Three Core Changes

### 1. Session Branch Strategy: Tags Not Branches

**Problem**: Every session creates `session/timestamp-name` branch, leading to hundreds of orphaned branches.

**Solution**: Use a single `work` branch with session tags.

```bash
main (clean, stable)
└── work (daily driver)
    ├── tag: session-001-start
    ├── tag: session-001-end
    ├── tag: session-002-start
    └── tag: session-002-end
```

**Implementation**:
```bash
# session-git-start.sh changes
CURRENT_BRANCH=$(git branch --show-current)
if [[ "$CURRENT_BRANCH" != "work" ]]; then
    git checkout -b work 2>/dev/null || git checkout work
fi
SESSION_TAG="session-$(date +%Y%m%d-%H%M%S)-start"
git tag -a "$SESSION_TAG" -m "Session: $1"
```

### 2. Wire Git Detection to Navigation

**Problem**: `git_detection.rs` exists but `navigate` command doesn't use it.

**Solution**: Update confidence based on actual Git state before returning results.

```rust
// src/commands/navigate.rs
use crate::indexer::internal::git_detection;

pub fn execute(args: NavigateArgs) -> Result<()> {
    let indexer = create_indexer()?;
    let mut results = indexer.navigate(&args.pattern)?;
    
    // NEW: Update confidence from Git
    for result in &mut results {
        if let Ok(state) = git_detection::detect_file_state(&result.location.path) {
            result.confidence = calculate_confidence_from_age(&state);
        }
    }
    
    display_results(results)
}

fn calculate_confidence_from_age(state: &GitState) -> Confidence {
    match state.age_days() {
        0..=7 => Confidence::Low,      // Too new
        8..=30 => Confidence::Medium,  // Settling
        31..=90 => Confidence::High,   // Proven
        _ => Confidence::Verified       // Battle-tested
    }
}
```

### 3. Track Sessions in SQLite

**Problem**: We have `state_transitions` and `git_states` tables but never write to them.

**Solution**: Make session scripts write to SQLite.

```bash
# In session-git-start.sh
sqlite3 .patina/navigation.db "
    INSERT INTO state_transitions (
        workspace_id, 
        to_state,
        transition_reason,
        metadata
    ) VALUES (
        '$SESSION_TAG',
        'SessionStart',
        'Session: $SESSION_TITLE',
        json_object(
            'branch', '$CURRENT_BRANCH',
            'parent_commit', '$(git rev-parse HEAD)',
            'goals', '$SESSION_TITLE'
        )
    );
"

# In session-git-end.sh
FILES_CHANGED=$(git diff --name-only $START_TAG..HEAD | wc -l)
COMMITS_MADE=$(git log --oneline $START_TAG..HEAD | wc -l)

sqlite3 .patina/navigation.db "
    INSERT INTO state_transitions (
        workspace_id,
        to_state,
        metadata
    ) VALUES (
        '$SESSION_TAG',
        'SessionEnd',
        json_object(
            'files_changed', $FILES_CHANGED,
            'commits_made', $COMMITS_MADE,
            'duration_minutes', $DURATION
        )
    );
"
```

## Domain-Based Memory Evolution

### Current: Single Navigation DB
```
.patina/navigation.db  -- All patterns mixed together
```

### Phase 1: Add Domain Awareness
```sql
ALTER TABLE documents ADD COLUMN domain TEXT DEFAULT 'general';
CREATE INDEX idx_documents_domain ON documents(domain);
```

### Phase 2: Domain Submodules
```
layer-general/     -- Default patterns
├── patterns.db
├── core/
├── surface/
└── dust/

layer-redis/       -- Redis domain (submodule)
├── patterns.db
├── core/
├── surface/
└── dust/
```

### Phase 3: Cross-Domain Intelligence
```sql
CREATE TABLE pattern_usage (
    pattern_id TEXT,
    domain TEXT,
    session_id TEXT,
    used_at TIMESTAMP,
    survived BOOLEAN
);

-- Auto-promote frequently used patterns
UPDATE documents SET layer = 'Core'
WHERE domain = ? AND id IN (
    SELECT pattern_id FROM pattern_usage
    WHERE survived = TRUE
    GROUP BY pattern_id
    HAVING COUNT(*) > 5
);
```

## Implementation Plan

### Step 1: Fix Git Detection (30 min)
- [ ] Wire git_detection to navigate command
- [ ] Calculate confidence from file age, not commit status
- [ ] Display actual confidence in results

### Step 2: Fix Session Branches (10 min)
- [ ] Modify session-git-start.sh to use work branch + tags
- [ ] Update session-git-end.sh to not expect branch creation
- [ ] Test session workflow without branch proliferation

### Step 3: Connect to SQLite (20 min)
- [ ] Add SQLite INSERT to session-start
- [ ] Add SQLite INSERT to session-end with metrics
- [ ] Query sessions: `SELECT * FROM state_transitions WHERE to_state LIKE 'Session%'`

### Step 4: Pattern Usage Tracking (30 min)
- [ ] Track which patterns are accessed during sessions
- [ ] Mark patterns that survive to main branch
- [ ] Auto-organize based on usage patterns

## Success Metrics

1. **No orphaned branches** - Only main + work + experiments
2. **Accurate confidence** - Based on age, not just commit status
3. **Populated SQLite** - Sessions tracked in database
4. **Pattern evolution** - Automatic promotion/demotion based on use

## Migration Commands

```bash
# Clean up old session branches (after backing up)
git branch | grep 'session/' | xargs -r git branch -D

# Create work branch
git checkout -b work

# Add domain column to existing DB
sqlite3 .patina/navigation.db "
    ALTER TABLE documents ADD COLUMN domain TEXT DEFAULT 'general';
"

# Verify session tracking
sqlite3 .patina/navigation.db "
    SELECT datetime(occurred_at, 'localtime'), to_state, 
           json_extract(metadata, '$.goals')
    FROM state_transitions 
    WHERE to_state LIKE 'Session%'
    ORDER BY occurred_at DESC;
"
```

## The Payoff

Once connected:
- Sessions become queryable work history
- Pattern confidence reflects real survival
- Failed experiments tracked but don't clutter
- Domains enable specialized knowledge banks
- Git becomes true memory, not just version control

## Next: Domain Implementation

After basic integration works, implement domain separation:
1. Create first domain submodule (layer-redis)
2. Auto-detect domain from imports/context
3. Share successful domains as public knowledge banks