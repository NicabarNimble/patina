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
                   ┌────────────────┼────────────────┐
                   ▼                ▼                ▼
             scrape git      scrape code      scrape forge
           (commits+parsed)   (symbols)      (issues, PRs)
                   │                │                │
                   └────────────────┴────────────────┘
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
| scrape git | Extract | Capture commits, co-changes, parsed conventional commits |
| scrape code | Extract | Capture symbols, functions, types |
| scrape forge | Extract | Capture issues, PRs from GitHub/Gitea |
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
| `patina report` | **NEW:** Full state report using own tools | Tool quality = report quality |

**Baseline metrics:** MRR 0.624, Recall@10 67.5%, Latency ~135ms

Run regularly to catch regressions.

---

## Specs

### Active

- [spec-database-identity.md](../surface/build/spec-database-identity.md) - **Design:** UIDs for databases, enables federation graph
- [spec-surface-layer.md](../surface/build/spec-surface-layer.md) - **Next:** Distillation layer, federation interface, `patina surface` command
- [spec-session-prompts.md](../surface/build/spec-session-prompts.md) - **Design:** Capture user prompts in session files (reads from ~/.claude/history.jsonl)
- [spec-report.md](../surface/build/spec-report.md) - **NEW:** Self-analysis reports using patina's own tools
- [spec-vocabulary-gap.md](../surface/build/spec-vocabulary-gap.md) - LLM query expansion for terminology mismatch
- [spec-mothership.md](../surface/build/spec-mothership.md) - **Phase 1 next:** Federated query (0.5 persona complete)
- [spec-three-layers.md](../surface/build/spec-three-layers.md) - **Workshop:** mother/patina/awaken separation

---

## Current Focus

### Surface Layer (Next)

**Problem:** Accumulated project wisdom is locked in eventlog/embeddings (local, not portable). When starting a new project, past learnings aren't visible or transferable. Other projects can't query your knowledge.

**Solution:** `patina surface` command that distills knowledge into atomic markdown files with wikilinks. Surface is:
- **Derived**: Extracted by querying scry/assay
- **Committed**: Lives in git (`layer/surface/`)
- **Portable**: Federation interface for cross-project queries
- **Queryable**: Scry can search surface nodes

**Phase 1 scope:** Basic generation - query scry for decisions/patterns, assay for components, generate atomic nodes with wikilinks from co-occurrence.

**Spec:** [spec-surface-layer.md](../surface/build/spec-surface-layer.md)

### Project Reports (NEW)

**Problem:** No way to get a comprehensive "state of the repo" that uses patina's own tools. Want to dogfood scry, assay, scrape data to generate reports - tool quality = report quality.

**Solution:** `patina report` command that internally runs scry queries, assay commands, reads knowledge.db, and assembles a timestamped markdown report.

**Dual purpose:**
1. Useful output (what's the state of this codebase?)
2. Tool validation (if scry can't answer "main modules", fix scry)

**Spec:** [spec-report.md](../surface/build/spec-report.md)

### Vocabulary Gap

**Problem:** FTS5 keyword matching fails when user vocabulary differs from codebase vocabulary ("commit message search" vs "commits_fts"). Measured in temporal queryset: MRR 0.100 (target: 0.4).

**Solution:** LLM query expansion or semantic search on commits.

**Spec:** [spec-vocabulary-gap.md](../surface/build/spec-vocabulary-gap.md)

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

- `spec/adapter-selection` - Two-flow adapter selection (explicit --adapter vs implicit prompt), select_adapter() function
- `spec/remove-codex` - Codex removed from adapter system (it's an agent, not adapter)
- `spec/patina-local` - .patina/local/ directory structure for derived state
- `spec/forge-sync-v2` - Background sync via fork, PID guards, 750ms pacing, --sync/--log/--limit flags
- `spec/forge-abstraction` - ForgeReader + ForgeWriter traits, conventional commits, GitHub impl
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

## Archive

Completed specs preserved via git tags. View with: `git show spec/<name>:layer/surface/build/spec-<name>.md`

**Recent completions:**
- `spec/adapter-selection` - Two-flow adapter selection, select_adapter(), project defaults
- `spec/remove-codex` - Codex removed (agent vs adapter distinction)
- `spec/patina-local` - .patina/local/ for derived state, clean gitignore
- `spec/forge-sync-v2` - Background sync via fork, PID guards, 750ms pacing, --sync/--log/--limit flags
- `spec/forge-abstraction` - ForgeReader + ForgeWriter traits, conventional commits, GitHub impl (Gitea deferred)
- `spec/mothership-graph` - Graph routing, 100% repo recall (~1000 lines)

**All tags:** `git tag -l 'spec/*'` (30+ archived specs)
