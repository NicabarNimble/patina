# Build Recipe

**Status:** Quality gates - measure before extending, clean before adding.

**Recent:** Persona fusion Phase 1 complete (observability). Retrieval regression detected (MRR 0.624 → 0.448). Shifting focus to measurement and cleanup before new features.

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

Active specs:

- [spec-quality-gates.md](../surface/build/spec-quality-gates.md) - **Current:** Measurement-first, fix retrieval regression, cleanup legacy
- [spec-persona-fusion.md](../surface/build/spec-persona-fusion.md) - Phase 1 complete (observability), Phase 2 deferred
- [spec-pipeline.md](../surface/build/spec-pipeline.md) - Pipeline architecture (scrape → oxidize/assay → scry)
- [spec-assay.md](../surface/build/spec-assay.md) - Structural queries + signals
- [spec-work-deferred.md](../surface/build/spec-work-deferred.md) - Deferred work with context for why/when
- [spec-hosts-deploy.md](../surface/build/spec-hosts-deploy.md) - Persistent server deployment (future exploration)

Archived specs (preserved via git tags):

- `spec/secrets-v2` - Secrets v2: Local age-encrypted vault with Touch ID (current)
- `spec/secrets-1password` - Secrets v1: 1Password integration (superseded by v2)
- `spec/observable-scry` - Phase 1-3: Structured response, explicit modes, feedback logging
- `spec/robust-signals` - Structural signals experiments (Phase 1-2)
- `spec/fts-deduplication` - FTS5 deduplication fix
- `spec/code-audit` - Code audit analysis
- `spec/feedback-loop` - Measure and learn from retrieval quality
- `spec/model-management` - Base model download, caching, provenance
- `spec/assay` - Phase 0: Structural query command (inventory, imports, callers)
- `spec/mcp-retrieval-polish` - MCP tool rename, temporal oracle, hybrid mode

Future specs (not yet planned):

- [spec-lab-automation.md](../surface/build/spec-lab-automation.md) - Automated benchmarking, model comparison, metrics history
- [spec-github-adapter.md](../surface/build/spec-github-adapter.md) - GitHub integration

---

## Completed

Shipped phases (details preserved in git tags and specs):

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

**Spec:** [spec-robust-signals.md](../surface/build/spec-robust-signals.md), [spec-work-deferred.md](../surface/build/spec-work-deferred.md)

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

**Tags:** `spec/observable-scry`, `spec/assay`, `spec/release-automation`, `spec/folder-structure`, `spec/agentic-rag`, `spec/eventlog-architecture`, `spec/scrape-pipeline`, `spec/oxidize`, `spec/scry`, `spec/lexical-search`, `spec/repo-command`, `spec/serve-command`, `spec/rebuild-command`, `spec/persona-capture`, `spec/main-refactor`, `spec/launcher-architecture`, `spec/template-centralization`, `spec/mcp-retrieval-polish`, `spec/model-management`, `spec/feedback-loop`
