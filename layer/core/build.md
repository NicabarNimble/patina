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
- `persona/` = beliefs that flow UP from projects
- `patina serve` = daemon for cross-project queries

See: [rag-network.md](../surface/rag-network.md)

---

## Current Direction (2025-12-07)

**Goal:** Cross-project knowledge that helps win hackathons.

**Phase 3 (Query Infrastructure):** ✅ Complete
- Scrape, oxidize, scry, repo, dependency dimension all working
- 8.6x improvement over random (measured)

**Phase 4 (Solid Foundation):** ← Current focus

| Phase | Deliverable | Validation |
|-------|-------------|------------|
| **4a** | `patina rebuild` | `git clone <project> && patina rebuild && patina scry` works |
| **4b** | Reference repo indexing | `patina repo add` creates dependency index, `--repo` queries work |
| **4c** | `--all-repos` query | Single query searches projects + reference repos |
| **4d** | Persona capture + query | Beliefs flow up from projects, inform queries |
| **4e** | `patina serve` complete | Containers query Mac mothership |

**Specs:**
- [spec-rebuild-command.md](../surface/build/spec-rebuild-command.md) - for projects
- [spec-repo-command.md](../surface/build/spec-repo-command.md) - for reference repos
- [spec-serve-command.md](../surface/build/spec-serve-command.md)
- [spec-persona-capture.md](../surface/build/spec-persona-capture.md)

**Deferred (use-case features, not core):**
- Bounty detection (future plugin system)
- GitHub semantic embeddings
- MLX runtime, model upgrades

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
**Status:** In Progress (2025-12-03)
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

**Phase 2: Model Caching + Embed API**
- [ ] ServerState with parking_lot::RwLock
- [ ] `/api/embed` and `/api/embed/batch` endpoints
- [ ] Thread-safe embedder access

**Phase 3: Scry API + Client Detection**
- [ ] `/api/scry` endpoint (semantic/lexical/file)
- [ ] Mothership client module
- [ ] Auto-detection: `PATINA_MOTHERSHIP` env var or localhost check
- [ ] Update scry command to route to daemon

**Phase 4: Container Integration**
- [ ] `--host 0.0.0.0` option for container access
- [ ] Update YOLO devcontainer with `PATINA_MOTHERSHIP` env var
- [ ] Test container → Mac queries

**Phase 5: Repo + Model APIs**
- [ ] `/api/repos` endpoints
- [ ] `/api/model` status endpoint
- [ ] Graceful shutdown (SIGTERM)

**API Endpoints:**
```
GET  /health              # Health check
POST /api/scry            # Query (semantic/lexical/file)
POST /api/embed           # Generate embedding
POST /api/embed/batch     # Batch embeddings
GET  /api/repos           # List repos
GET  /api/repos/{name}    # Repo details
GET  /api/model           # Model status
```

**Files to Create:**
```
src/commands/serve/
├── mod.rs              # Public interface
└── internal.rs         # Server implementation

src/mothership/
├── mod.rs              # Client interface
└── internal.rs         # HTTP client for daemon
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

### 4b: Reference Repo Indexing
**Spec:** [spec-repo-command.md](../surface/build/spec-repo-command.md)
**Status:** In Progress

Reference repos get lightweight indexing: code AST, call graph, FTS5, dependency dimension.

```bash
patina repo add dojoengine/dojo       # Clone, scrape, index
patina repo update --oxidize dojo     # Add dependency dimension
patina scry "spawn" --repo dojo       # Query via FTS5 or dependency
```

**What reference repos get:**
- Code AST and symbols (FTS5 lexical search)
- Call graph (dependency dimension)
- Shallow clone (no temporal dimension)
- No sessions (no semantic dimension)

**Validation:** `patina repo add` + `--oxidize` creates queryable index.

### 4c: `--all-repos` Query
**Status:** Not Started

Single query searches projects (full RAG) + reference repos (lightweight).

```bash
patina scry "entity component patterns" --all-repos
# Projects: semantic + temporal + dependency
# Reference: FTS5 + dependency
```

**Validation:** Query returns results from both projects and reference repos.

### 4d: Persona Capture + Query
**Spec:** [spec-persona-capture.md](../surface/build/spec-persona-capture.md)
**Status:** Not Started

Beliefs flow UP from projects to persona, inform future queries.

```bash
# In dojo project
/session-note "ECS systems should be stateless"

# Later, in different project
patina scry "system design"
# [PERSONA] ECS systems should be stateless (learned from dojo)
```

**Validation:** Note captured in one project → appears as `[PERSONA]` result in another.

### 4e: `patina serve` Complete
**Spec:** [spec-serve-command.md](../surface/build/spec-serve-command.md)
**Status:** Phase 1 done (/health endpoint)

Full daemon with `/api/scry`, container support, hot model caching.

```bash
# Mac
patina serve

# Container (YOLO dev)
PATINA_MOTHERSHIP=host.docker.internal:50051 patina scry "test"
```

**Validation:** YOLO container can query Mac mothership.

---

## Phase 5: Model Worlds (Future)
**Spec:** [spec-model-runtime.md](../surface/build/spec-model-runtime.md)
**Blocked until:** Phase 4 complete, multi-project coordination proven

**Design Direction:** Different models for different purposes
- E5-base-v2: Portable router/semantic (ONNX, runs everywhere)
- Code-specific models: Dependency/syntactic dimensions
- MLX: Mac-only acceleration for larger models

**Why deferred:**
- E5-base-v2 validated (+68% vs baseline)
- Model swap invalidates all trained projections
- Prove multi-project coordination first

### Dimensions Roadmap

| Dimension | Training Signal | Data Available | Status |
|-----------|-----------------|----------------|--------|
| Semantic | Same session = related | 2,174 session events | ✅ Done |
| Temporal | Same commit = related | 590 files, 17,685 co-changes | ✅ Done |
| Dependency | Caller/callee = related | 9,634 code.call events | Phase 3d |
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

## Key Sessions (Context Recovery)

When context is lost, read these sessions for architectural decisions:

| Session | Topic | Key Insight |
|---------|-------|-------------|
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

**Phase 3 Complete!** ✅ (2025-12-03) - Query infrastructure working.
- [x] Scry: semantic, FTS5, file-based, dimension selection
- [x] Repo: external repos, registry, --repo flag
- [x] GitHub adapter: issues MVP, FTS5 search

**Phase 4 Checklist:**

| Phase | Validation | Status |
|-------|------------|--------|
| 4a | `git clone <project> && patina rebuild && patina scry` works | [x] |
| 4b | `patina repo add` + `--oxidize` creates dependency index for reference repos | [ ] |
| 4c | `patina scry --all-repos` returns results from projects + reference repos | [ ] |
| 4d | `/session-note` in project A → `[PERSONA]` result in project B | [ ] |
| 4e | YOLO container queries Mac via `PATINA_MOTHERSHIP` | [ ] |

**Phase 4 Complete = Ready for hackathons with cross-project knowledge.**
