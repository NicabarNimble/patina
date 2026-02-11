---
type: feat
id: mother-v2
status: abandoned
created: 2026-02-05
updated: 2026-02-05
sessions:
  origin: 20260205-064001
related:
- layer/surface/build/feat/mother/SPEC.md
- layer/surface/build/feat/surface-layer/SPEC.md
- layer/surface/build/explore/beads-patterns/SPEC.md
- layer/surface/build/refactor/semantic-structural-split/SPEC.md
- layer/surface/epistemic/beliefs/mother-is-the-daemon.md
- layer/surface/epistemic/beliefs/patina-is-knowledge-layer.md
beliefs:
- mother-is-the-daemon
- patina-is-knowledge-layer
- mother-owns-ref-repo-indexing
- corpus-composition-over-model
---

# feat: Mother v2 — The Nervous System

> Mother is the always-running daemon that knows where all knowledge lives, owns the environment, and orchestrates belief networks across projects and users.

## Problem

Current architecture has three disconnected knowledge layers:

1. **Project beliefs** — `layer/surface/epistemic/beliefs/` — 59 beliefs, project-local, can't share
2. **Persona notes** — `~/.patina/personas/default/` — 4 entries, disconnected from beliefs
3. **Ref repo knowledge** — 18 repos indexed, code searchable, but no belief extraction

Mother v1 routes queries via graph but doesn't own the belief network. Projects are self-contained islands. User-level knowledge (persona) doesn't flow into project work. Ref repos contribute code examples but not distilled wisdom.

**The gap:** No place for beliefs that span projects. No graduation path for beliefs that prove themselves. No mechanism to extract beliefs from reference repositories.

---

## Vision

Mother is the nervous system connecting all knowledge:

```
┌─────────────────────────────────────────────────────────────────┐
│                         MOTHER                                   │
│                                                                  │
│   ~/.patina/                                                     │
│   ├── layer/                    # User-level knowledge           │
│   │   ├── core/                                                  │
│   │   │   ├── values/           # Deeply held (high validation)  │
│   │   │   └── rules/            # Beliefs → Actions              │
│   │   ├── surface/                                               │
│   │   │   └── beliefs/          # Active user beliefs            │
│   │   └── dust/                                                  │
│   │       └── archived/         # Historical                     │
│   │                                                              │
│   ├── mother/                                                    │
│   │   ├── graph.db              # Relationship network           │
│   │   ├── beliefs.db            # Cross-project belief index     │
│   │   ├── environment.toml      # Capabilities, models, state    │
│   │   └── cache/                # Hot model cache                │
│   │       └── models/           # ONNX models (moved from proj)  │
│   │                                                              │
│   └── registry.yaml             # All known repos                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
    [project-a/]         [project-b/]        [ref-repos/]
    layer/surface/       layer/surface/      (read-only)
    epistemic/beliefs/   epistemic/beliefs/
```

**Key shifts:**

| Aspect | v1 (Current) | v2 (Proposed) |
|--------|--------------|---------------|
| Belief storage | Project-only | Project + User + Ref |
| Model ownership | Per-project `.patina/` | Mother `~/.patina/mother/cache/` |
| Environment awareness | Project config | Mother knows all |
| Persona integration | Disconnected notes | Flows to user beliefs |
| Cross-project search | Code only | Code + Beliefs |

---

## Core Concepts

### 1. Three-Tier Belief Network

```
┌─────────────────────────────────────────────────────────────┐
│                     USER LAYER                               │
│           ~/.patina/layer/surface/beliefs/                   │
│                                                              │
│   Beliefs that apply across all your projects.               │
│   Source: persona notes, session learnings, manual capture   │
│   Example: "prefer explicit error handling"                  │
└─────────────────────────────────────────────────────────────┘
                              │
              flows down (inheritance)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   PROJECT LAYER                              │
│           <project>/layer/surface/epistemic/beliefs/         │
│                                                              │
│   Beliefs specific to this project.                          │
│   Source: session work, spec decisions, code patterns        │
│   Example: "use thiserror for error types"                   │
└─────────────────────────────────────────────────────────────┘
                              │
              queries (reference)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  REFERENCE LAYER                             │
│           ~/.patina/repos/<repo>/extracted-beliefs/          │
│           (or computed on-demand, not stored)                │
│                                                              │
│   Beliefs extracted from reference repositories.             │
│   Source: patterns observed in code, commit history          │
│   Example: "dojo uses felt252 for error codes"               │
└─────────────────────────────────────────────────────────────┘
```

**Query resolution:**
1. Search project beliefs first (most specific)
2. Merge user beliefs (inherited context)
3. Optionally include ref repo beliefs (via graph edges)

### 2. Belief Evolution: Beliefs → Values → Rules

```
BELIEF (surface)
    │
    │  Survives verification across N sessions
    │  Applied in M+ projects
    │  Zero contested verifications
    │
    ▼
VALUE (core)
    │
    │  Combined with trigger conditions
    │  Produces actionable output
    │
    ▼
RULE (core, experimental)
```

**Belief** (current):
```yaml
# ~/.patina/layer/surface/beliefs/explicit-errors.md
---
type: belief
id: explicit-errors
entrenchment: medium
---
Prefer explicit error handling over panic.
```

**Value** (proposed):
```yaml
# ~/.patina/layer/core/values/explicit-errors.md
---
type: value
id: explicit-errors
promoted_from: belief
promotion_evidence:
  - verified_in: [patina, cairo-game, my-cli]
  - sessions_applied: 47
  - contested: 0
entrenchment: very-high
---
Prefer explicit error handling over panic.

## Promotion Log
- 2026-02-05: Promoted from belief after 47 applications across 3 projects
```

**Rule** (experimental, beads-influenced):
```yaml
# ~/.patina/layer/core/rules/error-handling.md
---
type: rule
id: error-handling-rule
derives_from:
  - value: explicit-errors
  - belief: use-thiserror
trigger:
  - new_rust_file
  - error_type_defined
action:
  suggest: "Consider using thiserror for this error type"
---
When creating error types in Rust, use thiserror for derive macros.
```

### 3. Mother as Environment Owner

**Current:** Each project has `.patina/local/data/embeddings/<model>/` with models.

**Proposed:** Mother owns models centrally:

```
~/.patina/mother/
├── environment.toml          # What's installed, what's available
├── cache/
│   └── models/
│       ├── e5-base-v2/       # Shared across all projects
│       └── bge-small/
└── capabilities.json         # Computed: what can this machine do?
```

**environment.toml:**
```toml
[models]
default = "e5-base-v2"
available = ["e5-base-v2", "bge-small"]

[runtime]
onnx_path = "~/.patina/mother/lib/libonnxruntime.dylib"
wasm_grammars = "~/.patina/mother/lib/grammars/"

[features]
embeddings = true
tree_sitter = true
gpu_acceleration = false
```

**Benefits:**
- Projects don't duplicate 90MB model files
- `patina doctor` checks Mother environment, not per-project
- Model upgrades happen once, all projects benefit
- Mother daemon has hot cache of loaded models

### 4. Ref Repo Belief Extraction

Reference repositories (claude-code, beads, opencode, etc.) contain implicit beliefs in their code and commits. Extract and surface them.

**Extraction sources:**
1. **README/docs** — Stated principles
2. **Commit patterns** — "always run tests before merge" (observed)
3. **Code patterns** — "uses Result<T,E> everywhere" (detected)
4. **Issue discussions** — Design decisions

**Storage options:**

| Option | Pros | Cons |
|--------|------|------|
| A. Store extracted beliefs | Fast query, versioned | Storage overhead, staleness |
| B. Compute on-demand | Always fresh | Slow, repeated work |
| C. Cache with TTL | Balance | Complexity |

**Recommendation:** Option A with manual refresh. `patina repo update <repo>` re-extracts beliefs.

```
~/.patina/repos/
├── steveyegge-beads/
│   ├── patina.db              # Existing: code index
│   └── extracted/
│       └── beliefs/           # NEW: extracted beliefs
│           ├── content-hash-ids.md
│           └── three-layer-sync.md
```

### 5. Beads Integration Point

From `beads-patterns` exploration spec:

| Tool | Role |
|------|------|
| **Beads** | Task orchestration — "what to do next" |
| **Patina Mother** | Knowledge orchestration — "how we do things" |

**Integration design:**

```
┌─────────────────┐         ┌─────────────────┐
│     BEADS       │         │  PATINA MOTHER  │
│                 │         │                 │
│  Issues/Tasks   │◄───────►│  Beliefs/Values │
│  Ready Queue    │         │  Rules          │
│  Blockers       │         │  Patterns       │
│                 │         │                 │
│  "Do this next" │         │  "Do it this    │
│                 │         │   way"          │
└─────────────────┘         └─────────────────┘
         │                           │
         └───────────┬───────────────┘
                     ▼
              Agent Workflow
              1. bd ready → get task
              2. patina context → get beliefs
              3. work
              4. bd close → complete task
```

**Not in scope for v2:** Full beads integration. But design Mother to not conflict with beads' domain.

---

## Implementation Phases

### Phase 1: User Layer Structure

Create `~/.patina/layer/` mirroring project layer structure.

**Tasks:**
- [ ] Create `~/.patina/layer/surface/beliefs/` directory on first run
- [ ] Create `~/.patina/layer/core/values/` and `~/.patina/layer/core/rules/` (empty, future)
- [ ] Migrate existing persona notes to user beliefs format
- [ ] `patina persona note` writes to user beliefs (not separate persona store)

**Exit:** `ls ~/.patina/layer/surface/beliefs/` shows user beliefs.

### Phase 2: Cross-Project Belief Index

Mother tracks all beliefs across all sources.

**Tasks:**
- [ ] Create `~/.patina/mother/beliefs.db` schema
- [ ] Index user beliefs on `patina scrape` or `patina mother sync`
- [ ] Index project beliefs when Mother daemon sees project
- [ ] `patina scry --all-repos --content-type beliefs` queries the index

**Schema:**
```sql
CREATE TABLE belief_index (
    id TEXT PRIMARY KEY,           -- belief ID
    source_type TEXT,              -- 'user', 'project', 'ref'
    source_path TEXT,              -- file path or repo name
    title TEXT,
    statement TEXT,
    entrenchment TEXT,
    embedding BLOB,                -- for semantic search
    last_indexed TIMESTAMP
);

CREATE VIRTUAL TABLE belief_index_fts USING fts5(
    id, title, statement,
    content='belief_index'
);
```

**Exit:** `patina scry "error handling" --content-type beliefs --all-repos` returns beliefs from user, project, and ref repos.

### Phase 3: Environment Ownership

Move model management from project to Mother.

**Tasks:**
- [ ] Create `~/.patina/mother/environment.toml` schema
- [ ] Move model storage to `~/.patina/mother/cache/models/`
- [ ] Update `patina oxidize` to use Mother model path
- [ ] Update `patina model` commands to manage Mother cache
- [ ] Project `.patina/config.toml` references model by name, not path
- [ ] `patina doctor` checks Mother environment

**Migration:**
```bash
# Old: per-project
.patina/local/data/embeddings/e5-base-v2/model.onnx

# New: Mother-owned
~/.patina/mother/cache/models/e5-base-v2/model.onnx

# Project config just names it
[embeddings]
model = "e5-base-v2"  # Mother resolves path
```

**Exit:** Delete project model files, `patina oxidize` still works via Mother.

**Discovery from [[semantic-structural-split]] (2026-02-08):**
`oxidize_for_repo()` in `src/commands/oxidize/mod.rs` currently has the *project* creating recipes inside ref repos — the project reaches past mother to configure shared infra. This violates the ownership boundary. When mother owns environment and models, she should also own ref repo indexing: when to oxidize, which recipe, where to store results. The current function is a stopgap to migrate, not a pattern to preserve. See [[mother-owns-ref-repo-indexing]].

### Phase 4: Belief Evolution (Values)

Implement promotion from belief to value.

**Tasks:**
- [ ] Define promotion criteria (N sessions, M projects, 0 contested)
- [ ] `patina belief audit` shows promotion candidates
- [ ] `patina belief promote <id>` moves to values with evidence
- [ ] Values loaded with higher weight in context queries
- [ ] Values visible in `patina context` output

**Exit:** `ls ~/.patina/layer/core/values/` shows promoted beliefs.

### Phase 5: Rules (Experimental)

Combine beliefs into actionable rules.

**Tasks:**
- [ ] Define rule schema (derives_from, trigger, action)
- [ ] `patina rule create` command
- [ ] Rules surface in context when trigger matches
- [ ] Evaluate: do rules change agent behavior?

**Exit:** Rules exist and surface contextually. Measure impact.

### Phase 6: Ref Repo Belief Extraction

Extract beliefs from reference repositories.

**Tasks:**
- [ ] Define extraction heuristics (README patterns, commit patterns)
- [ ] `patina repo extract-beliefs <repo>` command
- [ ] Extracted beliefs stored in `~/.patina/repos/<repo>/extracted/beliefs/`
- [ ] Extracted beliefs indexed in `beliefs.db`
- [ ] `patina repo update` re-extracts

**Exit:** `patina scry "error handling" --repo beads --content-type beliefs` returns extracted beliefs.

**Discovery from [[semantic-structural-split]] (2026-02-08):**
The knowledge corpus (`query_knowledge_corpus()`) already works for ref repos — missing tables (patterns, beliefs) are handled gracefully, producing a commits-only index. This is a working baseline. Ref repos with commits-only knowledge indexes are correct and sufficient per [[corpus-composition-over-model]]. Phase 6 should build on this (adding extracted beliefs to the corpus) rather than designing a separate extraction pipeline from scratch. The indexing infrastructure exists; what's missing is mother owning when and how it runs. See [[mother-owns-ref-repo-indexing]].

---

## Non-Goals

- **Replace beads** — Beads owns task orchestration, Patina owns knowledge
- **Automatic belief creation** — User/LLM captures beliefs, system indexes them
- **Real-time sync** — Scrape/index is batch, not live
- **Windows support** — Mac-first, Linux for containers

---

## Open Questions

1. **User belief inheritance:** Should project beliefs override or merge with user beliefs?
2. **Ref repo belief authority:** Are extracted beliefs suggestions or facts?
3. **Value promotion automation:** Should promotion be automatic or require user approval?
4. **Rule trigger language:** How to express triggers? Regex? AST patterns? Natural language?
5. **Beads as dependency:** Should Patina depend on beads, or just interoperate?

---

## Dependencies

- **mother-delivery (v0.11.0)** — Must complete first (D0-D5, A/B eval)
- **v1-release (v0.12.0)** — Dynamic ONNX loading aligns with Phase 3
- **beads-patterns** — Informs Phase 5 (rules) design

---

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| User beliefs | 0 | 10+ |
| Cross-project belief queries | Not possible | Works |
| Model storage duplication | N copies | 1 copy |
| Values promoted | 0 | 5+ |
| Ref repo beliefs extracted | 0 | 50+ |

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-05 | design | Created from state-of-union session 20260205-064001 |
| 2026-02-08 | design | Added [[mother-owns-ref-repo-indexing]] + [[corpus-composition-over-model]] discoveries from [[semantic-structural-split]] Phase 2 cleanup |
