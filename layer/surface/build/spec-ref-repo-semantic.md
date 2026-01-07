# Spec: Ref Repo Semantic Training

**Status:** Phase 1 Complete, Phase 2 Defined
**Created:** 2026-01-07
**Prerequisite:** [analysis-commit-training-signal.md](../analysis-commit-training-signal.md) (complete)
**Goal:** Enable semantic search on ref repos via first-class commit signal

---

## Progress

**Phase 1 Complete (2026-01-07):**
- ✅ `src/commands/oxidize/commits.rs` created (372 lines)
- ✅ Fallback logic added to `oxidize semantic`
- ✅ Path normalization fix (`./foo` vs `foo`)
- ✅ Patterns table check for ref repos (graceful skip)
- ✅ Tier 1 validated: gemini-cli returns actual telemetry functions
- ✅ Tier 2 complete: dojo, opencode, codex (livestore hit token limit)

**Commits:**
```
d89d03e2 feat(oxidize): add commit-based semantic training for ref repos
9eebc9e9 feat(oxidize): fallback to commits when no sessions exist
```

**Results:**

| Repo | Semantic Vectors | Training Pairs |
|------|-----------------|----------------|
| gemini-cli | 3,736 | 100 |
| dojo | 2,231 | 72 |
| opencode | 2,680 | 100 |
| codex | 9,062 | 100 |
| livestore | weights only | 100 (index failed: token length) |

---

## The Problem

Ref repos have structural data but no semantic projection:

```
USER PROJECT                    REF REPO
───────────────                 ─────────────
Has sessions? ✅                Has sessions? ❌
Has commits?  ✅                Has commits?  ✅

oxidize trains on:              oxidize trains on:
  "same session = similar"        ??? (nothing)
         ↓                              ↓
  semantic.usearch ✅            semantic.usearch ❌
```

**Root cause:** `oxidize semantic` assumes sessions exist. It generates training pairs from "same session = semantically similar". Ref repos have no sessions.

**Impact:** When user queries ref repos, they only get dependency-based results (function calls, imports) not semantic similarity (conceptually related code).

---

## The Solution

Use commit messages as training signal. Commit messages are natural language descriptions of code changes—free (NL, code) pairs.

```
Training pair:
  Anchor:   commit message (natural language)
  Positive: content from files touched by commit
  Negative: content from files NOT touched by commit
```

This trains the projection to bring natural language queries close to relevant code.

---

## Design Principles

**From layer/core:**

| Principle | Application |
|-----------|-------------|
| **unix-philosophy** | Extend oxidize (one tool), don't create new system |
| **dependable-rust** | Same interface, new internal training source |
| **measure-first** | Baseline before/after on ref repo queries |

**From Ng/Sutton advisory:**

> "The simplest system that closes the loop. Then measure. Then iterate."

- Don't build Codex Q&A Agent (infrastructure)
- Implement commit-based training (~100 lines)
- Measure: does ref repo scry improve?

---

## Implementation

### Phase 1: Generate Commit Pairs (~100 lines)

Create `src/commands/oxidize/commits.rs`:

```rust
use rusqlite::Connection;
use anyhow::Result;

/// Training pair generated from commit
pub struct CommitPair {
    pub anchor: String,      // Commit message (NL)
    pub positive: String,    // Content from touched file
    pub negative: String,    // Content from untouched file
    pub weight: f32,         // Boost factor (1.0 default, 3.0 for breaking)
}

/// Generate training pairs from commits when no sessions exist
///
/// Strategy:
/// 1. Query filtered commits (conventional format + length > 30)
/// 2. For each commit: anchor=message, positive=touched file, negative=untouched file
/// 3. Weight by moment type (breaking=3x, big_bang=2x, migration=1.5x)
pub fn generate_commit_pairs(db: &Connection, limit: usize) -> Result<Vec<CommitPair>> {
    // Filter: conventional commits with meaningful messages
    let commits = db.prepare(r#"
        SELECT c.sha, c.message, c.timestamp
        FROM commits c
        WHERE (
            c.message LIKE 'feat%'
            OR c.message LIKE 'fix%'
            OR c.message LIKE 'refactor%'
            OR c.message LIKE 'perf%'
        )
        AND length(c.message) > 30
        AND c.message NOT LIKE '%wip%'
        AND c.message NOT LIKE 'Merge %'
        ORDER BY c.timestamp DESC
        LIMIT ?
    "#)?;

    // Get files touched by each commit
    let touched_files = db.prepare(r#"
        SELECT path FROM commit_files WHERE sha = ?
    "#)?;

    // Get random untouched file for negative sample
    let untouched_file = db.prepare(r#"
        SELECT path FROM code_search
        WHERE path NOT IN (SELECT path FROM commit_files WHERE sha = ?)
        ORDER BY RANDOM() LIMIT 1
    "#)?;

    // Get moment weight if exists
    let moment_weight = db.prepare(r#"
        SELECT moment_type FROM moments WHERE sha = ?
    "#)?;

    // Build pairs...
    // [implementation details]

    Ok(pairs)
}

/// Calculate weight multiplier based on moment type
fn moment_to_weight(moment_type: Option<&str>) -> f32 {
    match moment_type {
        Some("breaking") => 3.0,
        Some("big_bang") => 2.0,
        Some("migration") => 1.5,
        Some("rewrite") => 1.2,
        _ => 1.0,
    }
}
```

### Phase 2: Fallback in Oxidize (~20 lines)

Modify `src/commands/oxidize/mod.rs`:

```rust
// In generate_training_pairs() or equivalent
match projection_type {
    "semantic" => {
        if has_sessions(db)? {
            // User project: use session observations
            generate_same_session_pairs(db, num_pairs)?
        } else if has_commits(db)? {
            // Ref repo: use commit messages
            generate_commit_pairs(db, num_pairs)?
        } else {
            anyhow::bail!("No training signal: neither sessions nor commits found")
        }
    }
    // ... other projections
}

fn has_sessions(db: &Connection) -> Result<bool> {
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM sessions",
        [],
        |row| row.get(0)
    )?;
    Ok(count > 0)
}

fn has_commits(db: &Connection) -> Result<bool> {
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM commits",
        [],
        |row| row.get(0)
    )?;
    Ok(count > 0)
}
```

---

## Tier Strategy

From [analysis-commit-training-signal.md](../analysis-commit-training-signal.md):

| Tier | Repos | Conv% | Strategy |
|------|-------|-------|----------|
| **1** | gemini-cli | 55% | Use as-is (best quality) |
| **2** | dojo, opencode, codex, livestore, PAI | 50-62% | Filter + boost |
| **3** | SDL, USearch | 23-52% | Heavy filter, use moments |
| **4** | scryer-prolog, starknet-foundry, dust, daydreams | <30% | Moments only |
| **5** | game-engine | 13% | Skip (too noisy) |

**Initial rollout:** Tier 1-2 repos only (6 repos, ~18K usable commits)

---

## Measurement

### Before (Current State)

```bash
# Ref repo query returns dependency-based results only
patina scry "how does gemini-cli handle telemetry" --repo gemini-cli

# Expected: low semantic relevance, function-call matches
```

### After (With Semantic Projection)

```bash
# Run oxidize on ref repos
for repo in gemini-cli dojo opencode codex livestore; do
    patina oxidize --repo $repo semantic
done

# Verify semantic.usearch created
ls ~/.patina/cache/repos/*/semantic.usearch

# Query should now return semantically similar code
patina scry "how does gemini-cli handle telemetry" --repo gemini-cli

# Expected: conceptually related files, not just call graph
```

### Cross-Project (With Graph Routing)

```bash
# Graph routes to relevant repos, semantic finds relevant code
patina scry "telemetry best practices" --routing graph

# Expected: routes to gemini-cli (has telemetry scope), returns semantic hits
```

### Metrics

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Semantic results in top 5 | 0 | ? | > 3 |
| Cross-project relevance | Dependency only | Semantic + dependency | Both |
| User usefulness (scry.use) | Baseline | ? | Improvement |

---

## Tasks

| Task | Effort | Status |
|------|--------|--------|
| Create `src/commands/oxidize/commits.rs` | 372 lines | ✅ |
| Add `generate_commit_pairs()` function | included | ✅ |
| Add fallback in `oxidize semantic` | ~30 lines | ✅ |
| Add `has_sessions()` / `has_commits()` helpers | ~15 lines | ✅ |
| Path normalization (`./` prefix handling) | ~10 lines | ✅ |
| Patterns table check (ref repo compat) | ~15 lines | ✅ |
| Run on gemini-cli (Tier 1) | ~10 min | ✅ |
| Measure before/after on gemini-cli | ~10 min | ✅ Validated |
| Run on Tier 2 repos (dojo, opencode, codex, livestore) | ~20 min | ✅ (livestore partial) |
| Cross-project query test with graph routing | ~10 min | 🔲 |

---

## Exit Criteria

**Functional:**
- [x] `semantic.usearch` exists for Tier 1-2 repos after oxidize
- [x] `oxidize semantic` auto-detects commit fallback when no sessions
- [x] No changes to scry interface (just better data)

**Measurement:**
- [x] Ref repo scry returns semantic results (not just dependency)
  - Before: FTS5 text matches on "telemetry"
  - After: `updateTelemetryTokenCount`, `ActivityMonitor`, `MemoryMonitor`
- [ ] Cross-project queries with `--routing graph` find relevant ref repo code
- [ ] At least one user query marked useful (scry.use) from ref repo result

**Quality:**
- [x] Follows dependable-rust (internal implementation, same interface)
- [x] Follows unix-philosophy (extends oxidize, doesn't create new command)
- [x] Follows measure-first (baseline recorded before changes)

---

## Phase 2: First-Class Commit Signal

**Insight from implementation:** Commits are a first-class training signal, not a fallback.

Current code frames commits as "use when sessions don't exist." But commits capture **code cohesion** (what changes together) — valuable in its own right, available in ALL repos.

```
SIGNAL          WHERE IT EXISTS       WHAT IT CAPTURES
──────          ───────────────       ────────────────
Commits         Projects + Ref repos  Code cohesion (what changes together)
Sessions        Projects only         User intent (what user thinks about together)
```

**Current design (fallback framing):**
```rust
if has_sessions → use sessions only
else if has_commits → use commits only  // "fallback"
```

**Proposed design (first-class):**
```rust
// Commits are always valuable when available
if has_commits → use commit pairs
// Sessions are a separate signal (future phase)
```

### Phase 2 Tasks

| Task | Effort | Status |
|------|--------|--------|
| Refactor: commits as first-class (not fallback) | ~20 lines | 🔲 |
| Update output messages (remove "fallback" framing) | ~5 lines | 🔲 |
| Validate on ref repos (no regression) | ~10 min | 🔲 |
| Measure commit signal quality (Ng method) | ~30 min | 🔲 |

### Future: Session Signal Interaction

How do sessions alter/complement commits? To be explored after commits are first-class and measured.

---

## Future: Codex Q&A Agent (Deferred)

The analysis identified a more ambitious approach: Codex as an RL-style agent that generates persona-driven questions and builds Q&A documents. See [concept-repo-patina.md](../concept-repo-patina.md).

**Why deferred:**
- Commit-based semantic is simpler and addresses the immediate gap
- Codex requires infrastructure (question generation, evidence extraction, reward loop)
- Ng/Sutton principle: ship simplest fix, measure, then consider expansion

**When to revisit:**
- After semantic gap is closed and measured
- If users need higher-level understanding (not just code search)
- When 20+ queries show semantic search is working

---

## Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `src/commands/oxidize/commits.rs` | Create | Commit-based training pair generation |
| `src/commands/oxidize/mod.rs` | Modify | Add fallback logic for commits |
| `src/commands/oxidize/internal.rs` | Modify | Wire up commits.rs |

---

## References

- [analysis-commit-training-signal.md](../analysis-commit-training-signal.md) - Detailed analysis of 48K commits across 13 repos
- [concept-repo-patina.md](../concept-repo-patina.md) - Future Codex Q&A Agent vision
- [spec-mothership-graph.md](./spec-mothership-graph.md) - Graph routing (prerequisite for cross-project queries)
- Session 20260107-061556 - Origin of this spec
