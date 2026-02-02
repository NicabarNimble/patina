# Build Recipe

**Status:** v1.0 roadmap crystallized. Three pillars: epistemic, mother, distribution.

**Version:** 0.9.x patches → 0.10.0 (epistemic complete) → MINOR milestones → 1.0.0

**Recent:** v0.10.0 (2026-02-02). Epistemic layer complete — 48 beliefs, E4-E4.6c done, multi-hop grounding (100% precision, 86% recall), forge semantic integration (81 events).

---

## v1.0 Roadmap

**Spec:** [feat/v1-release/SPEC.md](../surface/build/feat/v1-release/SPEC.md)

| Pillar | Current | Target |
|--------|---------|--------|
| **Epistemic** | **COMPLETE** (v0.10.0) — 48 beliefs, verification, grounding, forge | E5/E6 deferred to mother scope |
| **Mother** | Registry + serve daemon | Federated query, persona fusion |
| **Distribution** | 52MB fat binary | Slim binary, `patina setup`, Homebrew |

**Milestones:**
```
0.9.0  ✓ Public release (fat binary)
0.9.1  ✓ Version/spec system alignment
0.9.2  ✓ Session system & adapter parity
0.9.3  ✓ Fix: session 0.9.2 hardening
0.9.4  ✓ Fix: spec archive command, belief verification
0.10.0 ✓ Epistemic layer complete (E4-E4.6c)
0.11.0 - Mother federated query
0.12.0 - Dynamic ONNX loading
0.13.0 - WASM grammars
0.14.0 - GitHub releases + Homebrew
1.0.0  - All pillars complete
```

---

## What Patina IS

A local-first RAG network: portable project knowledge + personal mother.

- **Patina Projects:** `patina init .` - full RAG (semantic, temporal, dependency)
- **Reference Repos:** `patina repo add <url>` - lightweight index in `~/.patina/cache/repos/`
- **Mother:** `~/.patina/` - registry, personas, `patina serve` daemon

**Completed infrastructure:** Scrape pipeline, oxidize embeddings, query/scry, serve daemon, persona, rebuild command, MCP server, hybrid retrieval (MRR 0.624), model management, feedback loop, assay structural queries. All working.

---

## The Architecture

**Spec:** [reference/spec-pipeline.md](../surface/build/reference/spec-pipeline.md)

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
| `patina bench retrieval` | MRR, Recall@k benchmarking | `resources/bench/*.json` |
| `patina report` | **NEW:** Full state report using own tools | Tool quality = report quality |

**Baseline metrics:** MRR 0.624, Recall@10 67.5%, Latency ~135ms

Run regularly to catch regressions.

---

## Specs

### Active

**v1.0 Pillars:**
- [feat/v1-release/SPEC.md](../surface/build/feat/v1-release/SPEC.md) - **Master roadmap:** Three pillars, patch versioning
- ~~feat/epistemic-layer~~ - **Pillar 1: COMPLETE** (v0.10.0) — 48 beliefs, E4-E4.6c, archived
- [feat/mother/SPEC.md](../surface/build/feat/mother/SPEC.md) - **Pillar 2:** Federated query, persona fusion

**Features:**
- [feat/spec-drift-detection/SPEC.md](../surface/build/feat/spec-drift-detection/SPEC.md) - **Building:** Detect stale specs before LLM reads them (temporal drift, status contradiction, assertions)
- [feat/surface-layer/SPEC.md](../surface/build/feat/surface-layer/SPEC.md) - **Design:** Distillation layer with success metrics, `patina surface` command

**In Progress:**
- [feat/ref-repo-semantic/SPEC.md](../surface/build/feat/ref-repo-semantic/SPEC.md) - **Phase 1-2 done:** Commit-based training working
- [refactor/database-identity/SPEC.md](../surface/build/refactor/database-identity/SPEC.md) - **Phase 1 done:** UIDs everywhere, Phase 2-3 remain

**Refactors:**
- [refactor/spec-system/SPEC.md](../surface/build/refactor/spec-system/SPEC.md) - **In Progress:** New folder-based spec format

**Exploration:**
- [explore/anti-slop/SPEC.md](../surface/build/explore/anti-slop/SPEC.md) - **Active:** Signal over noise, linkage as quality measure
- [explore/agents-and-yolo/SPEC.md](../surface/build/explore/agents-and-yolo/SPEC.md) - **Open:** yolo fate, agent concepts

---

## Current Focus

### Next: Mother Federated Query (v0.11.0)

**Epistemic layer is complete** (v0.10.0). The delivery layer problem (intent→principle matching)
identified during the A/B eval is a mother-scope concern. Next pillar work lives there.

**Priority 1: Delivery layer** — Beliefs don't reach the LLM during task-oriented work (delta -0.05).
The belief data is correct and valuable (+2.2 delta for knowledge queries). The gap is
intent→principle matching at retrieval time. OpenClaw's `contextFiles` pattern and OpenAI Codex's
`skills/injection.rs` are reference architectures. Mother's federated query could be the right place.

**Priority 2: Spec drift detection** — Belief `stale-context-is-hostile-context` identifies the
problem. Spec exists at `feat/spec-drift-detection/SPEC.md`.

### Epistemic Layer — COMPLETE (v0.10.0, archived)

48 beliefs, 25 verification queries, multi-hop code grounding (100% precision, 86% recall),
forge semantic integration (81 events). A/B eval confirmed belief data valuable; delivery gap
deferred to mother scope. E4.6b deprioritized, E5/E6 deferred.

**Archived:** `spec/epistemic-layer` (git tag)

### Signal Over Noise (Exploration)

**Problem:** Open source faces increasing noise (slop, duplicates, misaligned contributions). Git tracks what changed but not why or under what understanding.

**Thesis:** Linkage is the signal. Spec → Session → Commit → Code. If you can trace a change back to a spec that explains why, that's signal.

**Progress:** Problem framed, existing linkage audited (commit→session EXISTS via timestamp), gaps identified (code→spec missing). ~500-1000 lines needed to compute linkage scores.

**Spec:** [explore/anti-slop/SPEC.md](../surface/build/explore/anti-slop/SPEC.md)

### Surface Layer (Design)

**Problem:** Accumulated project wisdom is locked in eventlog/embeddings (local, not portable). When starting a new project, past learnings aren't visible.

**Solution:** `patina surface` command that distills knowledge into atomic markdown files with wikilinks.

**Status:** Design complete, needs baseline measurement before implementation.

**Spec:** [feat/surface-layer/SPEC.md](../surface/build/feat/surface-layer/SPEC.md)

### Spec System Reorg (In Progress)

**Problem:** Specs were inconsistent - lying status fields, unchecked boxes for done work, no provenance.

**Solution:** One folder-based format (SPEC.md + optional design.md), machine-parseable frontmatter, session links.

**Progress:** Format defined, new specs use it, migration ongoing.

**Spec:** [refactor/spec-system/SPEC.md](../surface/build/refactor/spec-system/SPEC.md)

### Commit Enrichment (COMPLETE)

**Problem:** Commits indexed at offset 3B but enrichment.rs only handled 1B (code) and 2B (patterns).

**Solution:** Added COMMIT_ID_OFFSET handling to scry enrichment.

**Status:** Complete (2026-01-22). Unblocks ref repo semantic search and surface layer connection scoring.

### Reference

Living documentation (not phased work):

- [reference/spec-architectural-alignment.md](../surface/build/reference/spec-architectural-alignment.md) - Command/library alignment matrices
- [reference/spec-pipeline.md](../surface/build/reference/spec-pipeline.md) - Pipeline architecture (scrape → oxidize/assay → scry)
- [reference/spec-assay.md](../surface/build/reference/spec-assay.md) - Structural queries + signals

### Deferred

See [deferred/](../surface/build/deferred/) folder (18 specs). Categories:

- **Parked** - Started, got partial win, waiting for conditions
- **Blocked** - Ready to start, waiting for dependency
- **Backlog** - Will do, lower priority than current focus
- **Ideas** - Might do, not planned

Key items:
- `belief-validation-system/` - Verifiable belief confidence (complex, needs epistemic layer first)
- `spec-skills-focused-adapter.md` - Skills-first adapter refactor (major redesign)
- `spec-skills-universal.md` - Universal SKILL.md format
- `spec-three-layers.md` - mother/patina/awaken separation concept
- `spec-report.md` - Phase 1 done, Phase 2-4 on hold
- `spec-retrieval-optimization.md` - Phase 0-1 complete (6.8x faster), Phase 2-4 need 100+ queries
- `spec-persona-fusion.md` - Phase 1 complete, Phase 2 deferred

### Archived (git tags)

Completed specs preserved via `git show spec/<name>:path/to/spec.md`:

- `spec/commit-enrichment` - Add COMMIT_ID_OFFSET to scry enrichment (2026-01-22)
- `spec/vocabulary-gap` - LLM query expansion via `expanded_terms` MCP param
- `spec/repo-org-namespace` - Fix repo name collisions with org/repo identifiers
- `spec/session-prompts` - Capture user prompts from ~/.claude/history.jsonl in session files
- `spec/remove-dev-env` - Remove dev_env subsystem (~490 lines): build/test commands, DevEnvironment trait, --dev flag
- `spec/remove-neuro-symbolic-debt` - Prolog removal (~2660 lines): reasoning/, storage/, query/, scryer-prolog dep
- `spec/ref-repo-storage` - Lean storage for ref repos: git/code direct insert, forge dedup (11-60% DB reduction)
- `spec/init-hardening` - Init/Adapter refactor: skeleton-only init, adapter refresh/doctor (Phases 1-2)
- `spec/adapter-selection` - Two-flow adapter selection (explicit --adapter vs implicit prompt), select_adapter() function
- `spec/remove-codex` - Codex removed from adapter system (it's an agent, not adapter)
- `spec/forge-bulk-fetch` - Bulk issue/PR fetch (100x faster than individual API calls)
- `spec/preflight` - Self-healing startup: auto-kill stale processes (>24h), prevent OOM conflicts
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

- `spec/session-092-hardening` - fix: Session System 0.9.2 Hardening (2026-02-02)
- `spec/reports-layer` - refactor: Reports Layer (2026-02-02)
- `spec/version-semver-alignment` - refactor: Align Version Model with Semver Convention (2026-02-02)
- `spec/verification-module-split` - refactor: Split verification.rs to Follow dependable-rust (2026-02-02)
- `spec/spec-archive-on-complete` - fix: Archive Specs on Completion (2026-02-02)
- `spec/belief-verification` - feat: Belief Verification — Connecting Beliefs to Their Ingredients (2026-02-02)
- `spec/epistemic-layer` - feat: Epistemic Markdown Layer (2026-02-02)
Full list: `git tag -l 'spec/*'`

---

## Archive

Completed specs preserved via git tags. View with: `git show spec/<name>:layer/surface/build/spec-<name>.md`

**Recent completions:**
- `spec/commit-enrichment` - Add COMMIT_ID_OFFSET to scry enrichment (2026-01-22)
- `spec/vocabulary-gap` - LLM query expansion via `expanded_terms` MCP param (2026-01-21)
- `spec/ref-repo-storage` - Lean storage for ref repos (11-60% DB reduction)
- `spec/forge-bulk-fetch` - Bulk issue/PR fetch (100x faster)
- `spec/remove-dev-env` - Remove dev_env subsystem (~490 lines)
- `spec/remove-neuro-symbolic-debt` - Prolog removal (~2660 lines)
- `spec/init-hardening` - Skeleton-only init, adapter refresh/doctor

**All tags:** `git tag -l 'spec/*'` (53 archived specs)
