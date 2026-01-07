# Build Recipe

**Status:** Architectural alignment - internal code quality meets core values.

**Recent:** Legacy cleanup complete (2025-12-30). Removed ~1,100 lines: layer/dust/repos system and audit.rs. Doctor slimmed to 278 lines (pure health checks). All priority refactors done - scry, assay, doctor now in Exemplary/Acceptable tiers. See spec-architectural-alignment.md for living alignment matrix.

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
- [unix-philosophy](unix-philosophy.md): One tool, one job
- [dependable-rust](dependable-rust.md): Black box interfaces
- [adapter-pattern](adapter-pattern.md): Trait-based external system integration
- local-first: No cloud, rebuild from git
- git as memory: layer/ tracked, .patina/ derived

---

## Measurement Tools

Built-in quality measurement infrastructure:

| Command | Purpose | Ground Truth |
|---------|---------|--------------|
| `patina eval` | Retrieval quality by dimension | - |
| `patina eval --feedback` | Real-world precision from sessions | Session data |
| `patina bench retrieval` | MRR, Recall@k benchmarking | `eval/retrieval-queryset.json` |

**Baseline metrics:** MRR 0.624, Recall@10 67.5%, Latency ~135ms

Run regularly to catch regressions.

---

## Specs

### Active

Currently being worked on:

- [spec-ref-repo-semantic.md](../surface/build/spec-ref-repo-semantic.md) - **CURRENT:** Phase 1-2 complete, Phase 3: Measure & optimize
- [spec-mothership.md](../surface/build/spec-mothership.md) - **Phase 0 complete:** Git narrative + measurement
- [spec-mothership-graph.md](../surface/build/spec-mothership-graph.md) - **COMPLETE (G0-G2.5):** Graph routing, weight learning (see `spec/mothership-graph` tag)
- [spec-observability.md](../surface/build/spec-observability.md) - **Phase 0 complete**, Phase 1 deferred
- [spec-three-layers.md](../surface/build/spec-three-layers.md) - **Workshop:** Responsibility separation (mother/patina/awaken)

---

## Next: Ref Repo Semantic Gap

**Problem:** Ref repos have `dependency.usearch` but NO `semantic.usearch`. Root cause: `oxidize semantic` trains on session observations ("same session = similar"), but ref repos have no sessions.

**Solution:** Use commit messages as training signal. Analysis complete in [analysis-commit-training-signal.md](../surface/analysis-commit-training-signal.md).

**Spec:** [spec-ref-repo-semantic.md](../surface/build/spec-ref-repo-semantic.md)

### Phase 1: Commit-Based Training (Complete)

| Task | Effort | Status |
|------|--------|--------|
| Implement `generate_commit_pairs()` | 372 lines | ✅ |
| Add fallback in oxidize (commits when no sessions) | ~30 lines | ✅ |
| Run oxidize on Tier 1-2 repos | ~30 min | ✅ |
| Measure semantic quality before/after | ~30 min | ✅ |

**Results:** Tier 1 (gemini-cli) and Tier 2 (dojo, opencode, codex) now have semantic search. Before: FTS5 text matches. After: actual telemetry functions (`updateTelemetryTokenCount`, `ActivityMonitor`).

### Phase 2: First-Class Commit Signal (Complete)

**Insight:** Commits are a first-class training signal, not a fallback.

Commits capture **code cohesion** (what changes together) — valuable in its own right, available in ALL repos.

| Task | Effort | Status |
|------|--------|--------|
| Refactor: commits as first-class (not fallback) | ~20 lines | ✅ |
| Update output messages (remove "fallback" framing) | ~5 lines | ✅ |
| Validate on ref repos (no regression) | ~10 min | ✅ |

### Phase 3: Measure & Optimize (Next)

Apply Ng method to commit signal quality.

| Task | Effort | Status |
|------|--------|--------|
| Build eval queries for ref repos | ~20 min | 🔲 |
| Measure commit signal quality | ~30 min | 🔲 |
| Identify weaknesses, iterate | TBD | 🔲 |

**Design principle (Ng/Sutton):** Simplest fix that closes the loop. Don't build Codex Q&A Agent infrastructure—implement commit-based training pairs and measure.

---

## Completed: Mothership Graph (G2.5)

**Specs:**
- [spec-mothership.md](../surface/build/spec-mothership.md) - Full architecture (phases 0-3)
- [spec-mothership-graph.md](../surface/build/spec-mothership-graph.md) - Graph layer for cross-project awareness

| Phase | Build | Exit |
|-------|-------|------|
| **0-0.25c** | Git Narrative + Measurement | ✅ Complete (2026-01-05) |
| **G0** | Cross-Project Measurement | ✅ Complete (2026-01-05) - gap proven |
| **G1** | Graph Foundation | ✅ Complete (2026-01-06) - CLI, sync, edges |
| **G2** | Smart Routing | ✅ Complete (2026-01-06) - proof of concept working |
| **G2.5** | Measurement + Learning | ✅ Complete (2026-01-06) - ~1000 lines implementation |
| **G3** | Auto-Detection | Deferred - auto-populate edges from code/sessions |
| **0.5** | Persona surfaces | Deferred - `[PERSONA]` + `[PROJECT]` sections in scry |

**G2.5 Delivered:**
- ✅ edge_usage table + routing context logging (475 lines)
- ✅ scry.use → edge_usage linking (feedback signal connected)
- ✅ Weight learning algorithm (290 lines)
- ✅ `patina mother stats` command (93 lines)
- ✅ `patina mother learn` command (83 lines)
- ✅ Bench repo recall metric (89 lines)
- ✅ Graph routing: 100% repo recall vs 0% dumb routing
- ✅ Weights learned from usage: 1.0 → 1.02-1.06

**Key insight:** Graph routing works. Now fix semantic gap in ref repos so routing has good content to find.

### Reference

Living documentation (not phased work):

- [spec-architectural-alignment.md](../surface/build/spec-architectural-alignment.md) - Command/library alignment matrices
- [spec-pipeline.md](../surface/build/spec-pipeline.md) - Pipeline architecture (scrape → oxidize/assay → scry)
- [spec-assay.md](../surface/build/spec-assay.md) - Structural queries + signals

### Deferred

See [deferred/](../surface/build/deferred/) folder. Categories:

- **Parked** - Started, got partial win, waiting for conditions
- **Blocked** - Ready to start, waiting for dependency
- **Backlog** - Will do, lower priority than current focus
- **Ideas** - Might do, not planned

Key items:
- `spec-retrieval-optimization.md` - Phase 0-1 complete (6.8x faster), Phase 2-4 need 100+ queries
- `spec-persona-fusion.md` - Phase 1 complete, Phase 2 deferred
- `spec-work-deferred.md` - Legacy backlog (needs rebuild into proper specs)

### Archived (git tags)

Completed specs preserved via `git show spec/<name>:path/to/spec.md`:

- `spec/llm-frontends` - Unified 5-command experience across Claude, Gemini, OpenCode
- `spec/remove-legacy-repos-and-audit` - Removed layer/dust/repos and audit.rs (~1,100 lines)
- `spec/quality-gates` - MRR regression fix (0.427→0.588), legacy cleanup, CI gate
- `spec/secrets-v2` - Local age-encrypted vault with Touch ID
- `spec/observable-scry` - Structured response, explicit modes, feedback logging
- `spec/robust-signals` - Structural signals experiments
- `spec/feedback-loop` - Measure and learn from retrieval quality
- `spec/model-management` - Base model download, caching, provenance
- `spec/assay` - Structural query command
- `spec/mcp-retrieval-polish` - MCP tool rename, temporal oracle, hybrid mode
- `spec/agentic-rag` - Oracle abstraction, hybrid retrieval, MCP server

Full list: `git tag -l 'spec/*'`

---

## Completed

Shipped phases (details preserved in git tags and specs):

### Legacy Cleanup (doctor/audit/repos)

Removed deprecated systems: layer/dust/repos (replaced by `patina repo`) and audit.rs (low-value hidden tool). Doctor slimmed from 602→278 lines, now pure health checks. Total: ~1,100 lines removed. Tracked in spec-architectural-alignment.md.

**Tag:** `spec/remove-legacy-repos-and-audit`

### Assay Refactoring

Refactored assay command from monolithic 997-line file to black-box pattern with internal/ modules. Result: mod.rs 134 lines (-86%), 6 focused internal modules (util, imports, inventory, functions, derive). Follows dependable-rust pattern established in scry refactoring. Tracked in spec-architectural-alignment.md.

### Quality Gates

Measurement-first cleanup before extending. Fixed MRR regression (0.427 → 0.588, +37.7%) caused by stale database entries from deleted commands. Archived 4 legacy commands (922 lines): `query`, `ask`, `embeddings`, `belief`. Added CI quality gate for retrieval benchmarks (informational mode, MRR >= 0.55 threshold).

**Tag:** `spec/quality-gates`

### Secrets v2 (Local Vault)

Replaced 1Password backend with local age-encrypted vault. Full implementation:
- **Identity:** macOS Keychain with Touch ID, `PATINA_IDENTITY` env for CI/headless
- **Vaults:** Global (`~/.patina/`) + Project (`.patina/`) with merge at runtime
- **Multi-recipient:** Team members and CI can decrypt project vaults
- **Session cache:** Via `patina serve` daemon, prevents repeated Touch ID prompts
- **SSH injection:** `patina secrets run --ssh host -- cmd` for remote execution
- **CLI:** `add`, `run`, `add-recipient`, `remove-recipient`, `list-recipients`, `--lock`, `--export-key`, `--import-key`, `--reset`

Fixes container/CI gaps from v1. No external account required.

**Tag:** `spec/secrets-v2`

### Observable Scry (Phase 1 → 3)

Made scry explainable, steerable, and instrumented. `--explain` flag shows per-oracle contributions. Explicit modes for intent (`orient`, `recent`, `why`). Feedback logging with query IDs, `scry open/copy/feedback` commands, MCP `use` mode callback. Gaps documented in spec-work-deferred.md.

**Tag:** `spec/observable-scry`

### Structural Signals (Phase 1 → 1.5 → 2)

Added structural signal computation to assay (`assay derive`): is_used, importer_count, activity_level, centrality, commit_count, contributor_count, is_entry_point, is_test_file, directory_depth, file_size_rank. Achieved MRR 0.554 (+2.2% from baseline).

Phase 2 experiment: tried boosting RRF scores with structural priors. Result: no improvement for relevance queries. Boost layer removed. Key lesson: structural signals are priors (importance), not relevance signals. Useful for orientation queries, not "where is X" queries.

**Tag:** `spec/robust-signals`

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

### Mothership Graph (G0-G2.5)
Cross-project awareness via relationship graph. Phases G0-G2.5 delivered ~1000 lines: graph foundation (graph.db, nodes, edges, CLI), smart routing (100% repo recall vs 0% dumb), learning loop (edge_usage, weight learning). Key commands: `patina mother graph/link/sync/stats/learn`. G3 (auto-detection) deferred.

**Tag:** `spec/mothership-graph`

---

## Archive

Completed specs preserved via git tags:

```bash
git tag -l 'spec/*'              # List archived specs
git show spec/scry:layer/surface/build/spec-scry.md  # View archived spec
```

**Tags:** `spec/llm-frontends`, `spec/quality-gates`, `spec/secrets-v2`, `spec/observable-scry`, `spec/assay`, `spec/release-automation`, `spec/folder-structure`, `spec/agentic-rag`, `spec/eventlog-architecture`, `spec/scrape-pipeline`, `spec/oxidize`, `spec/scry`, `spec/lexical-search`, `spec/repo-command`, `spec/serve-command`, `spec/rebuild-command`, `spec/persona-capture`, `spec/main-refactor`, `spec/launcher-architecture`, `spec/template-centralization`, `spec/mcp-retrieval-polish`, `spec/model-management`, `spec/feedback-loop`, `spec/remove-legacy-repos-and-audit`, `spec/robust-signals`, `spec/mothership-graph`
