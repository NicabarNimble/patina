# Build Recipe

**Current Phase:** Pipeline Architecture - scry as unified oracle

---

## What Patina IS

A local-first RAG network: portable project knowledge + personal mothership.

- **Patina Projects:** `patina init .` - full RAG (semantic, temporal, dependency)
- **Reference Repos:** `patina repo add <url>` - lightweight index in `~/.patina/cache/repos/`
- **Mothership:** `~/.patina/` - registry, personas, `patina serve` daemon

**Completed infrastructure:** Scrape pipeline, oxidize embeddings, query/scry, serve daemon, persona, rebuild command, MCP server, hybrid retrieval (MRR 0.624), model management, feedback loop, assay structural queries. All working.

---

## The Architecture

**Spec:** [spec-pipeline.md](../surface/build/spec-pipeline.md)

```
                            GIT (source of truth)
                                    │
                                    ▼
                                 scrape
                        (extract facts from reality)
                                    │
                                    ▼
                               SQLite DB
                                    │
                   ┌────────────────┴────────────────┐
                   ▼                                 ▼
               oxidize                            assay
           (→ embeddings)                      (→ signals)
                   │                                 │
                   └────────────┬────────────────────┘
                                ▼
                              scry
                       (unified oracle)
                                │
                                ▼
                          LLM Frontend
```

**Core insight:** scry is the API between LLM and codebase knowledge. Everything else prepares for that moment.

| Command | Role | "Do X" |
|---------|------|--------|
| scrape | Extract | Capture raw → structured facts |
| oxidize | Prepare (semantic) | Build embeddings from facts |
| assay | Prepare (structural) | Build signals from facts |
| scry | Deliver | Fuse and route knowledge to LLM |

**Values alignment:**
- unix-philosophy: One tool, one job
- dependable-rust: Black box interfaces
- local-first: No cloud, rebuild from git
- git as memory: layer/ tracked, .patina/ derived

---

## Specs

Active specs:

- [spec-pipeline.md](../surface/build/spec-pipeline.md) - Pipeline architecture (scrape → oxidize/assay → scry)
- [spec-assay.md](../surface/build/spec-assay.md) - Structural queries + signals
- [spec-work-deferred.md](../surface/build/spec-work-deferred.md) - Deferred work with context for why/when

Archived specs (preserved via git tags):

- `spec/assay` - Phase 0: Structural query command (inventory, imports, callers)
- `spec/feedback-loop` - Measure and learn from retrieval quality
- `spec/model-management` - Base model download, caching, provenance
- `spec/mcp-retrieval-polish` - MCP tool rename, temporal oracle, hybrid mode

Future specs (not yet planned):

- [spec-github-adapter.md](../surface/build/spec-github-adapter.md) - GitHub integration

---

## Current Work: Assay Signals

**Goal:** Add structural signal preparation to assay, wire into scry as StructuralOracle.

**Spec:** [spec-pipeline.md](../surface/build/spec-pipeline.md)

### Context

Audit of Patina architecture revealed missing ORGANIZE stage. We go scrape → scry, skipping derived signals. Assay currently queries raw facts; it should also prepare signals (health, activity, centrality, staleness) that scry can fuse with semantic results.

### Tasks

| Task | Status |
|------|--------|
| Write spec-pipeline.md | [x] |
| Add signal tables to schema | [x] |
| Implement `assay derive` subcommand | [x] |
| Compute health signal (importer_count, is_used) | [x] |
| Compute activity signal (commits/week, contributors) | [x] |
| Add StructuralOracle to scry | [x] |
| Wire signals into RRF fusion | [x] |
| Update MCP tool descriptions | [x] |

### Signals to Compute

| Signal | Source | Formula |
|--------|--------|---------|
| `is_used` | import_facts | importer_count > 0 OR is_entry_point |
| `activity_level` | co_changes, git | commits in last N days |
| `core_contributors` | git history | top authors by commit count |
| `centrality` | call_graph | PageRank or degree centrality |
| `staleness` | cross-reference | contradicts CI, references deleted things |

### Validation

| Criteria | Status |
|----------|--------|
| `patina assay derive` computes signals | [x] |
| Signals queryable via `patina assay` | [x] |
| scry includes structural signals in fusion | [x] |
| Lab metrics (eval/bench) show improvement | [ ] |

---

## Next: Phase 1.5 - Robust Signals

**Goal:** Add language-agnostic signals that work reliably across different repos.

**Spec:** [spec-robust-signals.md](../surface/build/spec-robust-signals.md) (to be written)

### ML/RL Insight

We're building a retrieval re-ranking system. Structural signals are **priors** - query-independent importance scores (like PageRank for code).

**Key insight:** We don't need *accurate* features. We need features that are:
- Correlated with usefulness
- Robust across repos/languages
- Cheap to compute

`importer_count` is ~60% accurate due to relative imports, language-specific syntax. That's fine - it's a weak signal. The fix isn't to make it accurate; it's to **add more weak signals** and let ensemble/fusion combine them.

### Signal Reliability Matrix

| Signal | Accuracy | Language-agnostic | Status |
|--------|----------|-------------------|--------|
| `importer_count` | ~60% | No | [x] Have (noisy, accept it) |
| `activity_level` | ~90% | Yes | [x] Have |
| `commit_count` | ~95% | Yes | [ ] Add |
| `contributor_count` | ~95% | Yes | [ ] Add |
| `is_entry_point` | ~99% | Yes | [ ] Add |
| `file_size_rank` | ~99% | Yes | [ ] Add |
| `directory_depth` | ~99% | Yes | [ ] Add |

### Tasks

| Task | Status |
|------|--------|
| Add `commit_count` to module_signals | [ ] |
| Add `contributor_count` to module_signals | [ ] |
| Add `is_entry_point` detection (main.rs, index.ts, __init__.py) | [ ] |
| Normalize all signals to 0-1 range | [ ] |
| Update StructuralOracle to use composite score | [ ] |
| Re-run lab metrics, compare to baseline (MRR 0.542) | [ ] |

### Design Principle

```
Many weak signals > One accurate signal
```

Let Phase 3 (learned weights) figure out which signals matter for which repos. Don't over-engineer individual signal accuracy.

---

## Completed

Shipped phases (details preserved in git tags):

### Assay Command (Phase 0)
Structural query interface for codebase facts. Inventory, imports/importers, callers/callees queries. MCP tool integration. Reduces 40+ shell calls to 1-3 patina commands.

**Tag:** `spec/assay`

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

**Tags:** `spec/assay`, `spec/release-automation`, `spec/folder-structure`, `spec/agentic-rag`, `spec/eventlog-architecture`, `spec/scrape-pipeline`, `spec/oxidize`, `spec/scry`, `spec/lexical-search`, `spec/repo-command`, `spec/serve-command`, `spec/rebuild-command`, `spec/persona-capture`, `spec/main-refactor`, `spec/launcher-architecture`, `spec/template-centralization`, `spec/mcp-retrieval-polish`, `spec/model-management`, `spec/feedback-loop`
