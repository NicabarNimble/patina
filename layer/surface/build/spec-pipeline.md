# Spec: Pipeline Architecture

**Purpose:** Define the three-stage pipeline where scry becomes the unified oracle for LLM frontends.

**Origin:** Architecture crystallization from session 20251219-104245. Audit of existing commands revealed missing ORGANIZE stage between scrape and query.

---

## System Context

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           MAC (Mothership)                               │
│  ~/.patina/                                                              │
│    ├── cache/models/     (embedding models)                              │
│    ├── cache/repos/      (reference repos)                               │
│    ├── persona/          (cross-project user knowledge)                  │
│    └── registry.yaml     (what repos exist)                              │
│                                                                          │
│  patina serve            (daemon, MCP server)                            │
└─────────────────────────────────────────────────────────────────────────┘
         │
         │ PATINA_MOTHERSHIP
         ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      YOLO Container / Local Dev                          │
│                                                                          │
│   project/                                                               │
│     ├── .git/            ← SOURCE OF TRUTH                               │
│     ├── layer/           ← PATTERNS (git-tracked)                        │
│     │    ├── core/       (eternal values)                                │
│     │    ├── surface/    (active architecture)                           │
│     │    └── sessions/   (distilled learnings)                           │
│     └── .patina/         ← DERIVED (rebuild from git + layer)            │
│          └── data/                                                       │
│               ├── patina.db        (facts)                               │
│               └── embeddings/      (vectors)                             │
└─────────────────────────────────────────────────────────────────────────┘
         │
         │ MCP / CLI
         ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                     LLM Frontend (Claude Code / Gemini)                  │
│                                                                          │
│   User asks question → scry → fused context → LLM responds               │
│   LLM writes code → git commit → scrape captures → loop                  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Problem

Current flow skips signal derivation:

```
scrape → facts → scry (queries raw facts)
```

LLM asks "is this module maintained?" but scry can only return:
- Semantic matches (similar text)
- Lexical matches (keyword hits)
- Temporal co-changes (files that change together)

Missing: derived signals like health, activity, centrality, staleness.

---

## Solution

Three-stage pipeline with scry as unified oracle:

```
                            GIT (source of truth)
                                    │
                                    ▼
                                 scrape
                        (extract facts from reality)
                                    │
                                    ▼
                               SQLite DB
                              (structured facts)
                                    │
                   ┌────────────────┴────────────────┐
                   ▼                                 ▼
               oxidize                            assay
           (→ embeddings)                      (→ signals)
                   │                                 │
                   │    ┌────────────────────────────┤
                   │    │                            │
                   ▼    ▼                            ▼
                usearch                       signal tables
                indexes                     (health, activity,
                                             centrality, staleness)
                   │                                 │
                   └────────────┬────────────────────┘
                                ▼
                              scry
                        (world model / oracle)
                                │
                                ├── SemanticOracle (oxidize)
                                ├── StructuralOracle (assay) ← NEW
                                ├── LexicalOracle (FTS5)
                                ├── TemporalOracle
                                └── PersonaOracle
                                │
                                ▼
                           RRF Fusion
                                │
                                ▼
                         LLM Frontend
                                │
                                ▼
                         Developer acts
                                │
                                ▼
                            git commit
                                │
                                └──────────→ (back to top)
```

---

## Command Roles

| Command | Stage | "Do X" | Input | Output |
|---------|-------|--------|-------|--------|
| scrape | EXTRACT | Capture raw → structured facts | code, git, sessions | SQLite tables |
| oxidize | PREPARE (semantic) | Build embeddings from facts | SQLite | usearch indexes |
| assay | PREPARE (structural) | Build signals from facts | SQLite | signal tables |
| scry | DELIVER | Fuse and route knowledge to LLM | all prepared data | answers |

---

## Values Alignment

| Value | How This Honors It |
|-------|-------------------|
| **unix-philosophy** | One tool, one job: scrape extracts, oxidize embeds, assay derives, scry delivers |
| **dependable-rust** | Each command is a black box with stable interface |
| **local-first** | Everything runs locally. SQLite + usearch = portable. No cloud. |
| **git as memory** | Git is source of truth. layer/ tracked. .patina/ derived (rebuildable). |
| **escape hatches** | Raw facts in SQLite. Can query directly if scry doesn't serve your need. |

---

## Signals (assay derive)

Structural signals computed from facts:

| Signal | Source | Formula |
|--------|--------|---------|
| `is_used` | import_facts | importer_count > 0 OR is_entry_point |
| `importer_count` | import_facts | COUNT of modules that import this |
| `activity_level` | co_changes + git | commits in last N days |
| `core_contributors` | git history | top authors by commit count |
| `centrality` | call_graph | degree or PageRank |
| `staleness` | cross-reference | contradicts CI, references deleted code |

### Schema Addition

```sql
CREATE TABLE IF NOT EXISTS module_signals (
    path TEXT PRIMARY KEY,
    is_used INTEGER,
    importer_count INTEGER,
    activity_level TEXT,  -- 'high', 'medium', 'low', 'dormant'
    last_commit_days INTEGER,
    top_contributors TEXT,  -- JSON array
    centrality_score REAL,
    staleness_flags TEXT,  -- JSON array of issues
    computed_at TEXT
);
```

---

## Scry as Unified Oracle

Scry becomes the single query interface. It fuses:

```
scry
  ├── SemanticOracle (from oxidize)
  ├── StructuralOracle (from assay signals) ← NEW
  ├── LexicalOracle (FTS5)
  ├── TemporalOracle (co-changes)
  └── PersonaOracle (user preferences)
```

### Example Query Flow

```
LLM: "What should I know about src/retrieval/engine.rs?"

scry fuses:
  SemanticOracle: "Related to query processing, similar to oracle.rs"
  StructuralOracle: "3 importers, 12 functions, high centrality, active"
  TemporalOracle: "Co-changes with fusion.rs, oracle.rs"
  PersonaOracle: "You've worked on this before"

Response: "Core retrieval module, actively maintained, well-connected.
           You've modified it in previous sessions."
```

---

## The Git Loop

```
Session Start
     │
     ▼
patina scrape ◄──────────────────────────────┐
     │                                        │
     ▼                                        │
oxidize + assay derive                        │
     │                                        │
     ▼                                        │
LLM queries scry                              │
     │                                        │
     ▼                                        │
LLM understands:                              │
  "This module is dead (0 importers)"         │
  "This area is active (12 commits/week)"     │
  "Core contributors: alice, bob"             │
     │                                        │
     ▼                                        │
LLM makes better decisions                    │
     │                                        │
     ▼                                        │
Developer commits code                        │
     │                                        │
     ▼                                        │
Session end → distill to layer/sessions/      │
     │                                        │
     └────────────────────────────────────────┘
```

Git is the coordination mechanism. Patina is the intelligence layer that makes git knowledge queryable.

---

## Multi-User Future

```
                    Shared Git Repo
                    (push / pull)
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
      Alice            Bob            Carol
    (her Mac)        (his Mac)       (her Mac)
         │               │               │
    ~/.patina/      ~/.patina/      ~/.patina/
    (her persona)   (his persona)   (her persona)
         │               │               │
    clone/.patina   clone/.patina   clone/.patina
    (local prep)    (local prep)    (local prep)
```

- **Git syncs:** layer/, code, session distillations
- **Patina prepares locally:** Each person runs scrape/oxidize/assay
- **Persona is personal:** ~/.patina/persona/ stays local
- **Rebuild reconstructs:** `patina rebuild` from git + layer/

No central server needed. Git IS the server.

---

## Does It Come Together?

Yes. This architecture:

1. **Clarifies roles:**
   - scrape = extract from reality
   - oxidize = prepare semantic
   - assay = prepare structural
   - scry = unified oracle

2. **Honors values:** One job each (unix-philosophy), black boxes (dependable-rust)

3. **Leverages git:** Signals derived FROM git (activity, contributors, staleness)

4. **Stays local-first:** No cloud. Rebuild from git.

5. **Scales naturally:** Git syncs, patina prepares locally, persona stays personal

6. **Clean MCP interface:** LLM talks to scry. scry talks to everything else.

**The Insight:** scry is the API between LLM and codebase knowledge. Everything else prepares for that moment.

---

## Implementation Plan

### Phase 1: Assay Signals

| Task | Effort |
|------|--------|
| Add module_signals table to schema | ~20 lines |
| Implement `assay derive` subcommand | ~150 lines |
| Compute is_used, importer_count | ~50 lines |
| Compute activity_level from git data | ~80 lines |
| Add StructuralOracle to scry | ~100 lines |
| Wire into RRF fusion | ~30 lines |

**Total:** ~430 lines

### Phase 2: Enhanced Signals

| Task | Effort |
|------|--------|
| Centrality (PageRank on call graph) | ~100 lines |
| Staleness detection (cross-reference) | ~150 lines |
| Core contributors extraction | ~50 lines |

### Phase 3: Scry Intelligence

| Task | Effort |
|------|--------|
| Query routing based on question type | TBD |
| Learned fusion weights | TBD |
| Summary generation | TBD |

---

## Performance

| Step | When | Cost | Incremental? |
|------|------|------|--------------|
| scrape | On change / init | ~500ms | Yes (hash check) |
| oxidize | On scrape / manual | ~10s | Yes (recipe-driven) |
| assay derive | On scrape / manual | ~100ms | Yes (SQL) |
| scry | Query time | ~100ms | N/A |

The heavy work (oxidize) is done once. Signals (assay derive) are cheap SQL. Queries (scry) are fast.

---

## Validation

| Criteria | Metric |
|----------|--------|
| Signals computed correctly | Unit tests on known codebase |
| Scry fusion includes structural | `patina eval` shows structural oracle hits |
| Real-world improvement | `patina eval --feedback` precision increase |
| No regression | MRR stays >= 0.624 |

---

## Exit Criteria

| Criteria | Status |
|----------|--------|
| spec-pipeline.md written | [x] |
| `assay derive` computes basic signals | [ ] |
| StructuralOracle wired into scry | [ ] |
| Lab metrics validate improvement | [ ] |
| build.md updated with direction | [x] |
