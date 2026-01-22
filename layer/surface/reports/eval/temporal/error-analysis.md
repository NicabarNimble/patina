---
type: analysis
id: temporal-error-analysis
created: 2026-01-22
session-origin: 20260122-061519
related:
  - eval/temporal-queryset-v2.json
  - src/commands/scrape/sessions/mod.rs
  - layer/surface/build/feat/surface-layer/SPEC.md
---

# Temporal Query Error Analysis

**Date:** 2026-01-22
**Session:** [[session-20260122-061519]] (spec review)
**Baseline MRR:** 0.159 (target: 0.400)
**Queryset:** [[temporal-queryset-v2.json]]

---

## Root Cause: Activity Log Not Indexed

The session scraper (`src/commands/scrape/sessions/mod.rs`) extracts observations from:
- `## Key Decisions` → observation_type: "decision"
- `## Patterns Observed` → observation_type: "pattern"
- `## Work Completed` → observation_type: "work" (numbered items only)
- `## Previous Session Context` → observation_type: "context"

**Missing:** `## Activity Log` section - where 80%+ of valuable session content lives.

---

## Failing Queries Analysis

### t5-observability: "how does secrets logging work"

| Expected | Found | Gap |
|----------|-------|-----|
| session 20260101-194122 | NOT FOUND | Activity Log not indexed |
| commit 631ab5e8 | NOT FOUND | Commit messages searchable but not ranked high |
| keychain.rs | ✅ Found (#3) | Code functions indexed correctly |

**Session content (line 26-28):**
```markdown
### 19:45-20:15 - Observability Phase 0
- Implemented inline `log_debug()` pattern in keychain.rs and identity.rs
- Added PATINA_LOG=1 support for secrets module
```

**Why it fails:** Content in Activity Log, not extracted to observations table.

### t7-adapter-refactor: "when was adapters module refactored to use internal"

| Expected | Found | Gap |
|----------|-------|-----|
| session 20250811-184121 | session 20260113 (wrong) | Wrong session returned |
| adapters/mod.rs | NOT FOUND | |

**Session 20250811-184121 has only 1 observation (context).** The refactoring details are in Activity Log.

### t9-persona-problem: "why does persona drown in RRF fusion results"

| Expected | Found | Gap |
|----------|-------|-----|
| session 20260101-194122 | spec-persona-fusion (related!) | Session not found |
| spec-mothership | NOT FOUND | |

**Exact match in session (line 33):**
```markdown
- Diagnosed persona surfacing: works in direct query (0.719) but drowns in RRF (0.86+)
```

**Why it fails:** This content is in Activity Log (### 20:15-21:30 - Mothership Deep Dive).

### t10-query-performance: "what caused the query performance speedup"

| Expected | Found | Gap |
|----------|-------|-----|
| session 20260101-194122 | Various performance sessions | Session not found |
| commit ba795139 | NOT FOUND | |

**Session content (Previous Session Context, line 10):**
```markdown
This followed the "specs" session which achieved a 6.8x query performance win (150ms → 22ms) by fixing model-loading bottleneck.
```

**Partial match:** This IS in the indexed "context" observation, but vocabulary gap ("speedup" vs "win", "performance" scattered).

---

## Recommendations

### Fix 1: Extract Activity Log (High Impact)

Modify `extract_observations()` in `src/commands/scrape/sessions/mod.rs` to parse Activity Log:

```rust
// Extract from Activity Log sections
if let Some(log) = extract_section(content, "Activity Log") {
    // Parse timestamped entries: ### HH:MM - Title
    for block in log.split("\n### ").skip(1) {
        // Extract meaningful bullets from each block
        for line in block.lines() {
            if line.starts_with("- ") {
                observations.push(Observation {
                    content: line[2..].trim().to_string(),
                    observation_type: "activity".to_string(),
                    timestamp: extract_time_from_header(block),
                });
            }
        }
    }
}
```

**Expected impact:** +50-100% more observations indexed, major MRR improvement.

### Fix 2: Improve Ground Truth Quality

Current queryset has only 10 queries. Many have unreliable ground truth:
- Commit SHAs hard to match (scry doesn't prioritize commits)
- Session IDs require exact match

Better ground truth:
- Focus on session observation content that IS indexed
- Add more "what", "how" queries (easier than "when")

### Fix 3: Re-scrape After Fix

```bash
patina scrape sessions
patina oxidize
patina bench retrieval --query-set eval/temporal-queryset.json
```

---

## Observation Counts

| Session | Observations | Notes |
|---------|-------------|-------|
| 20260101-194122 | 1 (context only) | Activity Log rich but unindexed |
| 20250811-184121 | 1 (context only) | Same issue |
| 20251120-110914 | 52 | Many Key Decisions |

**Total observations:** 2,573 across 510 sessions (5.0 avg)
**But:** Sessions with Activity Log only average ~1-2 observations.

---

## Next Steps

1. [ ] Implement Activity Log extraction
2. [ ] Re-scrape sessions
3. [ ] Re-run temporal benchmark
4. [ ] Measure MRR improvement

---

## References

- [[temporal-queryset-v2.json]] - Expanded queryset (35 queries) created from this analysis
- [[temporal-queryset.json]] - Original queryset (10 queries) with flawed ground truth
- `src/commands/scrape/sessions/mod.rs:194` - `extract_observations()` function to modify
- [[session-20260122-061519]] - Session where this analysis was performed
- [[feat/surface-layer/SPEC.md]] - Surface layer spec includes related measurement framework
