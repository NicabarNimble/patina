# Build Recipe

**Current Phase:** Phase 1 - MCP & Retrieval Polish

---

## What Patina IS

A local-first RAG network: portable project knowledge + personal mothership.

- **Patina Projects:** `patina init .` - full RAG (semantic, temporal, dependency)
- **Reference Repos:** `patina repo add <url>` - lightweight index in `~/.patina/cache/repos/`
- **Mothership:** `~/.patina/` - registry, personas, `patina serve` daemon

**Completed infrastructure:** Scrape pipeline, oxidize embeddings, query/scry, serve daemon, persona, rebuild command, MCP server, hybrid retrieval (MRR 0.624). All working.

---

## Specs

Deferred work (with context for why/when):

- [spec-work-deferred.md](../surface/build/spec-work-deferred.md) - Scope cuts, blockers, enhancements, future ideas

Future specs (not yet planned):

- [spec-github-adapter.md](../surface/build/spec-github-adapter.md) - GitHub integration
- [spec-model-runtime.md](../surface/build/spec-model-runtime.md) - Model flexibility

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
