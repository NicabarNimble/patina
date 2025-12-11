# Build Recipe

**Current Phase:** Phase 1 - Launcher & Adapters

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
patina              # Open project in default frontend (current dir)
patina -f claude    # Explicit frontend (short flag)
patina --frontend gemini  # Explicit frontend (long flag)
```

**Syntax:** Frontend is a flag (`-f`/`--frontend`), not a positional argument. No path argument - always operates on current directory.

**Key Insight:** Patina is an orchestrator, not a file generator. Embrace existing CLAUDE.md/GEMINI.md files - they're productive for their projects. Patina augments minimally, backs up before modifying, and moves toward MCP as the primary interface.

**Allowed Frontends Model:** Projects control which LLM frontends are permitted via `.patina/config.toml`. Files exist only for allowed frontends. Switching is parallel (allowed frontends coexist), not exclusive.

**"Are You Lost?" Prompt:** Running `patina` in a non-patina project shows helpful context (path, git info, remote) and asks to initialize. Default to contrib mode.

---

## Specs

- [spec-launcher-architecture.md](../surface/build/spec-launcher-architecture.md) - Overall launcher design
- [spec-template-centralization.md](../surface/build/spec-template-centralization.md) - Template extraction and LLM parity

---

## Phase 1 Tasks

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

### 1c: Launcher Command ✓
**New design:** Frontend via flag (`-f`/`--frontend`), no path argument, "Are you lost?" prompt.

**CLI structure (implemented):**
```rust
#[derive(Parser)]
struct Cli {
    #[arg(short = 'f', long = "frontend", global = true)]
    frontend: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}
// When command is None → launcher mode (calls launch::execute)
```

**Completed** (session 20251210-152252, 20251211-061558):
- [x] Refactor CLI to use `-f` flag for frontend selection
- [x] Make `command` optional - no subcommand = launcher mode
- [x] Auto-start mothership if not running
- [x] Launch frontend CLI via `exec`
- [x] Remove `Commands::Launch` subcommand (redundant)
- [x] "Are you lost?" prompt for non-patina projects
  - [x] Show: path, git branch+status, remote URL
  - [x] Single y/N question to initialize
  - [x] Auto-init on confirmation

**Remaining:**
- [ ] Ensure adapter templates exist via `templates::copy_to_project()`

### 1d: Patina Context Layer
- [ ] Create `.patina/context.md` schema (patina's project knowledge, LLM-agnostic)
- [x] Detect and preserve existing CLAUDE.md/GEMINI.md (don't clobber)
- [ ] Minimal augmentation: add patina hooks (MCP pointer, layer/ reference) if missing
- [x] Backup infrastructure exists (`project::backup_file()`)
- [ ] Log all actions for transparency

### 1e: Project Config Consolidation & Allowed Frontends ✓
**Background:** Consolidated two config files into unified `.patina/config.toml`.

**Completed** (session 20251210-094521, 11 commits):
- [x] Create unified `ProjectConfig` struct in `src/project/`
- [x] Schema: `[project]`, `[dev]`, `[frontends]`, `[embeddings]` sections
- [x] Migration: detect config.json → merge into config.toml → delete json
- [x] Update consumers: build.rs, test.rs, docker.rs, doctor.rs
- [x] Update init command to write unified TOML format
- [x] Add `[frontends]` section with `allowed` list and `default`
- [x] Enforce allowed frontends on launch (error if not in list)

**Remaining** (move to 1f):
- [ ] `mode = "owner"` - patina artifacts go to main via PR
- [ ] `mode = "contrib"` - CI strips patina artifacts from PRs

### 1f: Branch Model & Safety
**Philosophy:** Do and Inform (not warn and block)

**Branch scenarios:**
| Current | Patina Exists | Tree | Action |
|---------|---------------|------|--------|
| patina | - | clean | Proceed |
| patina | - | dirty | Proceed |
| patina (behind) | - | any | Auto-rebase |
| other | yes | clean | Auto-switch |
| other | yes | dirty | Stash → switch → hint |
| other | no | any | "Are you lost?" |
| (not git) | - | - | "Init git?" prompt |

**Completed** (session 20251211-061558):
- [x] `ensure_on_patina_branch()` - auto-stash, auto-switch, auto-rebase
- [x] Stash with named message: `patina-autostash-{timestamp}`
- [x] Show restore hint after stash: `git checkout X && git stash pop`
- [x] Handle rebase conflicts (stop, show instructions)
- [x] `--force` flag for `patina init` (nuclear reset, backup old branch) - already existed

**Remaining:**
- [ ] Handle stash failures (untracked files conflict) - partial, needs better error handling
- [ ] `mode = "owner"` - patina artifacts go to main via PR
- [ ] `mode = "contrib"` - CI strips patina artifacts from PRs

### 1g: Adapter Commands ✓
- [x] `patina adapter list` - show allowed + available frontends
- [x] `patina adapter add <frontend>` - add to allowed, create files
- [x] `patina adapter remove <frontend>` - backup, remove files, update config
- [x] `patina adapter default <frontend>` - set project default
- [x] Frontend detection via enum (global), allowed via config (project)

---

## Validation

| Criteria | Status |
|----------|--------|
| Gemini templates exist with full parity to Claude | [x] |
| First-run extracts templates to `~/.patina/adapters/` | [x] |
| `patina init` copies adapter templates from central location | [ ] |
| `patina` (no args) opens default frontend | [x] |
| `patina -f claude` opens Claude Code (if allowed) | [x] |
| `patina -f gemini` opens Gemini CLI (if allowed) | [ ] |
| "Are you lost?" prompt for non-patina projects | [x] |
| Auto-init on confirmation | [x] |
| Auto-stash on dirty working tree (with restore hint) | [x] |
| Auto-switch to patina branch | [x] |
| Auto-rebase if patina behind main | [x] |
| Existing CLAUDE.md preserved | [x] |
| `.patina/config.toml` has `[project]` and `[frontends]` sections | [x] |
| `patina adapter add/remove` manages allowed frontends | [x] |
| Backups created before modifying existing files | [x] |

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

Tags: `spec/eventlog-architecture`, `spec/scrape-pipeline`, `spec/oxidize`, `spec/scry`, `spec/lexical-search`, `spec/repo-command`, `spec/serve-command`, `spec/rebuild-command`, `spec/persona-capture`, `spec/main-refactor`
