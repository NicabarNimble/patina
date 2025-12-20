# Spec: Feedback Loop

**Phase:** 3
**Status:** Active
**Goal:** Measure whether Patina's retrievals are actually useful, learn from real user behavior, and improve over time.

---

## Problem Statement

### Current State

1. **Projections don't learn**: Temporal and dependency projections show constant loss during training (~0.12, ~0.17). The MLP trainer uses a broken gradient approximation that doesn't actually learn.

2. **No real-world measurement**: `patina eval` uses synthetic ground truth (same-session observations should retrieve each other). We don't know if retrievals actually help users.

3. **Flying blind**: We can't improve what we don't measure. Patina should get better with use, not stay static.

### Key Insight

**Git is truth. Sessions link queries to commits.**

We already have:
- `scry.query` → what was asked, what was returned (need to log)
- `git.commit` → what files were actually changed (already logged)
- `session tags` → bracket commits to sessions (already exist)

We can derive feedback without new storage.

---

## Design Principles

### 1. Git is Truth

We don't store feedback - we derive it from git. The eventlog is a lens into git, not a replacement for it.

```
Git stores:     commits, files, history
Patina derives: co-changes, call graphs, embeddings, FEEDBACK
```

### 2. Session Links Query to Commit

Session tags (`session-20251217-070135-start`, `session-20251217-070135-end`) bracket the work. Commits between these tags belong to the session. Queries during the session can be correlated with commits.

```
Session start → scry calls logged → work happens → commit → session end
     ↑                                                    ↓
     └──────────── session_id links them all ────────────┘
```

### 3. Stability + Utility = Relevance (Not Time Decay)

Traditional assumption: recent = relevant.

Better model:
- **Stable + useful = core knowledge** (should NOT decay)
- **Changing + useful = active work** (current relevance)
- **Stable + unused = archival** (low priority)
- **Changing + unused = noise** (deprioritize)

This maps to the layer model: core (stable+useful), surface (changing+useful), dust (the rest).

### 4. No New Commands

Extend existing infrastructure:
- `scry` → log queries (instrument)
- `scrape` → build feedback views (materialize)
- `eval --feedback` → show real-world metrics (present)
- `doctor` → unchanged (diagnose problems, not metrics)

---

## Architecture

### Event Flow

```
┌─────────────────────────────────────────────────────────────┐
│                     Event Stream                            │
├─────────────────────────────────────────────────────────────┤
│ scry.query     → what was asked, what was returned          │
│ git.commit     → what files were actually changed           │
│ session.*      → decisions, goals, context                  │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│              Feedback Materialization                        │
├─────────────────────────────────────────────────────────────┤
│ For each (query, session):                                  │
│   retrieved_files = files returned by scry                  │
│   touched_files = files in commits during session           │
│   hit = retrieved ∩ touched                                 │
│   miss = touched - retrieved                                │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│              Metrics                                         │
├─────────────────────────────────────────────────────────────┤
│ precision = |hit| / |retrieved|                             │
│ recall = |hit| / |touched|                                  │
│ utility[file] = times_useful / times_retrieved              │
└─────────────────────────────────────────────────────────────┘
```

### New Event Type: scry.query

```json
{
  "event_type": "scry.query",
  "source_id": "q_abc123",
  "timestamp": "2025-12-17T07:30:00Z",
  "data": {
    "query": "how does entity spawning work",
    "mode": "hybrid",
    "session_id": "20251217-070135",
    "results": [
      {"doc_id": "src/entity/spawn.rs", "score": 0.85, "rank": 1},
      {"doc_id": "src/world/manager.rs", "score": 0.72, "rank": 2},
      {"doc_id": "src/ecs/component.rs", "score": 0.68, "rank": 3}
    ]
  }
}
```

### Feedback Views (SQL)

```sql
-- View 1: Flatten scry.query results
CREATE VIEW IF NOT EXISTS scry_retrievals AS
SELECT
    e.seq as query_seq,
    e.timestamp as query_time,
    json_extract(e.data, '$.session_id') as session_id,
    json_extract(e.data, '$.query') as query_text,
    json_extract(e.data, '$.mode') as query_mode,
    r.value ->> '$.doc_id' as retrieved_doc,
    CAST(r.value ->> '$.rank' AS INTEGER) as rank,
    CAST(r.value ->> '$.score' AS REAL) as score
FROM eventlog e, json_each(json_extract(e.data, '$.results')) r
WHERE e.event_type = 'scry.query';

-- View 2: Session commit files
CREATE VIEW IF NOT EXISTS session_commits AS
SELECT DISTINCT
    json_extract(c.data, '$.session_id') as session_id,
    f.value as committed_file
FROM eventlog c, json_each(json_extract(c.data, '$.files')) f
WHERE c.event_type = 'git.commit'
  AND json_extract(c.data, '$.session_id') IS NOT NULL;

-- View 3: Join retrievals with commits
CREATE VIEW IF NOT EXISTS feedback_retrieval AS
SELECT
    sr.query_seq,
    sr.session_id,
    sr.query_text,
    sr.query_mode,
    sr.retrieved_doc,
    sr.rank,
    sr.score,
    CASE WHEN sc.committed_file IS NOT NULL THEN 1 ELSE 0 END as was_committed
FROM scry_retrievals sr
LEFT JOIN session_commits sc
    ON sr.session_id = sc.session_id
    AND sr.retrieved_doc = sc.committed_file;

-- View 4: Aggregate utility per document
CREATE VIEW IF NOT EXISTS doc_utility AS
SELECT
    retrieved_doc,
    COUNT(*) as times_retrieved,
    SUM(was_committed) as times_committed,
    ROUND(CAST(SUM(was_committed) AS REAL) / COUNT(*), 3) as hit_rate
FROM feedback_retrieval
GROUP BY retrieved_doc
HAVING times_retrieved >= 2;
```

---

## Implementation

### Task 3a: Instrument Scry

**File:** `src/commands/scry/mod.rs`

**Location:** After results computed, before display (~line 170)

```rust
// Log query to eventlog for feedback loop
if let Ok(session_id) = get_active_session_id() {
    let query_data = serde_json::json!({
        "query": query,
        "mode": if options.hybrid { "hybrid" }
                else if options.dimension.is_some() { options.dimension.as_ref().unwrap() }
                else { "semantic" },
        "session_id": session_id,
        "results": results.iter().enumerate().map(|(i, r)| {
            serde_json::json!({
                "doc_id": r.source_id,
                "score": r.score,
                "rank": i + 1,
                "event_type": r.event_type
            })
        }).collect::<Vec<_>>()
    });

    let _ = log_scry_query(&query_data);  // Best-effort
}
```

**Helpers:**

```rust
fn get_active_session_id() -> Result<String> {
    let content = std::fs::read_to_string(".claude/context/active-session.md")?;
    for line in content.lines() {
        if line.starts_with("**ID**:") {
            return Ok(line.replace("**ID**:", "").trim().to_string());
        }
    }
    anyhow::bail!("No active session")
}

fn log_scry_query(data: &serde_json::Value) -> Result<()> {
    use crate::commands::scrape::database::{PATINA_DB, insert_event};
    let conn = rusqlite::Connection::open(PATINA_DB)?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let query_hash = format!("q_{:x}", md5::compute(data.to_string()));
    insert_event(&conn, "scry.query", &timestamp, &query_hash, None, &data.to_string())?;
    Ok(())
}
```

### Task 3b: Session-Commit Linkage

**File:** `src/commands/scrape/git.rs`

**Approach:** Parse session tags to associate commits with sessions.

```rust
fn get_session_for_commit(commit_timestamp: &str, session_tags: &[(String, String, String)]) -> Option<String> {
    // session_tags: [(session_id, start_timestamp, end_timestamp)]
    for (session_id, start, end) in session_tags {
        if commit_timestamp >= start && commit_timestamp <= end {
            return Some(session_id.clone());
        }
    }
    None
}

fn parse_session_tags() -> Result<Vec<(String, String, String)>> {
    // Parse tags like: session-20251217-070135-start, session-20251217-070135-end
    // Extract session_id and get tag timestamps
    // Return list of (session_id, start_time, end_time)
}
```

**Change to git.commit events:** Add `session_id` to data payload when commit falls within session bounds.

### Task 3c: Feedback Views

**File:** `src/commands/scrape/database.rs`

**Location:** Add to `initialize()` after existing schema.

See SQL views in Architecture section above.

### Task 3d: Eval --feedback

**File:** `src/commands/eval/mod.rs`

**Add to execute():**

```rust
pub fn execute(dimension: Option<String>, feedback: bool) -> Result<()> {
    // ... existing eval code ...

    if feedback {
        println!("\n━━━ Real-World Feedback ━━━\n");
        let results = eval_feedback(&conn)?;
        print_feedback_results(&results);
    }

    Ok(())
}
```

**File:** `src/main.rs`

**Add flag:**

```rust
Eval {
    #[arg(long, value_enum)]
    dimension: Option<Dimension>,

    /// Show real-world feedback metrics (query → commit correlation)
    #[arg(long)]
    feedback: bool,
}
```

---

## Expected Output

```
$ patina eval --feedback

━━━ Real-World Feedback ━━━

Queries with commit data: 47
Files retrieved: 423
Files subsequently committed: 156

Overall precision: 36.9%

Precision by rank:
  #1: 52%  #2: 41%  #3: 33%  #4: 28%  #5: 25%

High utility files (retrieved → committed):
  src/commands/scry/mod.rs      (12/15 = 80%)
  src/retrieval/mod.rs          (6/9 = 67%)
  src/commands/scrape/mod.rs    (8/12 = 67%)

Missed files (committed but not retrieved):
  src/commands/init/mod.rs      (missed 4 times)
  src/lib.rs                    (missed 3 times)
```

---

## Future Work (Not This Phase)

### Learning from Feedback

Once we have measurement, we can:

1. **Boost utility scores**: Files with high hit_rate rank higher
2. **Adjust oracle weights**: Learn α,β,γ,δ for hybrid scoring
3. **Fix the trainer**: Implement proper backprop for projections
4. **Query expansion**: Retrieved file → also retrieve its co-change partners

### LLM-Agnostic Instrumentation

Currently session_id comes from `.claude/context/active-session.md`. For other adapters:
- Add MCP tool `patina_observe` for explicit file touch logging
- Or standardize session context location across adapters

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `src/commands/scry/mod.rs` | Add query logging | ~25 |
| `src/commands/scrape/database.rs` | Add feedback views | ~40 |
| `src/commands/scrape/git.rs` | Session-commit linkage | ~30 |
| `src/commands/eval/mod.rs` | Add --feedback flag + metrics | ~60 |
| `src/main.rs` | Add feedback flag to Eval | ~5 |

**Total: ~160 lines across 5 files**

---

## Validation Checklist

- [ ] `scry.query` events appear in eventlog after running scry
- [ ] Commits have `session_id` when made during active session
- [ ] `SELECT * FROM feedback_retrieval` returns correlated data
- [ ] `patina eval --feedback` shows precision metrics
- [ ] High-utility files are identifiable
- [ ] Missed files are identifiable
