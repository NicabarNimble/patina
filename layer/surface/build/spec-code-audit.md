# Spec: Code Audit

**Purpose:** Comprehensive multi-pass audit of Patina codebase. Understand before judging, clean before polishing.

**Approach:** Six iterative passes, each building on the previous. Document as we go.

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

## Pass Overview

| Pass | Focus | Goal | Output |
|------|-------|------|--------|
| **Pass 1** | Inventory | What do we have? Why? | Module map, purpose, origin |
| **Pass 2** | Cleanup | Remove dead weight | Leaner codebase |
| **Pass 3** | Alignment | Core value tightening | Pattern consistency |
| **Pass 4** | Deep Dive | Go deep, document | Doctests, architecture notes |
| **Pass 5** | Hardening | Security + testing | Coverage, validation |
| **Pass 6** | Polish | API + deps + final docs | Production ready |

---

## Pass 1: Inventory

**Goal:** Understand what we have and why it exists. No judgment yet - just mapping.

### Questions to Answer

- What does this module do? (one sentence)
- Why does it exist? (what problem does it solve?)
- When was it added? (git archaeology)
- Is it actively used? (grep for imports/calls)
- What depends on it? What does it depend on?

### 1.1 Top-Level Files

| File | Lines | Purpose | Origin | Used? | Notes |
|------|-------|---------|--------|-------|-------|
| main.rs | 844 | | | | |
| lib.rs | | | | | |
| paths.rs | | | | | |
| environment.rs | | | | | |
| migration.rs | | | | | |
| session.rs | | | | | |
| version.rs | | | | | |

### 1.2 Commands Layer

CLI entry points. What commands exist and what do they do?

| Module | Lines | Purpose | Origin | Used? | Notes |
|--------|-------|---------|--------|-------|-------|
| commands/mod.rs | | | | | |
| commands/adapter.rs | | | | | |
| commands/audit.rs | 797 | | | | |
| commands/build.rs | | | | | |
| commands/doctor.rs | 602 | | | | |
| commands/model.rs | | | | | |
| commands/test.rs | | | | | |
| commands/upgrade.rs | | | | | |
| commands/version.rs | | | | | |

#### Command Subdirectories

| Module | Lines | Purpose | Origin | Used? | Notes |
|--------|-------|---------|--------|-------|-------|
| commands/ask/ | | | | | |
| commands/bench/ | | | | | |
| commands/belief/ | | | | | |
| commands/dev/ | | | | | |
| commands/embeddings/ | | | | | |
| commands/eval/ | 593 | | | | |
| commands/init/ | | | | | |
| commands/launch/ | | | | | |
| commands/oxidize/ | | | | | |
| commands/persona/ | | | | | |
| commands/repo/ | | | | | |
| commands/scrape/ | | | | | Submodules: code/, git/, github/, layer/, sessions/ |
| commands/scry/ | 1358 | | | | Largest module |
| commands/yolo/ | | | | | |

### 1.3 Domain Layer

Core RAG logic - the heart of Patina.

| Module | Lines | Purpose | Origin | Used? | Notes |
|--------|-------|---------|--------|-------|-------|
| retrieval/mod.rs | | | | | |
| retrieval/engine.rs | | | | | |
| retrieval/fusion.rs | | | | | |
| retrieval/oracle.rs | | | | | |
| retrieval/oracles/* | | | | | |
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

Cross-cutting utilities.

| Module | Lines | Purpose | Origin | Used? | Notes |
|--------|-------|---------|--------|-------|-------|
| db/mod.rs | | | | | |
| db/sqlite.rs | | | | | |
| git/mod.rs | | | | | |
| git/fork.rs | | | | | |
| git/operations.rs | | | | | |
| git/validation.rs | | | | | |

### 1.5 Integration Layer

External system bridges.

| Module | Lines | Purpose | Origin | Used? | Notes |
|--------|-------|---------|--------|-------|-------|
| adapters/mod.rs | | | | | |
| adapters/launch.rs | | | | | |
| adapters/templates.rs | | | | | |
| adapters/claude/* | | | | | |
| adapters/gemini/* | | | | | |
| mcp/mod.rs | | | | | |
| mcp/protocol.rs | | | | | |
| mcp/server.rs | | | | | |
| models/mod.rs | | | | | |
| models/download.rs | | | | | |
| models/internal.rs | | | | | |

### 1.6 Project Management Layer

Config and state.

| Module | Lines | Purpose | Origin | Used? | Notes |
|--------|-------|---------|--------|-------|-------|
| project/mod.rs | | | | | |
| project/internal.rs | 671 | | | | |
| mothership/mod.rs | | | | | |
| mothership/internal.rs | | | | | |
| workspace/mod.rs | | | | | |
| workspace/internal.rs | | | | | |

### 1.7 Uncertain Status

Modules that may be legacy, experimental, or superseded.

| Module | Lines | Purpose | Origin | Used? | Verdict | Notes |
|--------|-------|---------|--------|-------|---------|-------|
| query/mod.rs | | | | | | Superseded by retrieval? |
| query/semantic_search.rs | | | | | | |
| reasoning/mod.rs | | | | | | |
| reasoning/engine.rs | | | | | | |
| dev_env/mod.rs | | | | | | |
| dev_env/docker.rs | | | | | | |

### 1.8 Dependency Map

Which modules import which? (Generated after inventory)

```
[To be filled: key dependency relationships]
```

### Pass 1 Exit Criteria

| Criteria | Status |
|----------|--------|
| All modules have Purpose filled | [ ] |
| All modules have Origin (when/why added) | [ ] |
| All modules have Used? determination | [ ] |
| Uncertain modules have Verdict | [ ] |
| Key dependencies mapped | [ ] |

---

## Pass 2: Cleanup

**Goal:** Remove dead weight. Less code = less to audit, less to maintain.

### 2.1 Dead Code Candidates

From Pass 1, modules marked as unused or uncertain:

| Module | Evidence | Decision | PR/Commit |
|--------|----------|----------|-----------|
| | | | |

### 2.2 Unused Dependencies

```bash
cargo machete  # or manual analysis
```

| Dependency | Used By | Decision | PR/Commit |
|------------|---------|----------|-----------|
| | | | |

### 2.3 Dead Functions/Types

Within modules that survive, any dead internal code?

| Module | Dead Code | Evidence | PR/Commit |
|--------|-----------|----------|-----------|
| | | | |

### 2.4 Consolidation Opportunities

Modules that should be merged or simplified?

| Modules | Proposal | Rationale | PR/Commit |
|---------|----------|-----------|-----------|
| | | | |

### Pass 2 Exit Criteria

| Criteria | Status |
|----------|--------|
| Dead modules removed | [ ] |
| Unused dependencies removed | [ ] |
| Dead internal code removed | [ ] |
| Consolidation complete | [ ] |
| All removals committed | [ ] |
| Tests still pass | [ ] |

---

## Pass 3: Alignment

**Goal:** Tighten remaining code against layer/core values. Apply patterns consistently.

### 3.1 dependable-rust.md Audit

| Check | Modules Passing | Modules Failing | Notes |
|-------|-----------------|-----------------|-------|
| Small public interfaces | | | |
| internal.rs used appropriately | | | |
| No `pub mod internal` | | | |
| No `internal::` in signatures | | | |
| Clear "Do X" (one sentence purpose) | | | |

**Violations to fix:**

| Module | Violation | Severity | Fix | PR/Commit |
|--------|-----------|----------|-----|-----------|
| | | | | |

### 3.2 unix-philosophy.md Audit

| Check | Modules Passing | Modules Failing | Notes |
|-------|-----------------|-----------------|-------|
| Single responsibility | | | |
| Tools not systems | | | |
| No flag soup | | | |
| Loose coupling | | | |
| Text interfaces | | | |

**Violations to fix:**

| Module | Violation | Severity | Fix | PR/Commit |
|--------|-----------|----------|-----|-----------|
| | | | | |

### 3.3 adapter-pattern.md Audit

| Check | Modules Passing | Modules Failing | Notes |
|-------|-----------------|-----------------|-------|
| Trait-based integration | | | |
| No adapter-specific type leakage | | | |
| Commands use trait objects | | | |
| Minimal trait interfaces (3-7 methods) | | | |
| Mock support for testing | | | |

**Violations to fix:**

| Module | Violation | Severity | Fix | PR/Commit |
|--------|-----------|----------|-----|-----------|
| | | | | |

### 3.4 Large File Decomposition

Files over 500 lines that may need splitting:

| File | Lines | Proposal | PR/Commit |
|------|-------|----------|-----------|
| main.rs | 844 | | |
| scry/mod.rs | 1358 | | |
| commands/audit.rs | 797 | | |
| project/internal.rs | 671 | | |
| commands/scrape/git/mod.rs | 665 | | |
| commands/doctor.rs | 602 | | |
| commands/eval/mod.rs | 593 | | |

### Pass 3 Exit Criteria

| Criteria | Status |
|----------|--------|
| dependable-rust violations fixed | [ ] |
| unix-philosophy violations fixed | [ ] |
| adapter-pattern violations fixed | [ ] |
| Large files addressed | [ ] |
| All fixes committed | [ ] |
| Tests still pass | [ ] |

---

## Pass 4: Deep Dive

**Goal:** Go deep on remaining modules. Document as we understand.

### 4.1 Module Deep Dives

For each significant module, create understanding:

| Module | Doctest Added | Architecture Notes | Inline Comments | Status |
|--------|---------------|-------------------|-----------------|--------|
| retrieval/ | | | | |
| embeddings/ | | | | |
| adapters/ | | | | |
| mcp/ | | | | |
| commands/scry/ | | | | |
| commands/scrape/ | | | | |
| commands/init/ | | | | |
| models/ | | | | |
| project/ | | | | |

### 4.2 Architecture Documentation

Create/update architecture notes in layer/surface/:

| Topic | Document | Status |
|-------|----------|--------|
| Retrieval pipeline | | |
| Embedding flow | | |
| MCP protocol | | |
| Scrape pipeline | | |
| Project lifecycle | | |

### 4.3 API Examples

Ensure key APIs have runnable examples:

| API | Example Added | Location |
|-----|---------------|----------|
| | | |

### Pass 4 Exit Criteria

| Criteria | Status |
|----------|--------|
| Core modules have doctests | [ ] |
| Architecture documented | [ ] |
| Complex code has inline comments | [ ] |
| Key APIs have examples | [ ] |

---

## Pass 5: Hardening

**Goal:** Security review and test coverage expansion.

### 5.1 Security Review

| Check | Status | Findings | Fix |
|-------|--------|----------|-----|
| Input validation (CLI args) | | | |
| Path traversal prevention | | | |
| SQL injection (if applicable) | | | |
| Secrets handling | | | |
| Dependency vulnerabilities | | | |
| File permission handling | | | |

```bash
cargo audit  # Check for known vulnerabilities
```

### 5.2 Test Coverage Analysis

| Module | Unit Tests | Integration Tests | Coverage | Gap |
|--------|------------|-------------------|----------|-----|
| | | | | |

### 5.3 Missing Test Cases

| Module | Missing Test | Priority | Added |
|--------|--------------|----------|-------|
| | | | |

### 5.4 Error Path Testing

Are error conditions tested?

| Module | Happy Path | Error Path | Edge Cases |
|--------|------------|------------|------------|
| | | | |

### Pass 5 Exit Criteria

| Criteria | Status |
|----------|--------|
| Security review complete | [ ] |
| No critical vulnerabilities | [ ] |
| Core modules have test coverage | [ ] |
| Error paths tested | [ ] |

---

## Pass 6: Polish

**Goal:** Final consistency pass. API polish, dependency health, final documentation.

### 6.1 API Consistency

| Check | Status | Notes |
|-------|--------|-------|
| Naming conventions consistent | | |
| Error types consistent | | |
| Return type patterns consistent | | |
| Public API stable | | |

### 6.2 Dependency Health

| Dependency | Version | Latest | Action |
|------------|---------|--------|--------|
| | | | |

```bash
cargo outdated  # Check for outdated deps
```

### 6.3 Final Documentation

| Document | Status | Notes |
|----------|--------|-------|
| README.md | | |
| CLAUDE.md | | |
| layer/core/* up to date | | |
| layer/surface/build/* current | | |

### 6.4 Clippy Clean

```bash
cargo clippy --workspace -- -W clippy::all -W clippy::pedantic
```

| Category | Count | Addressed |
|----------|-------|-----------|
| Warnings | | |
| Pedantic | | |

### Pass 6 Exit Criteria

| Criteria | Status |
|----------|--------|
| API consistent across modules | [ ] |
| Dependencies up to date | [ ] |
| Documentation current | [ ] |
| Clippy clean | [ ] |
| Ready for next phase of development | [ ] |

---

## Session Log

Track progress across sessions:

| Session | Date | Pass | Work Done | Findings | Commits |
|---------|------|------|-----------|----------|---------|
| | | | | | |

---

## Findings Summary

Populated after each pass:

### Critical Issues

| # | Pass | Module | Issue | Resolution |
|---|------|--------|-------|------------|
| | | | | |

### Patterns Observed

Recurring themes across the audit:

| Pattern | Occurrences | Notes |
|---------|-------------|-------|
| | | |

### Recommendations for Future Development

| Recommendation | Rationale |
|----------------|-----------|
| | |

---

## Exit Criteria (Full Audit)

| Criteria | Status |
|----------|--------|
| Pass 1: Inventory complete | [ ] |
| Pass 2: Cleanup complete | [ ] |
| Pass 3: Alignment complete | [ ] |
| Pass 4: Deep dive complete | [ ] |
| Pass 5: Hardening complete | [ ] |
| Pass 6: Polish complete | [ ] |
| Findings documented | [ ] |
| Phase 2 work identified | [ ] |

---

## How to Use This Spec

**Starting a session:**
```
/session-start "Audit: Pass X - [specific focus]"
```

**Working through a pass:**
1. Fill in tables as you audit
2. Use `/session-note` for significant findings
3. Commit changes with `audit(passX):` prefix
4. Update Session Log

**Completing a pass:**
1. Verify Exit Criteria met
2. Update Findings Summary
3. Commit spec
4. Start next pass or take a break

**Commit message format:**
```
audit(pass1): inventory commands layer
audit(pass2): remove dead query/ module
audit(pass3): fix internal:: leakage in adapters
audit(pass4): add doctests to retrieval engine
audit(pass5): add input validation tests
audit(pass6): update outdated dependencies
```
