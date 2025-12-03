# Build Recipe

Persistent roadmap across sessions. **Start here when picking up work.**

---

## Current Direction (2025-11-25)

**Goal:** 10x productivity for OnlyDust contributions and Ethereum/Starknet hackathons.

**Key Insight:** Phase 2.5 complete. Each dimension needs its own query interface:
- **Semantic** (text→text): Working (8.6x over random)
- **Temporal** (file→files): Working (3.2x over random), needs `--file` flag in scry

**Target Workflow:**
```
Mac (Mothership)              YOLO Container (Linux)
├── Persona knowledge         ├── Claude CLI
├── Cross-project indices     ├── Project code
├── Reference repo scrapes    └── Queries Mac via gRPC
└── Model hosting (MLX)
```

**Immediate Path (Phase 3):**
1. File-based scry queries (`patina scry --file src/foo.rs`) ✅
2. FTS5 lexical search for exact matches ✅
3. Mothership service for multi-project coordination ✅
4. Dependency dimension (call graph) ✅
5. GitHub integration (bounty discovery) ✅
6. **(Future) gRPC daemon for container queries**

**Explicitly Deferred:**
- MLX runtime (nice-to-have, E5 ONNX works everywhere)
- Qwen3/model upgrades (invalidates projections, premature)
- Syntactic/architectural dimensions (dependency first)

---

## Active Work

### Phase 3: Hackathon MVP

**Goal:** Enable 10x productivity for OnlyDust bounties and Starknet/Ethereum hackathons.

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
**Spec:** [spec-mothership-service.md](../surface/build/spec-mothership-service.md)
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
**Spec:** [spec-github-integration.md](../surface/build/spec-github-integration.md)
**Architecture:** [github-integration-architecture.md](../surface/build/github-integration-architecture.md)
**Why:** OnlyDust bounty discovery + hackathon context from issues/discussions

**Key Insight:** GitHub issues use SAME semantic space as code (E5 → MLP → 256-dim). Query "entity spawning" returns both code AND related issues.

**Phase 1: Issues MVP (Complete)**
- [x] Create `src/commands/scrape/github/mod.rs`
- [x] Add `github_issues` materialized view schema
- [x] Implement `gh issue list --json` integration
- [x] Bounty detection (labels + body parsing)
- [x] Add `--with-issues` flag to `patina repo add`
- [x] Add `github.issue` events to FTS5 index
- [x] Add `--include-issues` flag to `patina scry`
- [x] Test with dojoengine/dojo (500 issues indexed)
- [x] Graceful fallback to FTS5 when semantic index missing

```bash
patina repo add dojoengine/dojo --with-issues
patina scry "bounty cairo" --repo dojo --include-issues --label bounty
```

**Phase 2: Semantic Search (Future)**
- [ ] Generate E5 embeddings for issue title + body
- [ ] Store in embeddings table (same space as code)
- [ ] Cross-type ranking in scry results

**Phase 3: PRs + Discussions (Future)**
- [ ] Add `github_prs`, `github_discussions` tables
- [ ] `gh pr list` and `gh api graphql` integration
- [ ] Extend scry with `--include-prs`, `--include-discussions`

**Phase 4: Cross-Project Bounty Discovery (Future)**
- [ ] `patina scry "bounty" --all-repos --label bounty`
- [ ] Aggregate bounties from all registered repos
- [ ] Persona-aware bounty matching

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

## Future Phases

### Phase 4: Persona & Cross-Project Learning
**Spec:** [spec-persona-capture.md](../surface/build/spec-persona-capture.md)
**Blocked until:** Mothership working

- Your beliefs persist across projects
- Projects can query persona for patterns
- Non-contradictory adoption from persona to project

### Phase 5: Model Worlds
**Spec:** [spec-model-runtime.md](../surface/build/spec-model-runtime.md)
**Blocked until:** Hackathon MVP proves value

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

**Phase 2.5 Complete!** ✅ (2025-11-25)

**Phase 3 is complete when:**
1. [x] `patina scry --file src/foo.rs` returns co-changing files
2. [x] `patina scry "find X"` uses FTS5 for exact matches
3. [x] `patina repo <url>` adds external repos to `~/.patina/repos/`
4. [x] `patina scry "query" --repo <name>` queries external repos
5. [x] Dependency dimension trained and queryable
6. [x] GitHub issues searchable via `scry --include-issues`
7. [ ] (Future) gRPC daemon for container queries

**GitHub MVP complete when:**
- `patina repo add <url> --with-issues` fetches and indexes issues
- `patina scry "bounty" --include-issues --label bounty` finds bounties
- Bounty detection works (labels + body parsing)
- FTS5 search covers issue title + body

**Hackathon-ready when:** Can query Dojo patterns AND bounties while building Starknet game via `scry --repo dojo --include-issues`.
