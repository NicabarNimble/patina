---
type: feat
id: mother-beliefs
status: abandoned
created: 2026-02-09
sessions:
  origin: 20260209-215657
related:
- layer/surface/build/feat/mother/SPEC.md
- layer/surface/build/feat/mother-v2/SPEC.md
- layer/surface/build/feat/mother-repos/SPEC.md
- layer/surface/build/feat/surface-layer/SPEC.md
beliefs:
- mother-is-the-daemon
- four-layer-architecture
- patina-is-knowledge-layer
- corpus-composition-over-model
---

# feat: Mother Beliefs — Cross-Project Belief Layer

> Beliefs are the exposed interface between Mother and projects. Mother maintains
> a cross-project belief index (`beliefs.db`) that lets knowledge flow between
> projects. User-level beliefs live at `~/.patina/layer/surface/beliefs/`.
> Projects contribute beliefs upward; Mother makes them searchable across
> the entire workspace.

## Problem

### Beliefs Are Project-Local Islands

87 beliefs exist in `layer/surface/epistemic/beliefs/` — all project-local.
When a belief like [[dependable-rust]] proves itself across multiple projects,
there's no mechanism to share it. Each project is a self-contained island.

### Persona Notes Are Disconnected

The persona system (`~/.patina/personas/default/`) has 4 notes — cross-project
observations stored as JSONL events. These are disconnected from the belief
system:

- Persona notes don't have belief structure (no confidence, no evidence, no relationships)
- Belief scraper doesn't index persona notes
- No path from "persona observation" to "user belief" to "shared across projects"

### No User-Level Beliefs

There's no place for beliefs that span projects. "Always use append-only
event logs" isn't specific to patina — it's a user-level architectural
conviction. Today it lives in one project's beliefs or nowhere.

### At Multi-Project Scale, Scry Needs Beliefs

From [[session-20260209-160017]]: "At single-project scale (~84 beliefs), scry
is marginal — beliefs fit in a context window. At multi-project scale (Mother),
scry becomes essential — cross-project vocabulary bridging."

The [[four-layer-architecture]] positions Mother as the convergence engine.
Beliefs are what converge.

## Current State

```
# Project-level (exists):
layer/surface/epistemic/beliefs/
├── mother-is-the-daemon.md           # 87 belief files
├── dependable-rust.md
├── four-layer-architecture.md
└── ...

# User-level (does NOT exist):
~/.patina/layer/                       # not created
~/.patina/mother/beliefs.db            # not created

# Persona (disconnected):
~/.patina/personas/default/
├── events/                            # 4 JSONL events
└── persona.db                         # materialized view
```

## Solution

### 1. User-Level Belief Directory

```
~/.patina/layer/
└── surface/
    └── beliefs/                 # user-level beliefs (markdown)
        ├── append-only-events.md
        ├── read-code-before-write.md
        └── ...
```

Same format as project beliefs. `patina persona note` writes here (replacing
the disconnected JSONL event system).

### 2. Cross-Project Belief Index (`beliefs.db`)

```
~/.patina/mother/beliefs.db

Tables:
  beliefs (
    id TEXT PRIMARY KEY,        -- "dependable-rust"
    source TEXT NOT NULL,       -- "project:patina" | "user" | "ref:gastown"
    statement TEXT,
    confidence REAL,
    entrenchment TEXT,          -- low/medium/high
    file_path TEXT,             -- path to source .md file
    last_indexed TEXT
  )

  belief_fts (                  -- FTS5 for keyword search
    id TEXT,
    content TEXT                -- full markdown body
  )
```

Populated by `patina scrape` (project beliefs) and a new Mother-level
scrape (user beliefs). Federated: each source maintains its own beliefs,
Mother indexes them all.

### 3. Belief Search Across Projects

`patina scry --beliefs` or `patina context` should search `beliefs.db`,
not just the current project's beliefs. The MCP `scry` tool already has
belief support — extend it to query Mother's index.

```
Query: "how should I handle error boundaries?"

Results:
  [PROJECT:patina] graceful-degradation-over-strict-validation (0.87)
  [USER] always-validate-at-boundaries (0.82)
  [PROJECT:dojo] fail-fast-in-contracts (0.79)
```

### 4. Persona Note → User Belief Migration

`patina persona note` should write to `~/.patina/layer/surface/beliefs/`
(proper belief format) instead of the legacy JSONL event log. The 4 existing
persona notes become user beliefs via `patina persona migrate`.

This was attempted in [[session-20260209-210436]] and reverted because there
were no consumers. With `beliefs.db` as the consumer, this migration has a
purpose.

## Acceptance Criteria

1. [ ] `~/.patina/layer/surface/beliefs/` directory created by `patina init` or `ensure_user_layer()`
2. [ ] `beliefs.db` at `~/.patina/mother/beliefs.db` indexes beliefs from all sources
3. [ ] `patina scrape` contributes project beliefs to `beliefs.db`
4. [ ] Mother-level scrape indexes user beliefs from `~/.patina/layer/surface/beliefs/`
5. [ ] `patina persona note` writes user beliefs (not legacy JSONL events)
6. [ ] `patina persona migrate` converts existing persona notes to user beliefs
7. [ ] MCP `scry` tool can search `beliefs.db` for cross-project belief queries
8. [ ] `patina context` includes relevant beliefs from all sources (not just current project)

## Non-Goals

- Belief promotion (surface → core values) — future evolution, not this spec
- Belief extraction from ref repos — that's a [[mother-repos]] extension
- Rules engine (beliefs → automated actions) — too speculative for now
- Belief conflict resolution — surface it, don't auto-resolve
- Multi-user belief sharing — single user for now
