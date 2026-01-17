# Spec: Commit Enrichment

**Status:** Active (Bug Fix)
**Created:** 2026-01-15
**Origin:** Session 20260115-121358 (surface layer deep dive)
**Priority:** High (blocks connection scoring)

---

## Problem

Commits are indexed in the semantic index but cannot be retrieved. The enrichment code doesn't handle the COMMIT_ID_OFFSET.

**Oxidize indexes commits at offset 3B:**
```rust
// src/commands/oxidize/mod.rs:363
const COMMIT_ID_OFFSET: i64 = 3_000_000_000;
```

**Enrichment only knows about:**
```rust
// src/commands/scry/internal/enrichment.rs
const CODE_ID_OFFSET: i64 = 1_000_000_000;    // 1B - code facts
const PATTERN_ID_OFFSET: i64 = 2_000_000_000; // 2B - patterns
// COMMIT_ID_OFFSET missing!
```

**Result:** Commit vectors (key >= 3B) are incorrectly treated as patterns, lookup fails silently.

---

## Affected Scenarios

| Scenario | Impact |
|----------|--------|
| `patina scry "why did we add X"` | Commits not returned even when semantically relevant |
| Ref repo semantic search | Commits are primary content, none retrievable |
| Connection scoring (surface layer) | Can't correlate sessions to commits |

---

## Solution

Add COMMIT_ID_OFFSET handling to `enrich_results()` in `src/commands/scry/internal/enrichment.rs`.

### Changes Required

**1. Add constant:**
```rust
const COMMIT_ID_OFFSET: i64 = 3_000_000_000;
```

**2. Add case in match (before pattern check):**
```rust
if key >= COMMIT_ID_OFFSET {
    // Commit - look up in commits table
    let rowid = key - COMMIT_ID_OFFSET;
    let result = conn.query_row(
        "SELECT sha, message, author_name, timestamp
         FROM commits
         WHERE rowid = ?",
        [rowid],
        |row| {
            let sha: String = row.get(0)?;
            let message: String = row.get(1)?;
            let author: String = row.get(2)?;
            let timestamp: String = row.get(3)?;

            Ok(ScryResult {
                id: key,
                event_type: "git.commit".to_string(),
                source_id: sha.clone(),
                timestamp,
                content: format!("{}: {} ({})", &sha[..7], message, author),
                score,
            })
        },
    );

    if let Ok(r) = result {
        enriched.push(r);
    }
} else if key >= PATTERN_ID_OFFSET {
    // ... existing pattern handling
}
```

**3. Update ID range logic order:**
```
key >= 3B → commits
key >= 2B → patterns
key >= 1B → code facts
else      → eventlog
```

---

## Validation

### Test 1: Ref Repo Commit Search
```bash
patina scry "authentication flow" --repo opencode
# Should return commits about authentication
```

### Test 2: Project Commit Search
```bash
patina scry "why lean storage"
# Should return commit 34a0a65e: "feat(scrape): implement lean storage for ref repos"
```

### Test 3: Check Index Coverage
```bash
# Before fix: commits missing from results
# After fix: commits appear with git.commit event_type
```

---

## Implementation

**Files to change:**
- `src/commands/scry/internal/enrichment.rs` (~20 lines)

**Exit criteria:**
- `patina scry` returns commits with `event_type: "git.commit"`
- Ref repo queries return commit results
- No regression in session/code/pattern results

---

## Dependency

This fix unblocks:
- Connection scoring for surface layer (session → commit similarity)
- Ref repo semantic search quality

---

## References

- Session 20260115-121358 - Discovery during surface layer spec review
- `src/commands/oxidize/mod.rs:362-378` - Commit indexing (working)
- `src/commands/scry/internal/enrichment.rs:26-29` - ID offset constants (incomplete)
