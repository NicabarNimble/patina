# Build Recipe

**Current Phase:** Phase 1 - Code Audit

---

## What Patina IS

A local-first RAG network: portable project knowledge + personal mothership.

- **Patina Projects:** `patina init .` - full RAG (semantic, temporal, dependency)
- **Reference Repos:** `patina repo add <url>` - lightweight index in `~/.patina/cache/repos/`
- **Mothership:** `~/.patina/` - registry, personas, `patina serve` daemon

**Completed infrastructure:** Scrape pipeline, oxidize embeddings, query/scry, serve daemon, persona, rebuild command, MCP server, hybrid retrieval (MRR 0.624), model management, feedback loop. All working.

---

## Specs

Active specs:

- [spec-work-deferred.md](../surface/build/spec-work-deferred.md) - Deferred work with context for why/when

Archived specs (preserved via git tags):

- `spec/feedback-loop` - Phase 3: Measure and learn from retrieval quality
- `spec/model-management` - Phase 2: Base model download, caching, provenance
- `spec/mcp-retrieval-polish` - Phase 1: MCP tool rename, temporal oracle, hybrid mode

Future specs (not yet planned):

- [spec-github-adapter.md](../surface/build/spec-github-adapter.md) - GitHub integration
- [spec-build-system.md](../surface/build/spec-build-system.md) - Git-native task tracking (deferred)

---

## Phase 1: Code Audit

**Goal:** Comprehensive review of Patina codebase against layer/core values, informed by session history and git patterns.

### Context

Patina has grown through rapid iteration. Before adding new features, audit the codebase to:
- Verify adherence to core architectural principles
- Identify dead code, unused dependencies, inconsistencies
- Surface refactoring opportunities
- Document the current state for future contributors

### Audit Framework

#### From dependable-rust.md

| Check | Description |
|-------|-------------|
| Small public interfaces | Do modules expose minimal, stable APIs? |
| internal.rs usage | Is implementation hidden appropriately? |
| No `pub mod internal` | Is internal module private? |
| No `internal::` in signatures | Do public APIs leak internal types? |
| Clear "Do X" | Can each module's purpose be stated in one sentence? |
| Doctests | Do public APIs have usage examples? |

#### From unix-philosophy.md

| Check | Description |
|-------|-------------|
| Single responsibility | Does each component do one thing? |
| Tools vs systems | Are complex systems decomposed into tools? |
| No flag soup | Are flags used appropriately (not instead of commands)? |
| Loose coupling | Do components use public interfaces only? |
| Text interfaces | Is output parseable by other tools? |

#### From adapter-pattern.md

| Check | Description |
|-------|-------------|
| Trait-based integration | Do external systems use traits? |
| No type leakage | Do traits avoid adapter-specific types? |
| Trait object usage | Do commands use `&dyn Trait`? |
| Minimal traits | Are trait interfaces 3-7 methods? |
| Mock support | Can adapters be mocked for testing? |

#### Additional Checks

| Check | Description |
|-------|-------------|
| Dead code | Unused functions, modules, dependencies? |
| Error handling | Consistent error types and propagation? |
| Test coverage | Critical paths covered? |
| Security | OWASP top 10 considerations? |
| Git patterns | What files change together? (from session/git history) |

### Tasks

#### 1a: Module Inventory
- [ ] List all modules in `src/`
- [ ] Document "Do X" statement for each
- [ ] Identify modules violating dependable-rust pattern
- [ ] Flag modules with unclear boundaries

#### 1b: Interface Analysis
- [ ] Audit public APIs for each module
- [ ] Check for `internal::` leakage in signatures
- [ ] Verify `pub mod internal` not used
- [ ] List modules missing doctests

#### 1c: Coupling Analysis
- [ ] Map inter-module dependencies
- [ ] Identify tight coupling (using internal details)
- [ ] Check adapter usage in commands
- [ ] Review trait definitions for bloat

#### 1d: Code Health
- [ ] Run `cargo clippy` with all warnings
- [ ] Check for unused dependencies (`cargo machete` or manual)
- [ ] Identify dead code paths
- [ ] Review error handling consistency

#### 1e: Session/Git Analysis
- [ ] Query feedback views for high-churn files
- [ ] Analyze co-change patterns (temporal oracle data)
- [ ] Review session history for recurring pain points
- [ ] Identify modules that frequently change together

#### 1f: Documentation
- [ ] Document audit findings
- [ ] Prioritize issues by severity
- [ ] Create follow-up tasks for refactoring
- [ ] Update CLAUDE.md if needed

### Validation (Exit Criteria)

| Criteria | Status |
|----------|--------|
| All modules inventoried with "Do X" | [ ] |
| Interface violations documented | [ ] |
| Coupling analysis complete | [ ] |
| Code health issues catalogued | [ ] |
| Git/session patterns analyzed | [ ] |
| Findings documented with priorities | [ ] |

---

## Completed

Shipped phases (details preserved in git tags):

### MCP & Retrieval Polish
MCP tools renamed (`scry`/`context`), directive descriptions for LLM tool selection, TemporalOracle integration, CLI `--hybrid` mode, auto-oxidize on init.

**Tag:** `spec/mcp-retrieval-polish`

### Model Management
Base models moved to `~/.patina/cache/models/`, provenance tracking via `models.lock`, `patina model` command (list/add/remove/status), init validates model availability, oxidize derives dimensions from registry.

**Tag:** `spec/model-management`

### Feedback Loop
Scry query logging to eventlog, session-commit linkage via git tags (75% attribution), feedback SQL views (session_queries, commit_files, query_hits), `patina eval --feedback` for real-world precision metrics.

**Tag:** `spec/feedback-loop`

### Phase 1: Folder Restructure
Centralized paths module (`src/paths.rs`), migration logic (`src/migration.rs`), user-level path consolidation. Clean separation of source vs derived data at `~/.patina/`.

**Tag:** `spec/folder-structure`

### Launcher & Adapters
Template centralization, first-run setup, launcher command (`patina` / `patina -f claude`), config consolidation (`.patina/config.toml`), branch safety (auto-stash, auto-switch), adapter commands.

**Tags:** `spec/launcher-architecture`, `spec/template-centralization`

### Agentic RAG
Oracle abstraction (semantic, lexical, persona), hybrid retrieval + RRF fusion (k=60), MCP server (`patina serve --mcp`), `scry` and `context` tools.

**Metrics:** MRR 0.624, Recall@10 67.5%, Latency ~135ms

**Includes:** Lab infrastructure (benchmarks, config), retrieval quality fixes (FTS5, layer docs), multi-project federation.

**Tags:** `spec/agentic-rag`

### Release Automation
release-plz workflow for automated GitHub releases. v0.1.0 baseline created. Conventional commits (`feat:`, `fix:`) trigger Release PRs automatically.

**Tags:** `spec/release-automation`

---

## Archive

Completed specs preserved via git tags:

```bash
git tag -l 'spec/*'              # List archived specs
git show spec/scry:layer/surface/build/spec-scry.md  # View archived spec
```

**Tags:** `spec/release-automation`, `spec/folder-structure`, `spec/agentic-rag`, `spec/eventlog-architecture`, `spec/scrape-pipeline`, `spec/oxidize`, `spec/scry`, `spec/lexical-search`, `spec/repo-command`, `spec/serve-command`, `spec/rebuild-command`, `spec/persona-capture`, `spec/main-refactor`, `spec/launcher-architecture`, `spec/template-centralization`, `spec/mcp-retrieval-polish`, `spec/model-management`, `spec/feedback-loop`
