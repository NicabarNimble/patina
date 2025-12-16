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
| **Why deferred** | Prioritized `patina_query` and `patina_context` first |
| **When to revisit** | When starting capture automation |

**Original tasks:**
- `patina_session_start` - Start tracked session via MCP
- `patina_session_note` - Capture insight via MCP
- `patina_session_end` - End session and archive via MCP

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
