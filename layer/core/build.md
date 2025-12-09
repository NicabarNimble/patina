# Build Recipe

Persistent roadmap across sessions. **Start here when picking up work.**

---

## What Patina IS

A local-first RAG network: **portable project knowledge + personal mothership**.

### Two-Tier Architecture

**Patina Projects** (code you work on):
```bash
patina init .                    # Any repo you work on (owner or contributor)
```
- `layer/` = git-tracked knowledge (sessions, patterns)
- `.patina/` = local indices (db, embeddings) → rebuilt via `patina rebuild`
- Full RAG: semantic, temporal, dependency dimensions
- Owner vs contributor = git remote config, not patina concern

**Reference Repos** (read-only knowledge):
```bash
patina repo add <url>            # Code you learn from, not work on
```
- Lives in `~/.patina/repos/`
- Lightweight index: code AST, call graph, FTS5
- No `layer/`, no sessions
- Dependency dimension only (no temporal without full git history)

**Mothership** (`~/.patina/`):
- `registry.yaml` = all known projects and reference repos
- `personas/` = user knowledge that informs LLM responses (see 4d)
- `patina serve` = daemon for cross-project queries

See: [rag-network.md](../surface/rag-network.md)

---

## Current Direction (2025-12-08)

**Goal:** Cross-project knowledge that helps win hackathons.

### Completed Infrastructure
| Phase | Name | Status |
|-------|------|--------|
| **1** | Scrape Pipeline | ✅ Complete |
| **2** | Oxidize (Embeddings) | ✅ Complete |
| **3** | Query Infrastructure | ✅ Complete (8.6x vs random) |
| **4** | Solid Foundation | ✅ Complete |

### Next: Make LLMs Use It
| Phase | Name | Focus |
|-------|------|-------|
| **5** | Launcher & Adapters | `patina` as entry point, adapter system |
| **6** | MCP Integration | Mothership MCP server, universal tools |
| **7** | Capture Automation | Session → persona distillation |
| **8** | Model Worlds | MLX, code-specific models |

**The Gap:** Users run `claude` or `gemini` directly. Patina should be the launcher. Phase 5 makes `patina` the entry point, Phase 6 wires MCP for universal tool access.

**Specs:**
- [spec-launcher-architecture.md](../surface/build/spec-launcher-architecture.md) - launcher & adapter design
- [spec-rebuild-command.md](../surface/build/spec-rebuild-command.md) - for projects
- [spec-repo-command.md](../surface/build/spec-repo-command.md) - for reference repos
- [spec-serve-command.md](../surface/build/spec-serve-command.md)
- [spec-persona-capture.md](../surface/build/spec-persona-capture.md)

---

## Phase 3: Query Infrastructure ✅

**Goal:** Semantic + lexical search across code, sessions, and external repos.

#### 3a: File-Based Scry Queries ✅
**Status:** Complete (2025-11-25)
**Spec:** [spec-scry.md](../surface/build/spec-scry.md)

- [x] Add `--file` flag to scry command
- [x] Direct file vector lookup (no re-embedding needed)
- [x] Return co-changing files with scores
- [x] Works for temporal and future dependency dimensions

```bash
patina scry --file src/auth/login.rs    # What files change with this?
patina scry --file contracts/Game.cairo --dimension temporal
```

#### 3b: FTS5 Lexical Search ✅
**Status:** Complete (2025-11-25)
**Spec:** [spec-lexical-search.md](../surface/build/spec-lexical-search.md)

- [x] Add FTS5 virtual table to patina.db schema
- [x] Index code content, symbols, file paths
- [x] Auto-detect exact match queries in scry
- [ ] Return highlighted snippets (deferred - basic results work)

```bash
patina scry "find spawn_entity"         # Exact match via FTS5
patina scry "error handling patterns"   # Semantic via vectors
```

#### 3c: Repo Command (Cross-Project Knowledge)
**Status:** ✅ MVP Complete (2025-11-26)
**Spec:** [spec-repo-command.md](../surface/build/spec-repo-command.md)
**Why:** Query external repos for patterns and code understanding

- [x] `patina repo <url>` - clone, scaffold, scrape to `~/.patina/repos/`
- [x] `patina repo list` - show registered repos
- [x] `patina repo update <name>` - pull + rescrape
- [x] `patina scry "query" --repo <name>` - query external repo
- [x] Registry persistence (`~/.patina/registry.yaml`)
- [ ] `--contrib` fork mode (partial, gh cli dependency)

```bash
patina repo dojoengine/dojo              # Add repo
patina scry "spawn patterns" --repo dojo # Query it
patina repo update dojo                  # Refresh later
```

**Future (Phase 2+):** gRPC daemon for container queries, persona beliefs

#### 3d: Dependency Dimension
**Status:** ✅ Complete (2025-12-03)
**Spec:** [spec-oxidize.md](../surface/build/spec-oxidize.md)
**Why:** Claude needs call graph understanding for code changes

- [x] Create `src/commands/oxidize/dependency.rs`
- [x] Training pairs from call_graph (7,151 relationships, 4,004 functions)
- [x] Caller/callee = related signal
- [x] Query via `patina scry "function name" --dimension dependency`

#### 3e: GitHub Integration (Issues MVP)
**Status:** ✅ Complete (2025-12-03)
**Spec:** [spec-github-adapter.md](../surface/build/spec-github-adapter.md)
**Why:** Hackathon context from issues/discussions

**Key Insight:** GitHub issues use SAME semantic space as code (E5 → MLP → 256-dim). Query "entity spawning" returns both code AND related issues.

**What's Working:**
- [x] Create `src/commands/scrape/github/mod.rs`
- [x] Add `github_issues` materialized view schema
- [x] Implement `gh issue list --json` integration
- [x] Add `--with-issues` flag to `patina repo add`
- [x] Add `github.issue` events to FTS5 index
- [x] Add `--include-issues` flag to `patina scry`
- [x] Graceful fallback to FTS5 when semantic index missing

```bash
patina repo add dojoengine/dojo --with-issues
patina scry "spawn entity" --repo dojo --include-issues
```

**Deferred (use-case features, not core):**
- Bounty detection removed from core (2025-12-06) - future plugin
- Semantic embeddings for issues (depends on external repo oxidize)
- PRs + Discussions integration

#### 3f: Mothership Daemon (`patina serve`)
**Status:** Core Complete (2025-12-08), future APIs deferred
**Spec:** [spec-serve-command.md](../surface/build/spec-serve-command.md)
**Why:** Container queries to Mac, hot model caching, Ollama-style daemon

**Architecture:**
```
Mac (Mothership)                    Container
┌─────────────────────┐            ┌─────────────────────┐
│ patina serve        │            │ patina scry "query" │
│ localhost:50051     │◄───────────│ PATINA_MOTHERSHIP   │
│                     │   HTTP     │ =host.docker.internal│
│ ┌─────────────────┐ │            └─────────────────────┘
│ │ E5 Model (hot)  │ │
│ │ Projections     │ │
│ │ ~/.patina/repos │ │
│ └─────────────────┘ │
└─────────────────────┘
```

**Key Design Decisions:**
- **rouille** (blocking HTTP, no async/tokio) - thread-per-request
- **Ollama pattern** - `patina serve` subcommand, single binary
- **HTTP REST** on port 50051 (not gRPC) - simpler, curl-friendly
- **Lazy model loading** - load E5 on first request, keep hot

**Implementation Phases:**

**Phase 1: Basic Daemon** ✅ (2025-12-03)
- [x] Add `rouille = "3.6"` dependency
- [x] Create `src/commands/serve/` module
- [x] Implement `/health` endpoint
- [x] Add `Serve` command to CLI

**Phase 2: Model Caching + Embed API** (Future - no use case yet)
- [ ] ServerState with parking_lot::RwLock
- [ ] `/api/embed` and `/api/embed/batch` endpoints
- [ ] Thread-safe embedder access
- *Why deferred: /api/scry handles embedding internally, no external tool needs raw embeddings yet*

**Phase 3: Scry API + Client Detection** ✅ (2025-12-08)
- [x] `/api/scry` endpoint (semantic/lexical/file)
- [x] Mothership client module
- [x] Auto-detection: `PATINA_MOTHERSHIP` env var or localhost check
- [x] Update scry command to route to daemon
- [x] Persona integration (`include_persona` option)

**Phase 4: Container Integration** ✅
- [x] `--host 0.0.0.0` option for container access
- [ ] Update YOLO devcontainer with `PATINA_MOTHERSHIP` env var
- [ ] Test container → Mac queries
- *Partial: flag exists, container testing not validated*

**Phase 5: Repo + Model APIs** (Future - CLI works for now)
- [ ] `/api/repos` endpoints
- [ ] `/api/model` status endpoint
- [ ] Graceful shutdown (SIGTERM)
- *Why deferred: CLI works for repo management, no monitoring dashboard yet*

**API Endpoints:**
```
GET  /health              # Health check                    ✅ Done
GET  /version             # Version info                    ✅ Done
POST /api/scry            # Query (semantic/lexical/file)   ✅ Done
POST /api/embed           # Generate embedding              Future
POST /api/embed/batch     # Batch embeddings                Future
GET  /api/repos           # List repos                      Future
GET  /api/repos/{name}    # Repo details                    Future
GET  /api/model           # Model status                    Future
```

**Files Created:**
```
src/commands/serve/
├── mod.rs              # Public interface         ✅
└── internal.rs         # Server implementation    ✅

src/mothership/
├── mod.rs              # Client interface         ✅
└── internal.rs         # HTTP client for daemon   ✅
```

---

## Completed Phases

### Phase 2.5: Validate Multi-Dimension RAG ✅ (2025-11-25)

**Results:**
| Dimension | Query Type | P@10 | vs Random | Status |
|-----------|------------|------|-----------|--------|
| Semantic | text → text | 7.8% | **8.6x** | ✅ Works |
| Temporal | file → files | 24.4% | **3.2x** | ✅ Works |

**Key Insight:** Each dimension needs its own query interface. Text queries work for semantic, file queries work for temporal.

### Phase 1: Scrape Pipeline ✅ (2025-11-22)
**Specs:** [spec-eventlog-architecture.md](../surface/build/spec-eventlog-architecture.md), [spec-scrape-pipeline.md](../surface/build/spec-scrape-pipeline.md)

Unified eventlog with 16,027 events across 17 types:
- Git: 707 commits → commits, commit_files, co_changes views
- Sessions: 2,174 events → sessions, observations, goals views
- Code: 13,146 events → functions, call_graph, symbols views

### Phase 2: Oxidize (Semantic Only) ✅ (2025-11-24)
**Spec:** [spec-oxidize.md](../surface/build/spec-oxidize.md)

Working pipeline for single dimension:
- Recipe format: `oxidize.yaml`
- E5-base-v2 embeddings (768-dim)
- 2-layer MLP projection (768→1024→256)
- Safetensors export (v0.7, MLX-compatible)
- USearch HNSW index (1,807 vectors)

**Output:**
- `.patina/data/embeddings/e5-base-v2/projections/semantic.safetensors` (4.2MB)
- `.patina/data/embeddings/e5-base-v2/projections/semantic.usearch` (2.1MB)

---

## Phase 4: Solid Foundation (Current)

**Goal:** Cross-project knowledge. Projects have full RAG, reference repos have lightweight indices.

### 4a: `patina rebuild` ✅
**Spec:** [spec-rebuild-command.md](../surface/build/spec-rebuild-command.md)
**Status:** Complete (2025-12-06)

Regenerate `.patina/` from `layer/` for patina projects.

```bash
git clone <patina-project>
patina rebuild
patina scry "test"  # Full semantic search
```

**Validation:** Clone any patina project → rebuild → semantic scry works.

### 4b: Reference Repo Indexing ✅
**Spec:** [spec-repo-command.md](../surface/build/spec-repo-command.md)
**Status:** Complete (2025-12-08)

Reference repos get lightweight indexing: code AST, call graph, FTS5, dependency dimension.

```bash
patina repo add dojoengine/dojo       # Clone, scrape, index
patina repo update --oxidize dojo     # Add dependency dimension
patina scry "spawn" --repo dojo       # Query via dependency index
```

**What reference repos get:**
- Code AST and symbols (FTS5 lexical search)
- Call graph (dependency dimension only)
- Shallow clone (no temporal dimension)
- No sessions (no semantic dimension)

**Key design:** Recipe creates dependency-only projection (no semantic/temporal since data not available).

**Validation:** ✅ `patina repo update --oxidize` creates queryable index, scry auto-detects available dimensions.

### 4c: `--all-repos` Query ✅
**Status:** Complete (2025-12-08)

Single query searches projects (full RAG) + reference repos (lightweight).

```bash
patina scry "entity component patterns" --all-repos
# Projects: semantic + temporal + dependency
# Reference: FTS5 + dependency
```

**Key design:** Results tagged with source (`[PROJECT]`, `[DOJO]`, etc.), sorted by score.

**Validation:** ✅ Query returns combined results from project + all reference repos.

### 4d: Persona ✅
**Spec:** [spec-persona-capture.md](../surface/build/spec-persona-capture.md)
**Status:** Complete (2025-12-08)

**What persona IS:** A learned model of the user that enables LLMs to respond as the user would want. Includes beliefs, knowledge, style, history, preferences.

**Storage:** `~/.patina/personas/default/` (designed for multi-persona future)
```
~/.patina/personas/default/
├── events/           # Append-only capture (tagged by source)
├── materialized/     # persona.db + persona.usearch
└── config.yaml
```

**Capture paths (all continuous, all feed same store):**
1. Reflection flow (`/persona-start`) - dedicated Q&A distillation
2. Session observation - patterns from `/session-*` work
3. Session distillation - scrape sessions → persona knowledge
4. Direct capture - `patina persona note "..."`

**Query:**
```bash
patina persona query "error handling"
patina scry "error handling"  # includes [PERSONA] results
```

**LLM integration:** Adapters (CLAUDE.md, etc.) tell LLM about persona tools. LLM queries persona for context.

**Build sequence:** Storage structure → materialize command → query command → capture paths → scry integration → adapter rules.

**Validation:** Query in project B returns knowledge captured in project A.

### 4e: `patina serve` Complete ✅
**Spec:** [spec-serve-command.md](../surface/build/spec-serve-command.md)
**Status:** Complete (2025-12-08)

HTTP daemon with `/api/scry` for container and remote queries.

```bash
# Mac - start daemon
patina serve

# Query via API directly
curl -X POST localhost:50051/api/scry \
  -d '{"query": "session", "limit": 3}'

# Container or remote - auto-routes to mothership
PATINA_MOTHERSHIP=host.docker.internal:50051 patina scry "test"
```

**What's implemented:**
- Mothership client module (`src/mothership/`)
- `PATINA_MOTHERSHIP` env var detection
- Automatic scry routing to daemon when configured
- All query modes supported (`--all-repos`, `--dimension`, `--repo`)

**Validation:** ✅ Scry auto-routes to daemon when `PATINA_MOTHERSHIP` is set.

---

## Phase 5: Launcher & Adapters (Next)

**Goal:** `patina` becomes the launcher for AI-assisted development. Like `code .` for VS Code.
**Spec:** [spec-launcher-architecture.md](../surface/build/spec-launcher-architecture.md)

**Key Insight:** Source vs Presentation. `.patina/context.md` is committed (source of truth), `CLAUDE.md` is generated and gitignored (presentation).

```bash
patina              # Open project in default frontend
patina claude       # Open in Claude Code
patina gemini       # Open in Gemini CLI
patina --yolo gemini  # Launch in YOLO container
```

### 5a: First-Run Setup
- [ ] Detect first run → create `~/.patina/`
- [ ] Create workspace folder `~/Projects/Patina`
- [ ] Install adapter templates to `~/.patina/adapters/`
- [ ] Detect installed LLM CLIs (enum-based, not manifest files)
- [ ] Set default frontend

### 5b: Launcher Command
- [ ] `patina [path] [frontend]` as **default behavior** (not a subcommand)
- [ ] Parse: frontend names vs subcommands (serve, init, adapter, etc.)
- [ ] Auto-start mothership if not running
- [ ] Prompt `patina init` if not a patina project
- [ ] Launch frontend CLI via `exec`

### 5c: Source/Presentation Model
- [ ] `.patina/context.md` as source of truth (committed)
- [ ] Generate `CLAUDE.md`/`GEMINI.md` on launch (gitignored)
- [ ] Copy `.claude/`/`.gemini/` templates on launch (gitignored)
- [ ] Combine global persona + project context in generated files
- [ ] Update `.gitignore` for presentation files

### 5d: Branch Model
- [ ] Always work on `patina` branch
- [ ] `.patina/config.toml` with `mode = "owner"` or `"contrib"`
- [ ] Document CI stripping for contrib repos
- [ ] PR workflow: patina → main

### 5e: Adapter Commands
- [ ] `patina adapter list` - show available frontends (detected status)
- [ ] `patina adapter default X` - set default frontend
- [ ] Frontend detection via enum (simple, type-safe)

**Validation:** `patina claude` opens Claude Code with full patina integration in < 3 seconds.

---

## Phase 6: MCP Integration

**Goal:** Mothership provides MCP interface. Any frontend that connects gets full patina functionality.
**Spec:** [spec-launcher-architecture.md](../surface/build/spec-launcher-architecture.md)

**Key Insight:** `patina serve` = HTTP + MCP in one process. MCP wraps CLI for LLM frontends.

### 6a: MCP Server in Mothership
- [ ] Add MCP protocol support to `patina serve`
- [ ] stdio interface for frontend connections
- [ ] Shared state with HTTP server

### 6b: Core MCP Tools
- [ ] `patina_context` - get project context and rules
- [ ] `patina_scry` - search codebase knowledge
- [ ] `patina_session_start` - begin tracked session
- [ ] `patina_session_end` - end session, capture learnings
- [ ] `patina_session_note` - capture insight during session

### 6c: Workspace MCP Tools
- [ ] `patina_workspace_list` - list projects
- [ ] `patina_workspace_status` - overview of all projects
- [ ] Cross-project queries via MCP

### 6d: `patina context` CLI Command
- [ ] Design informed by MCP tool implementation
- [ ] `--format=markdown` (default), `--format=json`, `--format=xml`
- [ ] Combines: project rules + scry results + persona

### 6e: HTTP API Extensions
- [ ] `POST /api/context` endpoint
- [ ] `POST /api/session/start`, `/end`, `/note`
- [ ] Parity with MCP tools

**Validation:** MCP tools work from Claude Code, Gemini CLI, and Codex with identical functionality.

---

## Phase 7: Capture Automation

**Goal:** Persona grows automatically without manual `patina persona note`.

### 7a: Session Observation
- [ ] Extract patterns during `/session-*` workflow
- [ ] Tag observations for later distillation
- [ ] Low friction - doesn't interrupt flow

### 7b: Session Distillation
- [ ] `patina persona distill` - batch extract from session history
- [ ] Identify persona-worthy knowledge from sessions
- [ ] Can run nightly or on-demand

### 7c: Reflection Flow (`/persona-start`)
- [ ] Dedicated Q&A session for persona refinement
- [ ] LLM reviews observations, asks strategic questions
- [ ] High signal, intentional capture

**Validation:** After 10 sessions, persona has meaningful knowledge without manual notes.

---

## Phase 8: Model Worlds (Future)
**Spec:** [spec-model-runtime.md](../surface/build/spec-model-runtime.md)
**Blocked until:** Orchestration proven useful

**Design Direction:** Different models for different purposes
- E5-base-v2: Portable router/semantic (ONNX, runs everywhere)
- Code-specific models: Dependency/syntactic dimensions
- MLX: Mac-only acceleration for larger models

**Why deferred:**
- E5-base-v2 validated (+68% vs baseline)
- Model swap invalidates all trained projections
- Prove orchestration value first

### Dimensions Roadmap

| Dimension | Training Signal | Data Available | Status |
|-----------|-----------------|----------------|--------|
| Semantic | Same session = related | 2,174 session events | ✅ Done |
| Temporal | Same commit = related | 590 files, 17,685 co-changes | ✅ Done |
| Dependency | Caller/callee = related | 9,634 code.call events | ✅ Done |
| Syntactic | Similar AST = related | 790 code.function events | Future |
| Architectural | Same module = related | 13,146 code.* events | Future |
| Social | Same author = related | 707 commits | Skip (single-user) |

---

## Architecture Summary

```
Event Sources          →  scrape  →  Unified DB    →  oxidize  →  Vectors    →  scry
.git/ (commits)                      patina.db                    *.usearch       ↓
layer/sessions/*.md                  ├── eventlog                               Results
src/**/* (AST)                       └── views
```

**What's Git-Tracked:**
- `layer/sessions/*.md` - session events (decisions, observations)
- `.patina/oxidize.yaml` - recipe for building projections

**What's Local (rebuilt via scrape/oxidize):**
- `.patina/data/patina.db` - unified eventlog
- `.patina/data/embeddings/` - projection weights + indices

**6-Dimension Model:**
```
Query → E5-base-v2 (768-dim) → [Semantic MLP] → 256-dim ─┐
                              → [Temporal MLP] → 256-dim ─┼→ Concatenated → USearch
                              → [Dependency MLP] → 256-dim─┘   (768-dim with 3)
```

---

## Developer Experience (Future)

**Examples** (`examples/`):
- Currently: 2 demo files (semantic_search_demo.rs, test_e5_model.rs)
- Explore adding more examples for common workflows
- Cargo auto-discovers `examples/*.rs` → `cargo run --example <name>`

**Benchmarks** (`benches/`):
- Currently: None
- Add benchmarks for performance-critical paths (embedding, search, scrape)
- Track regressions over time
- Cargo auto-discovers `benches/*.rs` → `cargo bench`

**Integration Tests** (`tests/`):
- Currently: 11 integration test files, good coverage
- Consider property-based testing for embeddings

---

## Key Sessions (Context Recovery)

When context is lost, read these sessions for architectural decisions:

| Session | Topic | Key Insight |
|---------|-------|-------------|
| 20251208-083033 | Persona Deep Dive | Persona = learned model of user, multi-capture paths, design for multi-persona |
| 20251206-101304 | Continue | Phase 4a complete, bounty removed from core, pre-push script fix |
| 20251205-062457 | Planning Pitfalls | Vendor workflow friction, layer/ must be source of truth |
| 20251204-173633 | What is Patina | "LLM-agnostic agentic RAG network", bounty = drift |
| 20251128-140600 | GitHub Design | Unified semantic space for code + issues. 4 design docs created. |
| 20251125-130143 | Phase 2 Review | Hackathon 10x focus, E5 router, model worlds design |
| 20251125-095019 | Build Continue | Temporal + Scry + Eval complete. Query interface per dimension. |
| 20251125-065729 | RAG design review | "Don't optimize what you can't measure" |
| 20251119-061119 | Patina Cohesion | Mothership as librarian, three-tier projects, cross-project queries |
| 20251118-155141 | Review & Alignment | Hackathon MVP pivot, YOLO verified, Mac+container architecture |
| 20251124-220659 | Direction deep dive | Path C: 2-3 dims → Scry → validate |
| 20251120-110914 | Progressive adapters | Adapter pattern at every layer |

---

## Validation Criteria

### Phase 3 ✅ (2025-12-03)
- [x] Scry: semantic, FTS5, file-based, dimension selection
- [x] Repo: external repos, registry, --repo flag
- [x] GitHub adapter: issues MVP, FTS5 search

### Phase 4 ✅ (2025-12-08)
| Validation | Status |
|------------|--------|
| `git clone <project> && patina rebuild && patina scry` works | [x] |
| `patina repo add` + `--oxidize` creates dependency index | [x] |
| `patina scry --all-repos` returns combined results | [x] |
| `patina persona query` returns cross-project knowledge | [x] |
| `PATINA_MOTHERSHIP=... patina scry` routes to daemon | [x] |

### Phase 5 (Next)
| Validation | Status |
|------------|--------|
| First-run creates `~/.patina/` and workspace | [ ] |
| `patina` (no args) opens default frontend | [ ] |
| `patina claude` opens Claude Code | [ ] |
| `patina gemini` opens Gemini CLI | [ ] |
| `.patina/context.md` generates `CLAUDE.md` on launch | [ ] |
| Presentation files (`CLAUDE.md`, `.claude/`) are gitignored | [ ] |
| Switching frontends < 2 seconds (regenerate from same source) | [ ] |
| Owner mode: patina artifacts included in PR | [ ] |
| Contrib mode: CI strips patina artifacts | [ ] |

### Phase 6
| Validation | Status |
|------------|--------|
| `patina serve` includes MCP server | [ ] |
| MCP tools work from Claude Code | [ ] |
| MCP tools work from Gemini CLI | [ ] |
| `patina context "X"` returns LLM-formatted output | [ ] |
| Same project works with all frontends via MCP | [ ] |

### Phase 7
| Validation | Status |
|------------|--------|
| After 10 sessions, persona has knowledge without manual notes | [ ] |
