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
patina              # Open project in default frontend
patina claude       # Open in Claude Code
patina gemini       # Open in Gemini CLI
```

**Key Insight:** Source vs Presentation. `.patina/context.md` is committed (source of truth), `CLAUDE.md` is generated and gitignored (presentation).

---

## Specs

- [spec-launcher-architecture.md](../surface/build/spec-launcher-architecture.md) - Overall launcher design
- [spec-template-centralization.md](../surface/build/spec-template-centralization.md) - Template extraction and LLM parity

---

## Tasks

### 1a: Template Centralization
- [ ] Create `resources/gemini/.gemini/` templates (parity with claude)
- [ ] Create `src/adapters/templates.rs` extraction module
- [ ] Extract templates to `~/.patina/adapters/{frontend}/templates/` on first run
- [ ] Refactor claude adapter to copy from central templates
- [ ] Implement gemini adapter with full template support
- [ ] Update init to use `templates::copy_to_project()`

### 1b: First-Run Setup
- [ ] Detect first run → create `~/.patina/`
- [ ] Create workspace folder `~/Projects/Patina`
- [ ] Call `templates::install_all()` to extract embedded templates
- [ ] Detect installed LLM CLIs (enum-based, not manifest files)
- [ ] Set default frontend

### 1c: Launcher Command
- [ ] Remove `Commands::Launch` subcommand
- [ ] `patina [path] [frontend]` as implicit default behavior
- [ ] Parse: frontend names vs subcommands (serve, init, adapter, etc.)
- [ ] Auto-start mothership if not running
- [ ] Prompt `patina init` if not a patina project
- [ ] Ensure adapter templates exist via `templates::copy_to_project()`
- [ ] Launch frontend CLI via `exec`

### 1d: Source/Presentation Model
- [ ] `.patina/context.md` as source of truth (committed)
- [ ] Generate `CLAUDE.md`/`GEMINI.md` on launch from context + persona
- [ ] Combine global persona (`~/.patina/personas/`) + project context
- [ ] Update `.gitignore` for presentation files

### 1e: Branch Model & Safety
- [ ] Refactor `ensure_patina_branch()` to assisted mode (auto-stash, auto-switch)
- [ ] Add `ensure_patina_for_launch()` for launcher branch safety
- [ ] Auto-stash dirty working tree before switch
- [ ] Auto-rebase patina if behind main
- [ ] Show restore hints after stash
- [ ] Keep `--force` for nuclear reset only
- [ ] `.patina/config.toml` with `mode = "owner"` or `"contrib"`

### 1f: Adapter Commands
- [ ] `patina adapter list` - show available frontends
- [ ] `patina adapter default X` - set default frontend
- [ ] Frontend detection via enum

---

## Validation

| Criteria | Status |
|----------|--------|
| Gemini templates exist with full parity to Claude | [ ] |
| First-run extracts templates to `~/.patina/adapters/` | [ ] |
| `patina init --llm=claude` copies from central templates | [ ] |
| `patina init --llm=gemini` copies from central templates | [ ] |
| `patina` (no args) opens default frontend | [ ] |
| `patina claude` opens Claude Code | [ ] |
| `patina gemini` opens Gemini CLI | [ ] |
| `.patina/context.md` generates `CLAUDE.md` on launch | [ ] |
| Presentation files are gitignored | [ ] |
| `patina init` on dirty main auto-stashes and switches | [ ] |

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
