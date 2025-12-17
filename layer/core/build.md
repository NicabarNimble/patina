# Build Recipe

**Current Phase:** Phase 3 - Feedback Loop

---

## What Patina IS

A local-first RAG network: portable project knowledge + personal mothership.

- **Patina Projects:** `patina init .` - full RAG (semantic, temporal, dependency)
- **Reference Repos:** `patina repo add <url>` - lightweight index in `~/.patina/cache/repos/`
- **Mothership:** `~/.patina/` - registry, personas, `patina serve` daemon

**Completed infrastructure:** Scrape pipeline, oxidize embeddings, query/scry, serve daemon, persona, rebuild command, MCP server, hybrid retrieval (MRR 0.624). All working.

---

## Specs

Active specs:

- [spec-feedback-loop.md](../surface/build/spec-feedback-loop.md) - Phase 3: Measure and learn from retrieval quality
- [spec-model-management.md](../surface/build/spec-model-management.md) - Phase 2: Base model download, caching, provenance (complete)

Deferred work (with context for why/when):

- [spec-work-deferred.md](../surface/build/spec-work-deferred.md) - Scope cuts, blockers, enhancements, future ideas

Future specs (not yet planned):

- [spec-github-adapter.md](../surface/build/spec-github-adapter.md) - GitHub integration

---

## Phase 1: MCP & Retrieval Polish

**Goal:** Make MCP tools useful on fresh projects, expose all indexed data.

### Context

Live testing on dojo (fresh clone) revealed:
- MCP tools ARE called (directive descriptions work) ✅
- But results unhelpful on fresh projects:
  - Lexical only (semantic needs `oxidize`)
  - No temporal oracle (75K co-change relationships scraped but not queryable)
  - No session history yet (first session!)

This is partly expected (Patina's value grows over time), but we can expose more of what's already scraped.

### Tasks

#### 1a: MCP Tool Rename ✅
- [x] Rename `patina_query` → `scry` (CLI parity)
- [x] Rename `patina_context` → `context` (simpler)
- [x] Add directive descriptions ("USE FIRST for any code question")
- [x] Update CLAUDE.md generation with MCP guidance
- [x] Update bootstrap content in launch.rs

#### 1b: Add TemporalOracle ✅
- [x] Create `src/retrieval/oracles/temporal.rs`
- [x] Query co-change neighbors for a file path
- [x] Fuse temporal results with lexical/semantic via RRF
- [x] Test: `scry "files related to auth"` returns co-change data

#### 1c: Auto-Oxidize on Init ✅
- [x] Should `patina init` run `oxidize`? → Yes, for best first-run UX
- [x] Added to init flow after scrape (Step 5)
- [x] Non-fatal: warns and suggests manual `patina oxidize` if fails

#### 1d: CLI Hybrid Mode ✅
- [x] Add `--hybrid` flag to `patina scry` CLI
- [x] Add `hybrid` option to HTTP API (`POST /scry`)
- [x] Shows which oracles contributed to each result

### Validation (Exit Criteria)

| Criteria | Status |
|----------|--------|
| MCP tools renamed to `scry`/`context` | [x] |
| CLAUDE.md guides LLMs to use MCP tools | [x] |
| TemporalOracle in QueryEngine | [x] |
| Co-change data queryable via `scry` | [x] |
| CLI `--hybrid` exposes RRF fusion | [x] |
| Auto-oxidize on init | [x] |
| Fresh project `scry` returns useful results | [x] |

---

## Phase 2: Model Management

**Goal:** Base models are infrastructure managed at mothership level. Projects reference by name, mothership provides files.

**Spec:** [spec-model-management.md](../surface/build/spec-model-management.md)

### Summary

Base models (~100MB each) move from repo to `~/.patina/cache/models/`. Provenance tracked in `models.lock`. Projects just reference by name.

```
registry.toml (metadata)  →  What models exist
models.lock (mothership)  →  What's downloaded + provenance
config.toml (project)     →  What model to use
```

### Tasks

#### 2a: Mothership Model Cache ✅
- [x] Create `~/.patina/cache/models/` directory structure
- [x] Add `models.lock` TOML format + parser (`src/models/internal.rs`)
- [x] Update `src/embeddings/mod.rs` to read from cache via `models::resolve_model_path()`
- [x] Add `src/paths.rs::models` module with cache path helpers

#### 2b: Model Command ✅
- [x] `patina model list` - show registry + download status
- [x] `patina model add <name>` - download with progress bar
- [x] `patina model remove <name>` - remove from cache
- [x] `patina model status` - show project needs vs cache

#### 2c: Download Infrastructure ✅
- [x] HTTP download with progress (reqwest blocking client)
- [x] SHA256 verification via `shasum -a 256` (macOS)
- [x] Provenance recording to lock file

#### 2d: Init Integration ✅
- [x] Check model availability on init (`ensure_model_available()`)
- [x] Prompt to download if missing
- [x] Validate project model against registry

#### 2e: Oxidize Updates ✅
- [x] Derive `input_dim` from registry (not recipe)
- [x] Recipe v2 format (optional `embedding_model`, 2-element layers)
- [x] Backwards compat with v1 recipes

#### 2f: Migration Path ✅
- [x] Models already gitignored (never tracked in git)
- [x] `resources/models/` contains only `registry.toml` + `README.md`
- [x] New clones download models on-demand via `patina model add`

### Validation (Exit Criteria)

| Criteria | Status |
|----------|--------|
| `patina model list` shows registry + status | [x] |
| `patina model add` downloads with provenance | [x] |
| Models stored in `~/.patina/cache/models/` | [x] |
| `models.lock` tracks downloads + checksums | [x] |
| Init validates model availability | [x] |
| Oxidize derives dimensions from registry | [x] |
| Existing projects can migrate | [x] |

---

## Phase 3: Feedback Loop

**Goal:** Measure whether Patina's retrievals are actually useful, learn from real user behavior, and improve over time.

**Spec:** [spec-feedback-loop.md](../surface/build/spec-feedback-loop.md)

### Context

Current state:
- Projections train with constant loss (not learning)
- We don't know if retrievals help the user
- `eval` measures against synthetic ground truth (same-session observations)
- No measurement of real-world retrieval quality

Key insight: Git is truth. Sessions link queries to commits. We can derive feedback without new storage.

### Design Principles

1. **Git is truth** - we don't store feedback, we derive it from git
2. **Session links query to commit** - session tags bracket the work
3. **Stability + utility = relevance** - not time decay
4. **No new commands** - extend scrape (views), scry (logging), eval (metrics)

### Tasks

#### 3a: Instrument Scry
- [ ] Add `scry.query` event logging to `scry/mod.rs`
- [ ] Capture: query text, mode, session_id, results (doc_id, score, rank)
- [ ] Best-effort logging (don't fail scry if logging fails)
- [ ] Helper: `get_active_session_id()` from active-session.md

#### 3b: Session-Commit Linkage
- [ ] Enhance git scraper to associate commits with sessions
- [ ] Use session tags to bracket commits: `session-*-start` to `session-*-end`
- [ ] Store `session_id` in git.commit event data

#### 3c: Feedback Views
- [ ] Add `scry_retrievals` view (flatten query results)
- [ ] Add `session_commits` view (files committed per session)
- [ ] Add `feedback_retrieval` view (join retrievals with commits)
- [ ] Add `doc_utility` view (aggregate hit rate per document)

#### 3d: Eval --feedback
- [ ] Add `--feedback` flag to `patina eval`
- [ ] Query feedback views for real-world precision metrics
- [ ] Show precision by rank, top utility documents
- [ ] Compare real-world vs synthetic ground truth

### Validation (Exit Criteria)

| Criteria | Status |
|----------|--------|
| `scry.query` events logged to eventlog | [ ] |
| Commits linked to sessions via tags | [ ] |
| Feedback views created in scrape | [ ] |
| `patina eval --feedback` shows real-world precision | [ ] |
| Can identify high-utility and missed files | [ ] |

---

## Completed

Shipped phases (details preserved in git tags):

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

**Tags:** `spec/release-automation`, `spec/folder-structure`, `spec/agentic-rag`, `spec/eventlog-architecture`, `spec/scrape-pipeline`, `spec/oxidize`, `spec/scry`, `spec/lexical-search`, `spec/repo-command`, `spec/serve-command`, `spec/rebuild-command`, `spec/persona-capture`, `spec/main-refactor`, `spec/launcher-architecture`, `spec/template-centralization`
