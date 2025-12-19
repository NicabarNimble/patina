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
| main.rs | 844 | CLI entry point: all command definitions and routing | Core bootstrap | ✓ | Large - 28 commands, typed enums |
| lib.rs | 23 | Module declarations and re-exports | Core bootstrap | ✓ | Clean, minimal |
| paths.rs | 268 | Single source of truth for filesystem layout | Folder structure spec | ✓ | Pure functions, no I/O - good design |
| environment.rs | 450 | Detects OS/tools/languages for context generation | Adapter support | ✓ | Used by adapters, init, doctor |
| migration.rs | 263 | One-time data migration (old paths → new cache) | Folder structure spec | ✓ | Called from main early in startup |
| session.rs | 162 | Project discovery (find_project_root) | Core bootstrap | ✓ | Lean - original staging system removed |
| version.rs | 317 | Version manifest tracking component versions | Init/upgrade support | ✓ | Imports from adapters/dev_env for versions |

### 1.2 Commands Layer

CLI entry points. 24 public modules, ~19K lines total.

| Module | Lines | Purpose | Origin | Used? | Notes |
|--------|-------|---------|--------|-------|-------|
| commands/mod.rs | 28 | Module declarations | Core | ✓ | 24 public mods + dev feature-gated |
| commands/adapter.rs | 363 | Manage LLM adapter files | Adapter spec | ✓ | Sync, status, update operations |
| commands/audit.rs | 797 | File system audit/cleanup | Doctor enhancement | ✓ | Safety categorization, layer insights |
| commands/build.rs | 33 | Docker containerized build | Dev env spec | ✓ | Delegates to dev_env trait |
| commands/doctor.rs | 602 | Project health checks | Core | ✓ | Environment, config, recommendations |
| commands/model.rs | 195 | Manage embedding models | Model mgmt spec | ✓ | list/add/remove/status |
| commands/test.rs | 32 | Run tests in dev env | Dev env spec | ✓ | Delegates to dev_env trait |
| commands/upgrade.rs | 176 | Check for CLI updates | Core | ✓ | Checks crates.io for new versions |
| commands/version.rs | 158 | Show version info | Core | ✓ | Component versions, manifest |

#### Command Subdirectories

| Module | Lines | Purpose | Origin | Used? | Notes |
|--------|-------|---------|--------|-------|-------|
| commands/ask/ | 347 | Query codebase patterns | Code analysis | ✓ | Naming, conventions, architecture |
| commands/bench/ | 448 | Benchmark retrieval quality | RAG tuning | ✓ | Ground truth evaluation |
| commands/belief/ | ~100 | Neuro-symbolic reasoning | Research | ? | May be experimental |
| commands/dev/ | ~400 | Dev-only commands | Internal | ✓ | Feature-gated, release tools |
| commands/embeddings/ | ~150 | Generate/check embeddings | Oxidize support | ✓ | Lower-level than oxidize |
| commands/eval/ | 593 | Evaluate retrieval quality | Feedback loop | ✓ | MRR, recall metrics |
| commands/init/ | 1224 | Initialize project | Core | ✓ | Git, adapters, patterns, env detection |
| commands/launch/ | 518 | Open in AI frontend | Launcher spec | ✓ | Like `code .` for AI |
| commands/oxidize/ | 1805 | Build embeddings/projections | RAG pipeline | ✓ | Recipe-based, multi-dimension |
| commands/persona/ | 525 | Cross-project user knowledge | Persona spec | ✓ | Note, query, materialize |
| commands/query/ | ~200 | Semantic search interface | Legacy? | ? | May overlap with scry |
| commands/rebuild/ | 259 | Rebuild .patina from sources | Portability | ✓ | Clone-and-go support |
| commands/repo/ | 1068 | External repo management | Federation | ✓ | Add, remove, sync, fork |
| commands/scrape/ | **11,173** | Build knowledge database | RAG pipeline | ✓ | **HUGE** - see breakdown below |
| commands/scry/ | 1358 | Vector similarity search | MCP tool | ✓ | Core query interface, hybrid mode |
| commands/serve/ | 302 | Mothership HTTP daemon | Federation | ✓ | Container ↔ Mac queries |
| commands/yolo/ | 1613 | Generate devcontainers | Autonomous dev | ✓ | Scan repo, generate config |

#### Scrape Breakdown (11,173 lines)

| Submodule | Lines | Purpose | Notes |
|-----------|-------|---------|-------|
| scrape/mod.rs + database.rs | 463 | Core config/stats | Shared utilities |
| scrape/git/ | 665 | Git history scraping | Co-change, temporal |
| scrape/github/ | 342 | GitHub issues | Issue indexing |
| scrape/layer/ | 422 | Layer pattern scraping | Core/surface/dust |
| scrape/sessions/ | 537 | Session distillation | Learning capture |
| scrape/code/ (core) | 1521 | Code extraction engine | AST walking |
| scrape/code/languages/ | **7223** | 10 language parsers | ts/cpp/js/go/sol/py/rs/c/cairo |

**Finding:** Language scrapers are 7K+ lines - 20% of entire codebase. Candidate for consolidation?

### 1.3 Domain Layer

Core RAG logic - the heart of Patina. ~2,550 lines total.

#### retrieval/ (750 lines)
Multi-oracle knowledge retrieval with RRF fusion.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 18 | Public interface exports | Clean API |
| engine.rs | 353 | Parallel multi-oracle queries | Rayon parallelism, multi-repo |
| fusion.rs | 136 | RRF fusion algorithm | k=60 default |
| oracle.rs | 42 | Oracle trait definition | Abstraction point |
| oracles/mod.rs | 14 | Oracle implementations | 4 oracles |
| oracles/semantic.rs | 70 | Vector similarity | ONNX embeddings |
| oracles/lexical.rs | 65 | FTS5 full-text | SQLite |
| oracles/temporal.rs | 191 | Co-change patterns | Git history |
| oracles/persona.rs | 49 | Cross-project knowledge | User mothership |

**Status:** ✓ Well-designed. Clean trait abstraction, parallel execution.

#### embeddings/ (912 lines)
ONNX-based embedding generation.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 104 | EmbeddingEngine trait | Query vs passage handling |
| onnx.rs | 316 | ONNX Runtime integration | Pure Rust, no Python |
| database.rs | 231 | Embedding storage/retrieval | SQLite metadata |
| models.rs | 159 | Model registry/config | Provenance tracking |
| similarity.rs | 102 | Distance functions | Cosine, euclidean |

**Status:** ✓ Solid. Trait-based, model-agnostic design.

#### storage/ (799 lines)
SQLite + USearch hybrid storage.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 29 | Public interface | Clean exports |
| beliefs.rs | 330 | Belief storage | Vector + metadata |
| observations.rs | 386 | Observation storage | Event-sourced |
| types.rs | 54 | Type definitions | Shared types |

**Status:** ✓ Good separation. Dual-backend (SQL + vector) pattern.

#### layer/ (89 lines)
Pattern layer management.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 89 | Layer struct | Init, path helpers |

**Status:** ✓ Simple and correct. Just path management.

### 1.4 Infrastructure Layer

Cross-cutting utilities. ~1,120 lines total.

#### db/ (108 lines)
Simple SQLite wrapper.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 17 | Public interface | Clean exports |
| sqlite.rs | 91 | SQLite database ops | Basic CRUD |

**Status:** ✓ Minimal. Vector storage uses dedicated `storage` module.

#### git/ (1,012 lines)
Git repository management.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 26 | Public interface | Many exports |
| operations.rs | 349 | Core git operations | Shell commands |
| fork.rs | 501 | Fork detection/creation | GitHub integration |
| validation.rs | 136 | Branch validation | Safety checks |

**Status:** ✓ Comprehensive. Handles edge cases (forks, branch safety).

### 1.5 Integration Layer

External system bridges. ~2,580 lines total.

#### adapters/ (1,521 lines)
LLM-specific adapter implementations.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 82 | LLMAdapter trait | Clean abstraction |
| templates.rs | 335 | Template file management | Copy from resources |
| launch.rs | 337 | Frontend launcher | claude/gemini process launch |
| claude/mod.rs | 123 | Claude adapter impl | Version tracking |
| claude/internal/* | 382 | Claude internals | Context, scripts, paths |
| gemini/mod.rs | 87 | Gemini adapter impl | Lighter weight |
| gemini/internal/mod.rs | 85 | Gemini internals | Context generation |

**Status:** ✓ Trait-based. Clean adapter pattern implementation.

#### mcp/ (459 lines)
Model Context Protocol server.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 8 | Public interface | Single export |
| protocol.rs | 52 | JSON-RPC types | Minimal |
| server.rs | 399 | MCP server impl | scry + context tools |

**Status:** ✓ Clean. No external SDK, blocking I/O.

#### models/ (601 lines)
Base model management.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 252 | Public API + registry | Embedded model configs |
| download.rs | 142 | HTTP download + SHA256 | Provenance tracking |
| internal.rs | 207 | Lock file management | models.lock persistence |

**Status:** ✓ Good design. Registry → Lock → Cache pattern.

### 1.6 Project Management Layer

Config and state. ~1,434 lines total.

#### project/ (778 lines)
Project-level configuration.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 107 | Public API | Config load/save |
| internal.rs | 671 | Config types + migration | TOML sections |

**Status:** ✓ Handles `.patina/config.toml`, legacy JSON migration.

#### mothership/ (211 lines)
HTTP client for remote daemon.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 53 | Public API | Env var config |
| internal.rs | 158 | HTTP client impl | Container ↔ Mac |

**Status:** ✓ Simple. Used by containers to query host.

#### workspace/ (445 lines)
Global Patina configuration (~/.patina/).

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 86 | Public API | First-run detection |
| internal.rs | 359 | Setup + config | Dir structure, defaults |

**Status:** ✓ Clean. First-run wizard, global config.

### 1.7 Uncertain Status → RESOLVED

All "uncertain" modules were analyzed. **None are dead code.**

#### query/ (461 lines) - KEEP
Semantic search over beliefs/observations.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 8 | Exports | |
| semantic_search.rs | 453 | SemanticSearch engine | Wraps storage + embedder |

**Usage:** commands/query/semantic, commands/embeddings, commands/belief/validate, tests
**Verdict:** ✓ KEEP - Different domain than retrieval/. This is for **persona knowledge** (beliefs, observations). Retrieval/ is for **project knowledge** (code, git, layer).

#### reasoning/ (455 lines) - REVIEW
Prolog-based neuro-symbolic reasoning.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 8 | Exports | |
| engine.rs | 447 | Scryer Prolog integration | Symbolic validation |

**Usage:** commands/belief/validate (only consumer)
**Verdict:** ? REVIEW - Only used by `patina belief validate`. May be experimental research code. Keep if belief validation is valued, otherwise candidate for removal in Pass 2.

#### dev_env/ (222 lines) - KEEP
Development environment trait.

| Module | Lines | Purpose | Notes |
|--------|-------|---------|-------|
| mod.rs | 43 | DevEnvironment trait | build/test interface |
| docker.rs | 179 | Docker implementation | Default env |

**Usage:** commands/build, commands/test, commands/init, version.rs
**Verdict:** ✓ KEEP - Core infrastructure. The trait enables future envs (dagger, native).

### 1.8 Dependency Map

Key import relationships (by frequency):

```
MOST IMPORTED (Core Infrastructure)
├── paths (9 uses) - Filesystem layout, central truth
├── environment (10 uses) - OS/tool detection
├── project (8 uses) - Config management
└── embeddings (8 uses) - Vector operations

DOMAIN LAYER CONSUMERS
├── retrieval (8 uses) - QueryEngine for project search
│   └── Used by: commands/scry, mcp/server
├── query (3 uses) - SemanticSearch for beliefs
│   └── Used by: commands/query, commands/belief
└── storage (3 uses) - Dual SQLite + USearch
    └── Used by: query/, embeddings/

INTEGRATION LAYER
├── adapters (7 uses) - LLM adapters
│   └── Used by: commands/init, commands/launch
├── dev_env (4 uses) - Build environments
│   └── Used by: commands/build, commands/test
└── models (1 use) - Embedding model management
    └── Used by: commands/model

DEPENDENCY FLOW
main.rs → commands/* → {retrieval, query, embeddings, adapters}
                    → {project, paths, environment}
                    → {db, git, storage}
```

**Key Observations:**
1. `paths` is the foundation - pure functions defining layout
2. `project` is config hub - most commands need it
3. Two distinct query paths: `retrieval/` (project) vs `query/` (persona)
4. `embeddings` is shared infrastructure for both query paths

### Pass 1 Exit Criteria

| Criteria | Status |
|----------|--------|
| All modules have Purpose filled | [x] |
| All modules have Origin (when/why added) | [x] |
| All modules have Used? determination | [x] |
| Uncertain modules have Verdict | [x] |
| Key dependencies mapped | [x] |

### Pass 1 Summary

**Total Lines:** ~28,000 (excluding tests/examples)

**Distribution:**
| Layer | Lines | % |
|-------|-------|---|
| Commands | ~19,000 | 68% |
| Domain | ~2,550 | 9% |
| Integration | ~2,580 | 9% |
| Project Mgmt | ~1,434 | 5% |
| Infrastructure | ~1,120 | 4% |
| Top-Level | ~2,327 | 8% |
| Uncertain (resolved) | ~1,138 | 4% |

**Key Findings:**
1. **Scrape dominates** - 11K lines (39% of codebase), 7K in language parsers alone
2. **Language scrapers** are largest component - 10 parsers, highly repetitive
3. **Two query systems** - retrieval/ (project) vs query/ (persona) - intentional separation
4. **No dead code found** - all "uncertain" modules have active uses
5. **reasoning/** is only consumer of Prolog - review if belief validation is valuable
6. **Clean architecture** - layers are well-separated, dependencies flow downward

**Candidates for Pass 2 (Cleanup):**
1. Language scrapers - consolidation opportunity (7K → ?)
2. reasoning/ - may be removable if belief validation unused
3. commands/belief/ - only consumer of reasoning/

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
