# Build Recipe

**Current Phase:** Phase 1 - Folder Restructure

---

## What Patina IS

A local-first RAG network: portable project knowledge + personal mothership.

- **Patina Projects:** `patina init .` - full RAG (semantic, temporal, dependency)
- **Reference Repos:** `patina repo add <url>` - lightweight index in `~/.patina/repos/`
- **Mothership:** `~/.patina/` - registry, personas, `patina serve` daemon

**Completed infrastructure:** Scrape pipeline, oxidize embeddings, query/scry, serve daemon, persona, rebuild command, MCP server, hybrid retrieval (MRR 0.624). All working.

---

## Specs

Active specs for current work:

- [spec-folder-structure.md](../surface/build/spec-folder-structure.md) - Folder structure design (project + user level)

Deferred work (with context for why/when):

- [spec-work-deferred.md](../surface/build/spec-work-deferred.md) - Scope cuts, blockers, enhancements, future ideas

Future specs (not yet planned):

- [spec-github-adapter.md](../surface/build/spec-github-adapter.md) - GitHub integration
- [spec-model-runtime.md](../surface/build/spec-model-runtime.md) - Model flexibility

---

## Phase 1: Folder Restructure

**Goal:** Clean separation of source vs derived data at user level (`~/.patina/`).

**Spec:** [spec-folder-structure.md](../surface/build/spec-folder-structure.md)

**Key Insight:** Patina coats projects, not users.
- `layer/` = accumulated patina on a project (stays at project root)
- `~/.patina/` = user's tools and preferences (restructure needed)

### Target Structure

```
~/.patina/
├── personas/default/events/     # Source ONLY (valuable)
├── cache/                       # ALL rebuildable data
│   ├── repos/                   # Git clones
│   └── personas/default/        # Materialized indices
└── registry.yaml                # Source (valuable)
```

**Backup story:** `~/.patina/` minus `cache/` = everything valuable.

### Code Design: Centralized Paths Module

**Solution:** Single `src/paths.rs` module owns all filesystem layout decisions.

```rust
// src/paths.rs - Single source of truth
pub fn patina_home() -> PathBuf { ~/.patina/ }
pub fn patina_cache() -> PathBuf { ~/.patina/cache/ }

pub mod persona {
    pub fn events_dir() -> PathBuf { ~/.patina/personas/default/events/ }
    pub fn cache_dir() -> PathBuf { ~/.patina/cache/personas/default/ }
}

pub mod repos {
    pub fn cache_dir() -> PathBuf { ~/.patina/cache/repos/ }
}
```

**Alignment with core values:**
- **dependable-rust:** Small interface, one file shows entire layout
- **unix-philosophy:** One job (define paths), no I/O or business logic

### Tasks

#### 1a: Create `src/paths.rs` Module
- [ ] Create `src/paths.rs` with `patina_home()`, `patina_cache()`
- [ ] Add `persona::events_dir()`, `persona::cache_dir()`
- [ ] Add `repos::cache_dir()`
- [ ] Export from `src/lib.rs`

#### 1b: Update Persona to Use Paths Module
- [ ] Replace `persona_dir()` with `paths::persona::events_dir()`
- [ ] Replace `persona_dir().join("materialized")` with `paths::persona::cache_dir()`
- [ ] Update `materialize()`, `query()`, `list()`
- [ ] Test: `patina persona materialize` && `patina persona query`

#### 1c: Update Repo Paths
- [ ] Find all code referencing `~/.patina/repos/`
- [ ] Update to use `paths::repos::cache_dir()`
- [ ] Test: `patina repo` commands still work

#### 1d: Migration Logic
- [ ] Add `paths::migrate_if_needed()` function
- [ ] Detect old paths → move to new locations
- [ ] Print migration message so user knows what happened

#### 1e: Cleanup
- [ ] Delete stale `.patina/patina.db` (0 bytes) at project level
- [ ] **USER DECISION:** Evaluate `~/.patina/claude-linux/` - keep/delete/archive?
- [ ] Remove old `persona_dir()` function

### Validation

| Criteria | Status |
|----------|--------|
| `src/paths.rs` exists with clear module structure | [ ] |
| All path logic centralized | [ ] |
| `patina persona materialize` uses `cache/personas/` | [ ] |
| `patina persona query` uses `cache/personas/` | [ ] |
| Old paths auto-migrated | [ ] |
| `claude-linux/` evaluated (user decision) | [ ] |

---

## Completed

Shipped phases (details preserved in git tags):

### Launcher & Adapters
Template centralization, first-run setup, launcher command (`patina` / `patina -f claude`), config consolidation (`.patina/config.toml`), branch safety (auto-stash, auto-switch), adapter commands.

**Tags:** `spec/launcher-architecture`, `spec/template-centralization`

### Agentic RAG
Oracle abstraction (semantic, lexical, persona), hybrid retrieval + RRF fusion (k=60), MCP server (`patina serve --mcp`), `patina_query` and `patina_context` tools.

**Metrics:** MRR 0.624, Recall@10 67.5%, Latency ~135ms

**Includes:** Lab infrastructure (benchmarks, config), retrieval quality fixes (FTS5, layer docs), multi-project federation.

**Tags:** `spec/agentic-rag`

---

## Archive

Completed specs preserved via git tags:

```bash
git tag -l 'spec/*'              # List archived specs
git show spec/scry:layer/surface/build/spec-scry.md  # View archived spec
```

**Tags:** `spec/eventlog-architecture`, `spec/scrape-pipeline`, `spec/oxidize`, `spec/scry`, `spec/lexical-search`, `spec/repo-command`, `spec/serve-command`, `spec/rebuild-command`, `spec/persona-capture`, `spec/main-refactor`, `spec/launcher-architecture`, `spec/template-centralization`, `spec/agentic-rag`
