# Spec: Deferred Work

**Purpose:** Capture work that was planned but consciously deferred, with context for why and when to revisit.

**Pattern:** When archiving completed phases, unfinished items move here instead of disappearing into git history.

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
