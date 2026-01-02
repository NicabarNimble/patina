# Spec: Deferred Work

**Status:** Needs Rebuild
**Purpose:** Capture work that was planned but consciously deferred, with context for why and when to revisit.

**Pattern:** When archiving completed phases, unfinished items move here instead of disappearing into git history.

---

> **TODO: Rebuild into proper specs**
>
> This file is a collection of deferred items from various phases. Each major item should become its own spec in `deferred/` with:
> - Clear phase tracking
> - Resume triggers
> - Exit criteria
>
> For now, this serves as the backlog. Items here are either:
> - **Parked** - Started, got partial win, waiting for conditions
> - **Blocked** - Ready to start, waiting for dependency
> - **Backlog** - Will do, lower priority than current focus
> - **Ideas** - Might do, not planned

---

---

## Scope Cuts

Work cut from completed phases to ship faster.

### Context Layer

| Field | Value |
|-------|-------|
| **Origin** | Phase 1 (Launcher & Adapters) |
| **Archived** | `spec/launcher-architecture` |
| **Why deferred** | Scope reduction - focused on core launcher to ship |
| **When to revisit** | When starting capture automation |

**Original tasks:**
- Create `.patina/context.md` schema (LLM-agnostic project context)
- Minimal augmentation: add patina hooks (MCP pointer, layer/ reference)
- Log all actions for transparency

**Value:** Projects would have a single context file that works across Claude, Gemini, etc.

---

### Session MCP Tools

| Field | Value |
|-------|-------|
| **Origin** | Phase 2 (Agentic RAG) |
| **Archived** | `spec/agentic-rag` |
| **Why deferred** | Prioritized `scry` and `context` tools first |
| **When to revisit** | When starting capture automation |

**Original tasks:**
- `session_start` - Start tracked session via MCP
- `session_note` - Capture insight via MCP
- `session_end` - End session and archive via MCP

**Value:** LLMs could manage sessions directly without shell scripts.

---

## External Blockers

Work blocked on dependencies outside our control.

### Gemini MCP Testing

| Field | Value |
|-------|-------|
| **Origin** | Phase 2 (Agentic RAG) |
| **Archived** | `spec/agentic-rag` |
| **Why deferred** | Gemini CLI doesn't support MCP yet |
| **When to revisit** | When Gemini adds MCP support |

**Original tasks:**
- Test `patina serve --mcp` with Gemini CLI
- Verify tool discovery and invocation
- Document any Gemini-specific quirks

**Value:** Validate that Patina's MCP is truly LLM-agnostic.

---

## Enhancements

Nice-to-have work that wasn't required for exit criteria.

### Project-Level Path Consolidation

| Field | Value |
|-------|-------|
| **Origin** | Phase 1 (Folder Restructure) |
| **Spec** | `spec-folder-structure.md` |
| **Why deferred** | 45+ files work today; ship user-level restructure first |
| **When to revisit** | Incrementally, as you touch these files |

**Context:** `paths.rs` was designed with complete API (user + project level) following Eskil philosophy. User-level migrations shipped. Project-level migrations deferred.

**Files to migrate (when touched):**

| Category | Files | Path Functions Needed |
|----------|-------|----------------------|
| **Database** | `scrape/database.rs`, `retrieval/oracles/*.rs`, `retrieval/engine.rs` | `project::db_path()` |
| **Oxidize** | `oxidize/mod.rs`, `oxidize/recipe.rs` | `project::recipe_path()`, `project::model_projections_dir()` |
| **Rebuild** | `rebuild/mod.rs` | `project::data_dir()`, `project::db_path()`, `project::embeddings_dir()` |
| **Project** | `project/internal.rs`, `version.rs` | `project::config_path()`, `project::versions_path()` |
| **Init** | `commands/init/*.rs` | `project::patina_dir()`, `project::config_path()` |
| **Other** | ~30 more files | Various `project::*` |

**Migration pattern:**
```rust
// Before (hardcoded)
let db_path = ".patina/data/patina.db";

// After (using paths module)
use patina::paths::project;
let db_path = project::db_path(project_root);
```

**Value:** Single source of truth for all paths. But existing code works, so migrate opportunistically.

---

### Template Sync Across Projects

| Field | Value |
|-------|-------|
| **Origin** | Phase 2 (Release Automation) |
| **Why deferred** | Part of Phase 2, not blocking release-plz |
| **When to revisit** | After release-plz is working |

**Problem:** Template updates require manual propagation:
1. Edit `resources/claude/*.md`
2. Rebuild patina binary
3. Reinstall patina
4. Run `patina init` in each project

**Proposed solution:** `patina upgrade --templates`
- Sync templates from binary → `~/.patina/adapters/`
- Push templates from mothership → all registered projects
- Requires projects to be registered in `registry.yaml`

**Tasks:**
- [ ] Register projects in `registry.yaml` during `patina init`
- [ ] Add `projects` section to registry (currently only `repos`)
- [ ] Extend `upgrade.rs` to sync templates
- [ ] Add `--templates` flag for template-only sync

**Template flow (current):**
```
resources/claude/    →  compile  →  binary  →  first-run  →  ~/.patina/adapters/  →  patina init  →  project/.claude/
```

**Template flow (with upgrade):**
```
patina upgrade --templates
  ↓
binary embedded templates  →  ~/.patina/adapters/  →  all registered projects
```

**Value:** One command updates templates everywhere.

---

### Ground Truth Expansion

| Field | Value |
|-------|-------|
| **Origin** | Phase 2.7 (Retrieval Quality) |
| **Archived** | `spec/agentic-rag` |
| **Why deferred** | MRR 0.624 met exit criteria with 20 queries |
| **When to revisit** | When retrieval quality degrades or new query types needed |

**Original tasks:**
- Expand dogfood queries from 20 → 50+
- Cover more query types (architecture, debugging, "why" questions)
- Add queries that should hit lexical (exact function names)
- Add queries for layer docs (patterns, philosophy)

**Value:** More robust benchmark, catch regressions earlier.

---

### Hyperparameter Optimization

| Field | Value |
|-------|-------|
| **Origin** | Phase 2.7 (Retrieval Quality) |
| **Archived** | `spec/agentic-rag` |
| **Why deferred** | Current defaults (k=60, fetch_multiplier=2) work well |
| **When to revisit** | When adding new oracles or retrieval quality drops |

**Original tasks:**
- Sweep rrf_k values (20, 40, 60, 80, 100)
- Sweep fetch_multiplier (1, 2, 3, 4)
- Document optimal values with evidence

**Value:** Evidence-based configuration instead of paper defaults.

---

### Second Embedding Model

| Field | Value |
|-------|-------|
| **Origin** | Phase 2.5 (Lab Readiness) |
| **Archived** | `spec/agentic-rag` |
| **Why deferred** | Explicitly marked "Phase 3 - requires model download" |
| **When to revisit** | When exploring smaller/faster models |

**Original tasks:**
- Download alternative model (e.g., bge-small-en-v1.5)
- Test with `patina bench retrieval`
- Compare quality vs speed tradeoffs

**Value:** Validate model flexibility actually works.

---

## Experiments (Tried and Removed)

Work that was implemented, tested, and removed because results didn't support the hypothesis.

### Structural Boost Layer + StructuralOracle

| Field | Value |
|-------|-------|
| **Origin** | Phase 1-2 (Assay Signals → Fusion Integration) |
| **Spec** | `spec-robust-signals.md` |
| **Why removed** | Structural priors don't improve relevance queries; doc_id mismatch prevented fusion |
| **When to revisit** | When implementing query-type routing |

**Two things were tried and removed:**

1. **StructuralOracle** (~170 lines, `src/retrieval/oracles/structural.rs`)
   - Queried module_signals table, returned files ranked by composite_score
   - **Design bug:** returned file-level doc_ids (`./src/main.rs`) when other oracles return symbol-level (`./src/main.rs::fn:main`)
   - RRF fusion can't merge different doc_id granularities

2. **Boost Layer** (~120 lines, `src/retrieval/engine.rs`)
   - Workaround for doc_id mismatch: multiply RRF scores by structural boost
   - Formula: `boosted_score = rrf_score × (1 + boost_factor × composite_score)`
   - Results: 0.1 neutral, 0.5 regression

**Key lesson:**
```
Structural signals are priors (importance), not relevance signals.
```

- Structural signals = P(doc) — "how important is this file in general?"
- Semantic retrieval = P(doc|query) — "how relevant is this to the query?"
- Priors help when relevance is uncertain; they add noise when semantic match is clear
- "Where is X?" queries have clear semantic matches → prior adds noise

**What's preserved:**
- Signal computation: `patina assay derive` (commit_count, is_entry_point, etc.)
- Signal query: `patina assay --query-type derive` via MCP
- module_signals table in database

**Rebuild plan (when query routing exists):**

| Approach | Use Case | Design |
|----------|----------|--------|
| **Orientation mode** | "What's important in this module?" | StructuralOracle as primary, file-level doc_ids OK |
| **Tie-break mode** | "Where is X?" with ambiguous results | Structural features rerank top-N semantic candidates |

**Blockers before rebuild:**
1. Query-type router (detect orientation vs. targeted intent)
2. Explicit fusion level decision (file vs. symbol)
3. Tie-break semantics (only when top candidates are within margin)

---

## Scope Cuts (Recent)

Work cut from recent phases to focus on audit.

### Build Tracking System

| Field | Value |
|-------|-------|
| **Origin** | Phase 4 (planned, never started) |
| **Spec** | `spec-build-system.md` |
| **Why deferred** | Prioritizing code audit before adding new features |
| **When to revisit** | After code audit identifies architectural improvements |

**Original tasks:**
- TOML schema and parser for `.patina/build.toml`
- Query commands: `patina build status/tasks/deferred/explorations`
- Mutation commands: `patina build task start/done/abandon/add`
- Commit integration with trailers (`Task:`, `Deferred:`, `Exploration:`)
- CLAUDE.md integration for LLM guidance

**Value:** Git-native task tracking that LLMs can drive. Exploration/rabbit-hole tracking as first-class citizens.

---

## Enhancements (Recent)

### Assay Snapshot Subcommand

| Field | Value |
|-------|-------|
| **Origin** | Phase 1 (Assay Command) |
| **Spec** | `spec-assay.md` |
| **Why deferred** | Phase 0 covers core use cases; snapshot is edge case for audits |
| **When to revisit** | When doing systematic codebase audits |

**Problem:** Current assay requires multiple queries to get full picture. Snapshot would combine structure + dependencies + git origin + usage in one query.

**Proposed command:**
```bash
patina assay snapshot              # Holistic view
patina assay snapshot --format markdown  # Audit-ready tables
patina assay snapshot --with-origin      # Include git first-commit data
```

**Features:**
- Combines inventory + imports + callers in one output
- Flags unused modules (importer_count = 0)
- Git archaeology with `--with-origin` (first commit, author)
- Markdown output for audit docs

**Value:** One command for full "ore analysis" instead of 6 separate queries. Nice-to-have, not blocking.

---

### Per-Language Module Documentation Extraction

| Field | Value |
|-------|-------|
| **Origin** | Phase 0 (Assay Command) |
| **Spec** | `spec-assay.md` |
| **Why deferred** | Universal-first approach; language-specific logic adds complexity |
| **When to revisit** | After assay is working and tested on multi-language repos |

**Problem:** Each language has different doc comment conventions:

| Language | Convention | Tree-sitter Node |
|----------|------------|------------------|
| Rust | `//!` module doc | `inner_line_doc` |
| Python | `"""docstring"""` | `expression_statement > string` |
| Go | `// Package X` | `comment` before `package` |
| TypeScript | `/** @module */` | `comment` at file start |
| Solidity | `/// @title` NatSpec | `natspec_comment` |
| C/C++ | `/** @file */` | `comment` with `@file` |

**Existing patterns:** Python (`extract_docstring`) and Solidity (`extract_natspec`) already have doc extraction in their language parsers.

**Tasks:**
- [ ] Add `extract_module_doc()` to each language parser
- [ ] Store in new `module_docs` table or column in `index_state`
- [ ] Expose via `patina assay --with-docs`
- [ ] Update MCP tool to include docs in inventory

**Value:** `patina assay` returns module purpose alongside stats, reducing need to read files for basic understanding.

---

## Future Ideas

Ideas captured but never formally planned.

### Capture Automation
Session → persona distillation. When sessions end, automatically extract learnings into persona events.

### Progressive Adapters
Project-specific embedding dimensions. Small adapter layers on frozen E5 for domain-specific retrieval.

### Roadmap as Patina Feature
Structured `roadmap.yaml`, MCP queryable (`patina_roadmap`), git-integrated. Build.md becomes a first-class Patina feature.

### GitHub Adapter
Issues/PRs as knowledge sources. Draft spec exists: [spec-github-adapter.md](spec-github-adapter.md)

---

## Observable Scry Gaps (Dec 2025)

Issues identified during Phase 3 review. Infrastructure is built, but these gaps remain.

### Feedback Capture in LLM Workflow

| Field | Value |
|-------|-------|
| **Origin** | Phase 3 (Observable Scry) |
| **Spec** | `spec-observable-scry.md` |
| **Why gap exists** | LLM frontends (Claude Code) use `Read` tool after scry, not `scry open` |
| **When to revisit** | When designing query routing or LLM integration patterns |

**Problem:** When Claude runs `patina scry "query"` via Bash, then uses the `Read` tool to view a result file, patina never learns which result was used. The `scry open` command bundles open+log, but LLMs don't naturally use it.

**Current state:**
- `scry open/copy/feedback` CLI commands exist but aren't used by LLMs
- MCP `mode: "use"` callback exists but requires explicit LLM action
- Git correlation (`feedback_query_hits`) captures commits, not reads

**Options to explore:**
1. Accept git correlation as sufficient (commits = signal, reads = noise)
2. Instruct LLMs to use `scry open` instead of `Read` (fights natural workflow)
3. Session-level correlation (all queries ↔ all files touched in session)

**Lab data:** `patina eval --feedback` shows 1.1% precision (retrievals → commits). Hits at rank 6-7, not rank 1.

---

### Query Intent Routing

| Field | Value |
|-------|-------|
| **Origin** | Phase 2-3 (Observable Scry) |
| **Spec** | `spec-observable-scry.md` |
| **Why gap exists** | Explicit modes built, auto-routing deferred |
| **When to revisit** | After collecting mode usage data from real sessions |

**Problem:** LLM must manually choose mode (find/orient/recent/why). No automatic detection of query intent.

**Current state:**
- Explicit modes work: `scry orient`, `scry recent`, `scry why`
- Default `find` mode fuses all oracles via RRF
- No signal collection on which mode was used

**Spec note:** "We learn which modes are used → informs future automatic routing." This data isn't being collected yet.

---

### Cross-Repo Validation

| Field | Value |
|-------|-------|
| **Origin** | Phase 3 review |
| **Why gap exists** | Only tested on patina (Rust CLI) |
| **When to revisit** | Before trusting signals on diverse codebases |

**Problem:** Signals (importer_count, activity_level, centrality) are computed but only validated on one repo type.

**Repo styles to test:**
- Python monorepo (different import patterns)
- Go microservices (sparse co-changes)
- TypeScript frontend (npm dependencies)
- Solidity contracts (different structure)

**Validation approach:** Run scry queries on each, manually check if top results are correct.

---

### Lab Query Set

| Field | Value |
|-------|-------|
| **Origin** | Phase 2.7 (Retrieval Quality) |
| **Spec** | `spec/agentic-rag` tag |
| **Why gap exists** | MRR 0.624 met exit criteria with ad-hoc queries |
| **When to revisit** | When systematic benchmarking is needed |

**Problem:** `patina bench retrieval` requires `--query-set <file>` but no query set exists.

**What's needed:**
- Ground truth file: `{ "query": "...", "expected": ["file1", "file2"] }`
- Cover query types: targeted, orientation, "where is X", "what does Y do"
- At least 20-50 queries for statistical significance

---

## How to Use This Spec

**When archiving a phase:**
1. Move unfinished tasks here with context
2. Link to the git tag where full history lives
3. Note why deferred and when to revisit

**When starting new work:**
1. Check this spec for related deferred items
2. Decide: pull into current phase or keep deferred
3. If pulling in, remove from this spec

**This spec is never "done"** - it evolves as work is deferred and later picked up.
