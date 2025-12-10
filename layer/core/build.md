# Build Recipe

**Current Phase:** Phase 0 - main.rs Refactor (then Phase 1 - Launcher & Adapters)

---

## What Patina IS

A local-first RAG network: portable project knowledge + personal mothership.

- **Patina Projects:** `patina init .` - full RAG (semantic, temporal, dependency)
- **Reference Repos:** `patina repo add <url>` - lightweight index in `~/.patina/repos/`
- **Mothership:** `~/.patina/` - registry, personas, `patina serve` daemon

**Completed infrastructure:** Scrape pipeline, oxidize embeddings, query/scry, serve daemon, persona, rebuild command. All working.

---

## Current Goal

`patina` becomes the launcher for AI-assisted development. Like `code .` for VS Code.

```bash
patina              # Open project in default frontend
patina claude       # Open in Claude Code
patina gemini       # Open in Gemini CLI
```

**Key Insight:** Patina is an orchestrator, not a file generator. Embrace existing CLAUDE.md/GEMINI.md files - they're productive for their projects. Patina augments minimally, backs up before modifying, and moves toward MCP as the primary interface.

**Allowed Frontends Model:** Projects control which LLM frontends are permitted via `.patina/config.toml`. Files exist only for allowed frontends. Switching is parallel (allowed frontends coexist), not exclusive.

---

## Specs

- [spec-launcher-architecture.md](../surface/build/spec-launcher-architecture.md) - Overall launcher design
- [spec-template-centralization.md](../surface/build/spec-template-centralization.md) - Template extraction and LLM parity
- [spec-main-refactor.md](../surface/build/spec-main-refactor.md) - CLI dispatcher cleanup

---

## Phase 0: main.rs Refactor (Prerequisite)

**Problem:** `src/main.rs` has grown to 1000 lines with 500 lines of business logic in match arms. This violates `dependable-rust` (small interfaces) and `unix-philosophy` (one job per component).

**Goal:** main.rs becomes a thin dispatcher - CLI definition + 1-line match arms only.

See [spec-main-refactor.md](../surface/build/spec-main-refactor.md) for full details.

### 0a: Extract Adapter Command Handler
- [ ] Create `src/commands/adapter.rs` module
- [ ] Move 170 lines from `Commands::Adapter` match arm
- [ ] main.rs: `Commands::Adapter { command } => commands::adapter::execute(command)?`

### 0b: Extract Scrape Orchestration
- [ ] Create `commands::scrape::execute_all()` function
- [ ] Move inline orchestration (15 lines) from `Commands::Scrape { None }` arm
- [ ] Consolidate scrape subcommand handling

### 0c: Unify Repo Command Types
- [ ] Remove `RepoCommands` enum from main.rs
- [ ] Have `commands::repo` accept clap's parsed args directly
- [ ] Eliminate 48 lines of translation code

### 0d: Type String Enums
- [ ] Create `Dimension` enum with `ValueEnum` derive
- [ ] Create `Llm` enum (claude, gemini, codex, local)
- [ ] Create `DevEnv` enum (docker, dagger, native)
- [ ] Update CLI args to use typed enums

### 0e: Configurable ML Thresholds
- [ ] Add `[search]` section to `ProjectConfig`
- [ ] Move hardcoded `min_score` defaults to config
- [ ] Document why different commands have different defaults

### Phase 0 Validation

| Criteria | Status |
|----------|--------|
| main.rs < 600 lines | [ ] |
| No match arm > 5 lines | [ ] |
| All string enums converted to typed | [ ] |
| `Commands::Adapter` delegated to module | [ ] |
| ML thresholds configurable | [ ] |

---

## Tasks

### 1a: Template Centralization ✓
- [x] Create `resources/gemini/` templates (parity with claude)
- [x] Create `src/adapters/templates.rs` extraction module
- [x] Extract templates to `~/.patina/adapters/{frontend}/templates/` on first run
- [ ] Refactor claude adapter to copy from central templates
- [ ] Implement gemini adapter with full template support
- [ ] Update init to use `templates::copy_to_project()`

### 1b: First-Run Setup ✓
- [x] Detect first run → create `~/.patina/`
- [x] Create workspace folder `~/Projects/Patina`
- [x] Call `templates::install_all()` to extract embedded templates
- [x] Detect installed LLM CLIs (enum-based, not manifest files)
- [x] Set default frontend

### 1c: Launcher Command
- [ ] Remove `Commands::Launch` subcommand
- [ ] `patina [path] [frontend]` as implicit default behavior
- [ ] Parse: frontend names vs subcommands (serve, init, adapter, etc.)
- [ ] Auto-start mothership if not running
- [ ] Prompt `patina init` if not a patina project
- [ ] Ensure adapter templates exist via `templates::copy_to_project()`
- [ ] Launch frontend CLI via `exec`

### 1d: Patina Context Layer
- [ ] Create `.patina/context.md` schema (patina's project knowledge, LLM-agnostic)
- [ ] Detect and preserve existing CLAUDE.md/GEMINI.md (don't clobber)
- [ ] Minimal augmentation: add patina hooks (MCP pointer, layer/ reference) if missing
- [ ] Backup before any modification to `.patina/backups/`
- [ ] Log all actions for transparency

### 1e: Project Config Consolidation & Allowed Frontends
**Background:** Currently `.patina/` has two config files:
- `config.json` (Aug 2025) - project metadata from init (name, llm, dev, environment_snapshot)
- `config.toml` (Nov 2025) - embeddings model selection

**Decision:** Consolidate into unified `.patina/config.toml` with migration support.

- [ ] Create unified `ProjectConfig` struct in `src/project/`
- [ ] Schema: `[project]`, `[dev]`, `[frontends]`, `[embeddings]` sections
- [ ] Migration: detect config.json → merge into config.toml → delete json
- [ ] Update consumers: build.rs, test.rs, docker.rs, doctor.rs
- [ ] Update init command to write unified TOML format
- [ ] Add `[frontends]` section with `allowed` list and `default`
- [ ] `mode = "owner"` - patina artifacts go to main via PR
- [ ] `mode = "contrib"` - CI strips patina artifacts from PRs
- [ ] Enforce allowed frontends on launch (error if not in list)

### 1f: Branch Model & Safety
- [ ] Refactor `ensure_patina_branch()` to assisted mode (auto-stash, auto-switch)
- [ ] Add `ensure_patina_for_launch()` for launcher branch safety
- [ ] Auto-stash dirty working tree before switch
- [ ] Auto-rebase patina if behind main
- [ ] Show restore hints after stash
- [ ] Keep `--force` for nuclear reset only

### 1g: Adapter Commands
- [ ] `patina adapter list` - show allowed + available frontends
- [ ] `patina adapter add <frontend>` - add to allowed, create files
- [ ] `patina adapter remove <frontend>` - backup, remove files, update config
- [ ] `patina adapter default <frontend>` - set project default
- [ ] Frontend detection via enum (global), allowed via config (project)

---

## Validation

| Criteria | Status |
|----------|--------|
| Gemini templates exist with full parity to Claude | [x] |
| First-run extracts templates to `~/.patina/adapters/` | [x] |
| `patina init` copies adapter templates from central location | [ ] |
| `patina` (no args) opens default frontend | [ ] |
| `patina claude` opens Claude Code (if allowed) | [ ] |
| `patina gemini` opens Gemini CLI (if allowed) | [ ] |
| Existing CLAUDE.md preserved, not clobbered | [ ] |
| `.patina/config.toml` has `[project]` and `[frontends]` sections | [ ] |
| `patina adapter add/remove` manages allowed frontends | [ ] |
| Non-allowed frontend files cleaned up | [ ] |
| `patina init` on dirty main auto-stashes and switches | [ ] |
| Backups created before modifying existing files | [ ] |

---

## Future Phases

| Phase | Name | Focus |
|-------|------|-------|
| **2** | MCP Integration | Mothership MCP server, universal tools |
| **3** | Capture Automation | Session → persona distillation |
| **4** | Model Worlds | MLX, code-specific models |

**Future Specs:**
- [spec-github-adapter.md](../surface/build/spec-github-adapter.md) - Has pending work
- [spec-model-runtime.md](../surface/build/spec-model-runtime.md) - Phase 4

---

## Archive

Completed specs preserved via git tags:

```bash
git tag -l 'spec/*'              # List archived specs
git show spec/scry:layer/surface/build/spec-scry.md  # View archived spec
```

Tags: `spec/eventlog-architecture`, `spec/scrape-pipeline`, `spec/oxidize`, `spec/scry`, `spec/lexical-search`, `spec/repo-command`, `spec/serve-command`, `spec/rebuild-command`, `spec/persona-capture`
