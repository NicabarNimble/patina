# Build Recipe

**Current Phase:** Phase 2 - Release Automation & Template Sync

---

## What Patina IS

A local-first RAG network: portable project knowledge + personal mothership.

- **Patina Projects:** `patina init .` - full RAG (semantic, temporal, dependency)
- **Reference Repos:** `patina repo add <url>` - lightweight index in `~/.patina/cache/repos/`
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

**Philosophy (from rationale-eskil-steenberg.md):**
> "It's faster to write 5 lines of code today than to write 1 line today and edit it later."

The paths module should be **complete from day one** - defining ALL Patina paths (user-level AND project-level). Not minimal, not clever, but correct.

**Solution:** Single `src/paths.rs` module owns ALL filesystem layout decisions.

```rust
// src/paths.rs - Single source of truth for ALL Patina filesystem layout

// === User Level (~/.patina/) ===
pub fn patina_home() -> PathBuf { ~/.patina/ }
pub fn patina_cache() -> PathBuf { ~/.patina/cache/ }
pub fn config_path() -> PathBuf { ~/.patina/config.toml }
pub fn registry_path() -> PathBuf { ~/.patina/registry.yaml }
pub fn adapters_dir() -> PathBuf { ~/.patina/adapters/ }

pub mod persona {
    pub fn events_dir() -> PathBuf { ~/.patina/personas/default/events/ }
    pub fn cache_dir() -> PathBuf { ~/.patina/cache/personas/default/ }
}

pub mod repos {
    pub fn cache_dir() -> PathBuf { ~/.patina/cache/repos/ }
}

// === Project Level (project/.patina/) ===
pub mod project {
    pub fn patina_dir(root: &Path) -> PathBuf { .patina/ }
    pub fn config_path(root: &Path) -> PathBuf { .patina/config.toml }
    pub fn data_dir(root: &Path) -> PathBuf { .patina/data/ }
    pub fn db_path(root: &Path) -> PathBuf { .patina/data/patina.db }
    pub fn embeddings_dir(root: &Path) -> PathBuf { .patina/data/embeddings/ }
    pub fn model_projections_dir(root: &Path, model: &str) -> PathBuf { ... }
    pub fn recipe_path(root: &Path) -> PathBuf { .patina/oxidize.yaml }
    pub fn versions_path(root: &Path) -> PathBuf { .patina/versions.json }
    pub fn backups_dir(root: &Path) -> PathBuf { .patina/backups/ }
}
```

**Full design:** See [spec-folder-structure.md](../surface/build/spec-folder-structure.md)

**Alignment with core values:**
- **eskil-steenberg:** Complete from day one, never needs to change
- **dependable-rust:** Small interface, one file shows ENTIRE layout
- **unix-philosophy:** One job (define paths), no I/O or business logic

### Tasks

**Approach:** Design complete API, ship user-level restructure, iterate on project-level.

#### 1a: Create `src/paths.rs` Module
- [x] Create `src/paths.rs` with complete API (user + project level)
- [x] User-level: `patina_home()`, `patina_cache()`, `config_path()`, `registry_path()`, `adapters_dir()`
- [x] User-level: `persona::events_dir()`, `persona::cache_dir()`, `repos::cache_dir()`
- [x] Project-level: `project::*` (full API ready for future use)
- [x] Export from `src/lib.rs`

#### 1b: Update Persona Paths
- [x] Replace `persona_dir()` in `src/commands/persona/mod.rs`
- [x] Replace hardcoded path in `src/retrieval/oracles/persona.rs`
- [x] Update `note()`, `materialize()`, `query()`, `list()`
- [x] Test: `patina persona materialize` && `patina persona query`

#### 1c: Update Repo & Registry Paths
- [x] Replace `repos_dir()`, `mothership_dir()`, `registry_path()` in `repo/internal.rs`
- [x] Remove old functions
- [x] Test: `patina repo list`, `patina repo add`

#### 1d: Update Workspace & Adapters Paths
- [x] Replace path functions in `workspace/internal.rs`
- [x] Replace `workspace::adapters_dir()` in `adapters/templates.rs`
- [x] Update `workspace/mod.rs` re-exports
- [x] Remove old path functions, keep behavior functions
- [x] Delete unused `projects_dir()`
- [x] Test: `patina` launcher, first-run setup

#### 1e: Migration Logic
- [x] Create `src/migration.rs`
- [x] Add `migrate_if_needed()` - move old paths to new `cache/` locations
- [x] Print migration message
- [x] Call from startup (main.rs)

#### 1f: Ship It
- [x] Delete stale `.patina/patina.db` (0 bytes)
- [x] `~/.patina/claude-linux/` - removed
- [x] Run test suite
- [x] Build release, test with live install
- [x] Commit and tag

### Validation (Exit Criteria)

| Criteria | Status |
|----------|--------|
| `src/paths.rs` exists with complete API | [x] |
| `src/migration.rs` exists | [x] |
| All user-level path functions consolidated | [x] |
| `patina persona materialize` writes to `cache/personas/` | [x] |
| `patina persona query` reads from `cache/personas/` | [x] |
| `patina repo` commands work | [x] |
| `patina` launcher works | [x] |
| Old paths auto-migrated | [x] |
| `claude-linux/` evaluated | [x] removed |

### Deferred

Project-level path consolidation deferred to [spec-work-deferred.md](../surface/build/spec-work-deferred.md).

**Why:** The 45+ files with hardcoded project paths work today. Migrate them incrementally as we touch those files.

---

## Phase 2: Release Automation & Template Sync

**Goal:** Proper versioning with release-plz, template propagation to projects.

### Context

**Problem 1: No release automation** ✅ SOLVED
- Was: 943 commits, no GitHub releases
- Now: v0.1.0 released, release-plz automates future releases
- Conventional commits (`feat:`, `fix:`) trigger Release PRs automatically

**Problem 2: Template updates don't propagate** (deferred)
- Templates live in 3 places: `resources/` → binary → `~/.patina/adapters/` → projects
- Changing a template requires: edit source, rebuild, reinstall, re-init each project
- Future: `patina upgrade --templates` to sync to all registered projects

### Solution: release-plz

**Why release-plz** (researched in session 20251216):
- Rust-native, zero-config
- Used by mise (popular Rust tool)
- 1.2k stars, 291 releases, actively maintained
- Supports GitHub, GitLab, Gitea
- Works with conventional commits (we're 85% consistent already)

**How it works:**
1. Push to main with conventional commits (`feat:`, `fix:`, `docs:`)
2. release-plz creates a "Release PR" with version bump + changelog
3. Merge PR → GitHub Release created automatically

### Tasks

#### 2a: Commit Template Improvements
- [x] Commit `resources/claude/session-update.md` (added "Discussion context" bullet)
- [x] Rebuild patina binary (`cargo build --release && cargo install --path .`)

#### 2b: Add release-plz Workflow
- [x] Create `.github/workflows/release-plz.yml`
- [x] Create `release-plz.toml` with `publish = false` (GitHub only, no crates.io)
- [x] Test workflow runs on push to main (PR #60)

#### 2c: Create v0.1.0 Baseline Release
- [x] Create git tag `v0.1.0`
- [x] Create GitHub release with changelog summarizing all work to date
- [x] Changelog covers: scrape, oxidize, scry, persona, MCP, hybrid retrieval, folder restructure

#### 2d: Template Sync Command
- Deferred to [spec-work-deferred.md](../surface/build/spec-work-deferred.md#template-sync-across-projects)
- Not blocking release automation

### Validation (Exit Criteria)

| Criteria | Status |
|----------|--------|
| `resources/claude/session-update.md` committed | [x] |
| `.github/workflows/release-plz.yml` exists | [x] |
| `release-plz.toml` configured (GitHub only) | [x] |
| v0.1.0 GitHub release created | [x] |
| release-plz workflow runs on push to main | [x] |
| `patina upgrade --templates` syncs to projects | deferred |

### Research Notes (Session 20251216)

**Tools compared:**
| Tool | Stars | Used By | Approach |
|------|-------|---------|----------|
| release-plz | 1.2k | mise | Rust-native, auto |
| release-please | 6.2k | Google | Multi-lang, auto |
| cargo-release | 1.5k | Dojo | Rust, semi-manual |
| tinysemver | 27 | USearch | Multi-lang, auto |
| Manual tags | n/a | ethrex, starknet-foundry | Full control |

**Why blockchain projects use manual tags:** Coordination, security review, breaking changes matter more.

**Why patina uses auto:** Dev tool, no downstream dependencies, ship fast.

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
