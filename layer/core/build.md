# Build Recipe

**Current Phase:** Phase 2 Complete - Ready for Lab & Production Testing

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
- [x] Fix template structure: install to `.{frontend}/` subdirectory for copy_to_project()
- [x] Implement gemini adapter with full template support (uses templates::copy_to_project)
- [x] `patina adapter add` creates adapter files from templates
- [-] Refactor claude adapter - kept as-is (embedded approach works, has version management)

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

**Completed** (session 20251211-103012):
- [x] Ensure adapter templates exist via `templates::copy_to_project()` (in adapter add)

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

**Completed** (session 20251211-081016):
- [x] Remove `mode` field - replaced with `[upstream]` section
- [x] Add `[upstream]` section: `repo`, `branch`, `remote` for contribution PRs
- [x] Add `[ci]` section: `checks` (pre-PR commands), `branch_prefix`
- [x] LLM-driven PR workflow - config provides metadata, LLM handles git

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

**Completed** (session 20251211-081016):
- [x] Handle stash failures - `--include-untracked` flag captures all changes
- [x] Replaced mode concept with `[upstream]` config for LLM-driven PRs

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
| `patina adapter add` copies templates from central location | [x] |
| `patina` (no args) opens default frontend | [x] |
| `patina -f claude` opens Claude Code (if allowed) | [x] |
| `patina -f gemini` opens Gemini CLI (if allowed) | [x] |
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

## Phase 2: Agentic RAG

**Goal:** Transform Patina from a tool provider into an intelligent retrieval layer via MCP.

**Key Insight:** No local LLM needed for routing. Research shows parallel retrieval + RRF fusion + frontier LLM synthesis beats small-model routing.

**Spec:** [spec-agentic-rag.md](../surface/build/spec-agentic-rag.md)

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    patina serve --mcp                           │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Query Processor (NO LLM)                                 │  │
│  │  - Parallel oracle dispatch                               │  │
│  │  - RRF fusion (k=60)                                      │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│       ┌──────────────────────┼──────────────────────┐          │
│       ▼                      ▼                      ▼          │
│  ┌─────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
│  │ Semantic    │  │ Lexical         │  │ Session         │     │
│  │ (E5+USearch)│  │ (BM25/FTS5)     │  │ (Persona)       │     │
│  └─────────────┘  └─────────────────┘  └─────────────────┘     │
│                              ▼                                  │
│                    RRF Fusion → Top-K                           │
│                              ▼                                  │
│                    MCP Response (JSON-RPC)                      │
└─────────────────────────────────────────────────────────────────┘
```

### Phase 2 Tasks

#### 2a: Oracle Abstraction ✓
- [x] Create `Oracle` trait in `src/retrieval/oracle.rs` (strategy pattern, not adapter)
- [x] Wrap existing scry functions as oracle implementations
- [x] Add parallel query execution with rayon

#### 2b: Hybrid Retrieval + RRF ✓
- [x] Run semantic + BM25 in parallel for every query
- [x] Implement RRF fusion (k=60) in `src/retrieval/fusion.rs`
- [x] Cross-oracle deduplication for proper RRF boosting

#### 2c: MCP Server ✓
- [x] Add `--mcp` flag to `patina serve`
- [x] JSON-RPC over stdio transport (hand-rolled, no external SDK)
- [x] JSON-RPC 2.0 version validation
- [x] Tool: `patina_query` with hybrid retrieval
- [x] Rich output format (file path, event type, timestamp)
- [x] `patina adapter mcp claude` one-command setup
- [ ] Tool: `patina_context` (project rules/patterns)
- [ ] Session tools: `patina_session_start/end/note`

#### 2d: Integration Testing
- [ ] Test with Claude Code MCP config (immediate next step)
- [ ] Test with Gemini CLI (when MCP supported)
- [ ] Latency benchmarks (<500ms target)

### Validation

| Criteria | Status |
|----------|--------|
| `patina serve --mcp` starts MCP server | [x] |
| `patina adapter mcp claude` configures Claude Code | [x] |
| Claude Code can call `patina_query` tool | [ ] needs live test |
| Returns fused results (semantic + lexical + persona) | [x] |
| Output includes metadata (path, event_type, timestamp) | [x] |
| Latency < 500ms for typical query | [ ] needs benchmark |
| Session tools work via MCP | [ ] not yet implemented |

---

## Phase 2.5: Lab Readiness

**Goal:** Enable experimentation (new models, fusion strategies) while keeping Patina running in production.

**Philosophy:** Patina is not an academic exercise. It runs daily on hackathons, bounties, and repo contributions while we experiment with its internals.

### Current State Assessment

| Component | Production | Lab Ready | Blocker |
|-----------|------------|-----------|---------|
| Scrape pipeline | ✅ | ⚠️ | Schema hardcoded |
| Embeddings (E5) | ✅ | ❌ | Model locked in code |
| Retrieval/Oracles | ✅ | ✅ | - |
| Fusion (RRF) | ✅ | ⚠️ | k=60 hardcoded |
| MCP server | ✅ | ✅ | - |
| Persona | ✅ | ⚠️ | Storage format locked |
| Frontends | ✅ | ✅ | Adapter pattern works |

### Phase 2.5 Tasks

#### 2.5a: Retrieval Configuration
- [ ] Add `[retrieval]` section to config.toml
- [ ] Make RRF k value configurable (default 60)
- [ ] Make fetch_multiplier configurable (default 2x)
- [ ] Config validation on load

#### 2.5b: Benchmark Infrastructure
- [ ] `patina bench retrieval` command skeleton
- [ ] Query set format (JSON with ground truth)
- [ ] Metrics: MRR, Recall@K, latency p50/p95
- [ ] Baseline measurement before changes

#### 2.5c: Model Flexibility
- [ ] Document model addition process
- [ ] Test with second embedding model (bge-small or nomic)
- [ ] Verify vector space compatibility

### Validation

| Criteria | Status |
|----------|--------|
| Can change RRF k via config | [ ] |
| `patina bench` produces metrics | [ ] |
| Second embedding model works | [ ] |
| No regression in production use | [ ] |

---

## Future Phases

| Phase | Name | Focus |
|-------|------|-------|
| **3** | Capture Automation | Session → persona distillation |
| **4** | Progressive Adapters | Project-specific embedding dimensions |

### Phase 4: Progressive Adapters

**Goal:** Improve retrieval quality with learned project-specific embeddings.

**Concept:** Small adapter layers (~1-2M params) on frozen E5-base-v2. NOT fine-tuning.

- Preserves E5 quality (trained on billions of pairs)
- Data efficient: 10K pairs vs 100K+ for fine-tuning
- Fast training: hours on Mac Studio
- Extensible: add dimensions without retraining existing

**Six planned dimensions:**
1. Semantic (768-dim) - session observations *(exists)*
2. Temporal (256-dim) - git co-change *(exists)*
3. Dependency (256-dim) - call graph *(exists)*
4. Syntactic (256-dim) - AST similarity
5. Architectural (256-dim) - directory structure
6. Social (256-dim) - GitHub metadata

**Key References:**
- [architecture-patina-embedding.md](../surface/architecture-patina-embedding.md) - Full spec
- Sessions: 20251120-110914 (vision), 20251121-042111 (implementation)

**Future Specs:**
- [spec-github-adapter.md](../surface/build/spec-github-adapter.md) - Has pending work

---

## Archive

Completed specs preserved via git tags:

```bash
git tag -l 'spec/*'              # List archived specs
git show spec/scry:layer/surface/build/spec-scry.md  # View archived spec
```

Tags: `spec/eventlog-architecture`, `spec/scrape-pipeline`, `spec/oxidize`, `spec/scry`, `spec/lexical-search`, `spec/repo-command`, `spec/serve-command`, `spec/rebuild-command`, `spec/persona-capture`, `spec/main-refactor`
