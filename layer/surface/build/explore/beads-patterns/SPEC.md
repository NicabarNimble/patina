---
type: explore
id: beads-patterns
status: design
created: 2026-02-05
updated: 2026-02-05
sessions:
  origin: 20260204-233452
  work: []
related:
  - https://github.com/steveyegge/beads
  - layer/surface/epistemic/beliefs/mother-is-the-daemon.md
---

# explore: Beads Pattern Evaluation

> An issue tracker should feel like a Git branch to developers and a TODO list to agents. — Steve Yegge

**Problem:** Beads by Steve Yegge demonstrates patterns for AI-native tooling built in 6 days with Claude. Several architectural choices align with or challenge Patina's approach. Need to evaluate before adopting.

**Thesis:** Beads solves "persistent memory for agents" differently than Patina. Some patterns (hash-based IDs, three-layer sync, chemistry metaphor) may improve Patina's context orchestration.

**Triggers:** Check this spec when:
- Designing new data persistence (beliefs, sessions, patterns)
- Implementing multi-agent coordination
- Adding sync/conflict resolution logic
- Considering task/work tracking features

**Complete when:** All patterns either validated (→ belief/implementation), rejected (→ documented why), or explicitly deferred with rationale.

---

## Exit Criteria

- [x] Patterns documented with hypotheses
- [ ] Content hash deduplication - evaluated for beliefs
- [ ] Three-layer sync model - compared to current approach
- [ ] Daemon + CLI fallback - compared to mother architecture
- [ ] Ready queue concept - evaluated for agent task orchestration
- [ ] Chemistry metaphor - evaluated for workflow templates

---

## Patterns Under Evaluation

### 1. Content Hash-Based Deduplication

**Observed in:** Issue ID generation, sync/merge logic

Beads derives short hash IDs from content UUIDs. Same content = same hash across machines. Eliminates merge conflicts in distributed/multi-agent work.

**Hypothesis:** Content hashing for beliefs/patterns would prevent duplicates and enable conflict-free multi-session work.

**Current State:** Patina beliefs use filename as ID (e.g., `mother-is-the-daemon.md`). Duplicate detection relies on human judgment.

**Test Design:**

| Step | Action |
|------|--------|
| 1 | Audit existing beliefs for near-duplicates |
| 2 | Prototype content hash generation for belief files |
| 3 | Measure: duplicate detection rate, false positives |

**Rejection Criteria:** If <5% of beliefs are duplicates, overhead not justified.

**Status:** Pending - needs belief corpus analysis

---

### 2. Three-Layer Sync Model

**Observed in:** ARCHITECTURE.md

```
CLI ↔ SQLite (fast) ↔ JSONL (git-tracked) ↔ Git Remote
```

Beads uses SQLite for speed, JSONL for git-friendliness, git for distribution.

**Hypothesis:** Separating query layer (SQLite) from persistence layer (git-tracked files) enables better performance without sacrificing git-as-source-of-truth.

**Current State:** Patina has similar separation:
- `.patina/local/data/patina.db` (SQLite, not git-tracked)
- `layer/` directory (markdown, git-tracked)
- Scrape rebuilds SQLite from git-tracked sources

**Comparison:**

| Aspect | Beads | Patina |
|--------|-------|--------|
| Write path | Immediate to SQLite, debounced to JSONL | Direct to files, scrape rebuilds SQLite |
| Sync trigger | 5-second debounce | Manual `patina scrape` |
| Format | JSONL (one entity per line) | Markdown with frontmatter |

**Question:** Should Patina auto-sync SQLite ↔ layer files like Beads?

**Rejection Criteria:** If manual scrape is sufficient and auto-sync adds complexity without user benefit, reject.

**Status:** Pending - compare workflows in practice

---

### 3. Daemon + CLI Fallback (LSP Model)

**Observed in:** Daemon architecture

Beads CLI tries daemon first (faster, batched), falls back to direct DB. Daemon auto-starts on first command.

**Hypothesis:** This pattern improves responsiveness while maintaining CLI-first usability.

**Current State:** Patina `mother` daemon serves MCP only. CLI commands use SQLite directly.

**Comparison:**

| Aspect | Beads | Patina |
|--------|-------|--------|
| Daemon purpose | Fast CLI + batching | MCP server only |
| CLI without daemon | Works (direct DB) | Works (direct DB) |
| Auto-start | On first CLI command | On MCP connection |

**Question:** Should Patina CLI route through mother for consistency?

**Rejection Criteria:** If CLI latency is acceptable without daemon routing, don't add complexity.

**Status:** Deferred - mother daemon just unified, let it stabilize

---

### 4. Ready Queue Concept

**Observed in:** `bd ready` command

Returns only issues with no open blockers. Agents pick work, execute, close, repeat. Forces dependency discipline.

**Hypothesis:** A "ready queue" abstraction would help agents work on unblocked tasks without understanding full dependency graph.

**Current State:** Patina has no task/work queue. Sessions track goals but not dependencies. Claude Code's task tools (`TaskCreate`, `TaskUpdate`) exist but aren't integrated with Patina.

**Test Design:**

| Step | Action |
|------|--------|
| 1 | Track "blocked because X not done" occurrences in sessions |
| 2 | Prototype `patina ready` using existing spec status fields |
| 3 | Measure: reduction in wasted agent cycles |

**Question:** Is this Beads' domain (issue tracking) vs Patina's domain (context orchestration)?

**Rejection Criteria:** If agents rarely hit blocking dependencies, overhead not justified. If Beads is the right tool, integrate rather than reimplement.

**Status:** Pending - consider beads integration vs reimplementation

---

### 5. Chemistry Metaphor (Proto → Molecule → Wisp)

**Observed in:** MOLECULES.md

- **Proto** (solid): Frozen workflow template
- **Molecule** (liquid): Active work with real issues
- **Wisp** (vapor): Ephemeral operational steps

**Hypothesis:** Distinguishing template vs active vs ephemeral helps manage workflow lifecycle.

**Current State:** Patina has:
- **Core patterns** (frozen, rarely change)
- **Surface patterns** (active development)
- **Dust** (historical, archived)
- **Sessions** (ephemeral, but archived)

**Comparison:**

| Beads | Patina |
|-------|--------|
| Proto | Core patterns |
| Molecule | Surface specs |
| Wisp | Session notes (but we archive these) |

**Observation:** Patina archives sessions; Beads' wisps evaporate. Different philosophies on ephemeral data.

**Question:** Should some session data be truly ephemeral (not archived)?

**Rejection Criteria:** If archived sessions provide value for pattern evolution, don't add wisp-like ephemeral layer.

**Status:** Pending - review session archive utility

---

### 6. JSONL Format (One Entity Per Line)

**Observed in:** Sync layer

Beads uses JSONL for git-friendliness. Additions are append-only (no merge conflicts). Updates modify single lines.

**Hypothesis:** JSONL would reduce merge conflicts in multi-branch belief/pattern work.

**Current State:** Patina uses markdown files. Each belief/pattern is one file. Edits within files can conflict.

**Comparison:**

| Aspect | JSONL | Markdown files |
|--------|-------|----------------|
| Merge on additions | Clean (append) | Clean (new file) |
| Merge on edits | Single line | Full file |
| Human readability | Requires tooling | Native |
| Tooling required | Parser | None |

**Observation:** Patina's one-file-per-entity already avoids most conflicts. JSONL's benefit is mainly for high-volume entity updates.

**Rejection Criteria:** If one-file-per-entity suffices, don't change format.

**Status:** Likely reject - current approach adequate

---

## Integration Consideration

**Alternative:** Instead of adopting patterns into Patina, integrate Beads as a complementary tool.

| Tool | Role |
|------|------|
| Beads | Issue tracking, task orchestration, ready queue |
| Patina | Context orchestration, pattern evolution, semantic search |

**Hypothesis:** Beads for "what to do", Patina for "how we do things here".

**Test:** Use beads in a real project alongside Patina. Document friction points.

**Status:** Pending - try on next greenfield project

---

## Summary

| Pattern | Overlaps Patina? | Benefit Clear? | Decision |
|---------|------------------|----------------|----------|
| Content hash IDs | Partial | Maybe | Test |
| Three-layer sync | Yes (different impl) | Unclear | Compare |
| Daemon + CLI | Yes (mother) | No | Defer |
| Ready queue | No | Yes for agents | Test or integrate beads |
| Chemistry metaphor | Partial | Unclear | Review |
| JSONL format | No | Unlikely | Likely reject |

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | design | Created from beads deep dive in session 20260204-233452 |
