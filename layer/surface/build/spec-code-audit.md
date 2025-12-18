# Spec: Code Audit

**Purpose:** Comprehensive review of Patina codebase against layer/core values, informed by git history and session patterns. Multi-session effort.

**Approach:** Combine all categorization strategies - architectural layers, churn analysis, user impact, and core value alignment.

---

## Codebase Snapshot

Captured at audit start (2025-12-17):

| Metric | Value |
|--------|-------|
| Total lines | ~36K |
| Modules (mod.rs) | 45 |
| Using internal.rs pattern | 11 |
| Top-level directories | 17 |
| Command modules | 26 (17 dirs + 9 files) |
| Largest file | scry/mod.rs (1358 lines) |

---

## Part 1: Module Inventory

**Goal:** Document every module with its "Do X" statement and initial assessment.

### 1.1 Top-Level Files

| File | Lines | Do X | Status | Notes |
|------|-------|------|--------|-------|
| main.rs | 844 | | | |
| lib.rs | | | | |
| paths.rs | | | | |
| environment.rs | | | | |
| migration.rs | | | | |
| session.rs | | | | |
| version.rs | | | | |

### 1.2 Entry Layer (commands/)

**Purpose:** User-facing CLI commands. Should follow unix-philosophy (one tool, one job).

| Module | Lines | Do X | Internal Pattern | Status | Notes |
|--------|-------|------|------------------|--------|-------|
| commands/mod.rs | | | | | |
| commands/adapter.rs | | | | | |
| commands/audit.rs | 797 | | | | |
| commands/build.rs | | | | | |
| commands/doctor.rs | 602 | | | | |
| commands/model.rs | | | | | |
| commands/test.rs | | | | | |
| commands/upgrade.rs | | | | | |
| commands/version.rs | | | | | |
| commands/ask/ | | | | | |
| commands/bench/ | | | | | |
| commands/belief/ | | | | | |
| commands/dev/ | | | | | |
| commands/embeddings/ | | | | | |
| commands/eval/ | 593 | | | | |
| commands/init/ | | | yes | | |
| commands/launch/ | | | | | |
| commands/oxidize/ | | | | | |
| commands/persona/ | | | | | |
| commands/repo/ | | | yes | | |
| commands/scrape/ | | | | | Complex: code/, git/, github/, layer/, sessions/ |
| commands/scry/ | 1358 | | | | Largest module |
| commands/yolo/ | | | | | |

### 1.3 Domain Layer

**Purpose:** Core RAG logic. Should follow dependable-rust (stable interfaces, hidden internals).

| Module | Lines | Do X | Internal Pattern | Status | Notes |
|--------|-------|------|------------------|--------|-------|
| retrieval/mod.rs | | | | | |
| retrieval/engine.rs | | | | | |
| retrieval/fusion.rs | | | | | |
| retrieval/oracle.rs | | | | | |
| retrieval/oracles/ | | | | | |
| embeddings/mod.rs | | | | | |
| embeddings/database.rs | | | | | |
| embeddings/models.rs | | | | | |
| embeddings/onnx.rs | | | | | |
| embeddings/similarity.rs | | | | | |
| storage/mod.rs | | | | | |
| storage/beliefs.rs | | | | | |
| storage/observations.rs | | | | | |
| storage/types.rs | | | | | |
| layer/mod.rs | | | | | |

### 1.4 Infrastructure Layer

**Purpose:** Cross-cutting utilities. Should be stable, well-tested, rarely changed.

| Module | Lines | Do X | Internal Pattern | Status | Notes |
|--------|-------|------|------------------|--------|-------|
| db/mod.rs | | | | | |
| db/sqlite.rs | | | | | |
| git/mod.rs | | | | | |
| git/fork.rs | | | | | |
| git/operations.rs | | | | | |
| git/validation.rs | | | | | |

### 1.5 Integration Layer

**Purpose:** External system bridges. Should follow adapter-pattern (traits, no type leakage).

| Module | Lines | Do X | Internal Pattern | Status | Notes |
|--------|-------|------|------------------|--------|-------|
| adapters/mod.rs | | | | | |
| adapters/launch.rs | | | | | |
| adapters/templates.rs | | | | | |
| adapters/claude/ | | | yes | | |
| adapters/gemini/ | | | yes | | |
| mcp/mod.rs | | | | | |
| mcp/protocol.rs | | | | | |
| mcp/server.rs | | | | | |
| models/mod.rs | | | | | |
| models/download.rs | | | | | |
| models/internal.rs | | | yes | | |

### 1.6 Project Management Layer

**Purpose:** Config and state management. Should follow dependable-rust.

| Module | Lines | Do X | Internal Pattern | Status | Notes |
|--------|-------|------|------------------|--------|-------|
| project/mod.rs | | | | | |
| project/internal.rs | 671 | | yes | | |
| mothership/mod.rs | | | | | |
| mothership/internal.rs | | | yes | | |
| workspace/mod.rs | | | | | |
| workspace/internal.rs | | | yes | | |

### 1.7 Legacy/Unclear

**Purpose:** Modules with unclear status. May be dead code or superseded.

| Module | Lines | Do X | Internal Pattern | Status | Notes |
|--------|-------|------|------------------|--------|-------|
| query/mod.rs | | | | | Superseded by retrieval? |
| query/semantic_search.rs | | | | | |
| reasoning/mod.rs | | | | | Used? |
| reasoning/engine.rs | | | | | |
| dev_env/mod.rs | | | | | Docker only? |
| dev_env/docker.rs | | | | | |

---

## Part 2: Churn Analysis

**Goal:** Use git history to identify high-change modules and co-change patterns.

### 2.1 High-Churn Files

Query from git history (top 20 by commit count):

| File | Commits | Lines Changed | Last Modified | Notes |
|------|---------|---------------|---------------|-------|
| | | | | |

### 2.2 Co-Change Clusters

Files that frequently change together (from temporal oracle):

| Cluster | Files | Change Count | Notes |
|---------|-------|--------------|-------|
| | | | |

### 2.3 Session Pain Points

Recurring issues from layer/sessions/ history:

| Pattern | Sessions | Description | Resolution |
|---------|----------|-------------|------------|
| | | | |

---

## Part 3: Core Value Audit

**Goal:** Assess each module against layer/core principles.

### 3.1 dependable-rust.md Compliance

| Check | Pass | Fail | Notes |
|-------|------|------|-------|
| Small public interfaces | | | |
| internal.rs used appropriately | | | |
| No `pub mod internal` | | | |
| No `internal::` in signatures | | | |
| Clear "Do X" statements | | | |
| Doctests present | | | |

**Violations:**

| Module | Violation | Severity | Fix |
|--------|-----------|----------|-----|
| | | | |

### 3.2 unix-philosophy.md Compliance

| Check | Pass | Fail | Notes |
|-------|------|------|-------|
| Single responsibility | | | |
| Tools not systems | | | |
| No flag soup | | | |
| Loose coupling | | | |
| Text interfaces | | | |

**Violations:**

| Module | Violation | Severity | Fix |
|--------|-----------|----------|-----|
| | | | |

### 3.3 adapter-pattern.md Compliance

| Check | Pass | Fail | Notes |
|-------|------|------|-------|
| Trait-based integration | | | |
| No adapter-specific type leakage | | | |
| Commands use trait objects | | | |
| Minimal trait interfaces | | | |
| Mock support for testing | | | |

**Violations:**

| Module | Violation | Severity | Fix |
|--------|-----------|----------|-----|
| | | | |

---

## Part 4: Code Health

**Goal:** Mechanical checks for code quality.

### 4.1 Clippy Findings

```bash
cargo clippy --workspace -- -W clippy::all
```

| Category | Count | Notes |
|----------|-------|-------|
| Warnings | | |
| Pedantic | | |
| Nursery | | |

**Notable findings:**

| File | Warning | Fix |
|------|---------|-----|
| | | |

### 4.2 Unused Dependencies

```bash
cargo machete
```

| Dependency | Used By | Remove? |
|------------|---------|---------|
| | | |

### 4.3 Dead Code

| Module | Dead Code | Evidence | Action |
|--------|-----------|----------|--------|
| | | | |

### 4.4 Error Handling

| Pattern | Count | Modules | Notes |
|---------|-------|---------|-------|
| `anyhow::Result` | | | |
| Custom error types | | | |
| `unwrap()` usage | | | |
| `expect()` usage | | | |
| `?` propagation | | | |

### 4.5 Test Coverage

| Module | Unit Tests | Integration Tests | Doctests | Coverage |
|--------|------------|-------------------|----------|----------|
| | | | | |

---

## Part 5: User Impact Priority

**Goal:** Rank modules by user impact for prioritized remediation.

### P0: Daily Commands

| Module | Issues Found | Severity | Session |
|--------|--------------|----------|---------|
| scry | | | |
| scrape | | | |
| init | | | |
| serve (mcp) | | | |

### P1: Core Functionality

| Module | Issues Found | Severity | Session |
|--------|--------------|----------|---------|
| retrieval | | | |
| embeddings | | | |
| models | | | |

### P2: Integration Points

| Module | Issues Found | Severity | Session |
|--------|--------------|----------|---------|
| adapters | | | |
| mcp | | | |

### P3: Supporting Infrastructure

| Module | Issues Found | Severity | Session |
|--------|--------------|----------|---------|
| db | | | |
| git | | | |
| paths | | | |
| project | | | |
| mothership | | | |

---

## Part 6: Findings Summary

### Critical Issues

| # | Module | Issue | Impact | Fix Complexity |
|---|--------|-------|--------|----------------|
| | | | | |

### High Priority

| # | Module | Issue | Impact | Fix Complexity |
|---|--------|-------|--------|----------------|
| | | | | |

### Medium Priority

| # | Module | Issue | Impact | Fix Complexity |
|---|--------|-------|--------|----------------|
| | | | | |

### Low Priority / Tech Debt

| # | Module | Issue | Impact | Fix Complexity |
|---|--------|-------|--------|----------------|
| | | | | |

### Recommended Phase 2 Tasks

Based on audit findings, recommended follow-up work:

| Task | Modules | Complexity | Value |
|------|---------|------------|-------|
| | | | |

---

## Session Log

Track audit progress across sessions:

| Session | Date | Parts Completed | Findings | Notes |
|---------|------|-----------------|----------|-------|
| | | | | |

---

## Exit Criteria

| Criteria | Status |
|----------|--------|
| All modules inventoried with "Do X" | [ ] |
| Churn analysis complete | [ ] |
| Core value audit complete | [ ] |
| Code health checks run | [ ] |
| Findings prioritized | [ ] |
| Phase 2 tasks identified | [ ] |

---

## How to Use This Spec

**Starting a session:**
1. Run `/session-start "Audit: Part X"`
2. Pick up where last session left off (check Session Log)
3. Fill in tables as you audit
4. Use `/session-note` for significant findings

**Completing a section:**
1. Update Session Log with progress
2. Commit spec with findings
3. Note any blockers or questions

**When audit is complete:**
1. Write Findings Summary (Part 6)
2. Create Phase 2 tasks in build.md
3. Archive spec via `spec/code-audit` tag
