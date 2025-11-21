# Build Recipe

Persistent task tracking across sessions. Check items as completed, add notes inline.

**Specs:** Detailed implementation specs live in `layer/surface/spec-*.md`. Each phase below links to its spec.

---

## Active

- [ ] Unified eventlog - refactor scrapers to populate single patina.db

## Queued

### Phase 1: Scrape Pipeline (functional, needs unification)
**Specs:**
- [spec-eventlog-architecture.md](../surface/spec-eventlog-architecture.md) - LiveStore pattern, multi-user alignment
- [spec-scrape-pipeline.md](../surface/spec-scrape-pipeline.md) - Implementation details

Materialize SQLite views from event sources (git history, session files, code).

**Completed (separate DBs):**
- [x] `patina scrape git` - commits, co-changes → git.db (2025-11-21)
- [x] `patina scrape sessions` - observations, decisions → sessions.db (2025-11-21)
- [x] `patina scrape code` - AST, call_graph → code.db
- [x] `patina scrape` - run all three (2025-11-21)

**Next (unified eventlog):**
- [ ] Refactor to populate unified `patina.db` with eventlog table
- [ ] Create materialized views from eventlog
- [ ] Validate cross-cutting queries

### Phase 2: Oxidize (Embeddings + Projections)
**Spec:** [layer/surface/spec-oxidize.md](../surface/spec-oxidize.md)

Recipe-driven embedding and projection training.

- [ ] `oxidize.yaml` recipe format
- [ ] `patina oxidize` - recipe + SQLite → vectors
- [ ] Embedding model plugins (E5, BGE, nomic)
- [ ] Dimension projections (semantic, temporal, dependency, etc.)
- [ ] World-model projections (state-encoder, action-encoder, transition-predictor)

### Phase 3: Scry (Query Interface)
**Spec:** [layer/surface/spec-scry.md](../surface/spec-scry.md)

LLM ↔ database query interface.

- [ ] `patina scry "query"` - unified search
- [ ] Vector search + SQLite metadata
- [ ] Project + persona result merging
- [ ] Result tagging ([PROJECT], [PERSONA])
- [ ] Prolog reasoning integration (optional)

### Phase 4: Mothership Service
**Spec:** [layer/surface/spec-mothership-service.md](../surface/spec-mothership-service.md)

Local daemon for embeddings and cross-project queries.

- [ ] `patina serve` daemon (axum REST on :50051)
- [ ] `POST /embed` - generate embeddings
- [ ] `POST /scry` - unified query endpoint
- [ ] `GET /projects` - list registered projects
- [ ] `projects.registry` (YAML)
- [ ] Recipe version tracking

### Phase 5: Persona
**Spec:** [layer/surface/spec-persona-capture.md](../surface/spec-persona-capture.md)

Personal cross-project beliefs (not git-tracked).

- [ ] `patina persona note "belief"` - capture to ~/.patina/persona/
- [ ] `patina persona query "term"` - search personal beliefs
- [ ] Persona materialize (events → beliefs.db)
- [ ] Integration with scry (tagged results)

### Phase 6: Multi-User & Sharing
**Spec:** [layer/surface/spec-cross-project.md](../surface/spec-cross-project.md)

Multi-user workflows, recipe sharing.

- [ ] Recipe-based adapter rebuilding
- [ ] Version tracking (recipe version + events hash)
- [ ] Peer discovery (Bonjour/mDNS) - future
- [ ] P2P adapter sharing - future

---

## Done

- [x] E5-base-v2 model working (2025-11)
- [x] USearch HNSW indices working (2025-11)
- [x] SQLite + call_graph data available (2025-11)
- [x] `patina scrape code` working (2025-11)
- [x] Mothership architecture clarified - Ollama-style daemon (2025-11-21)
- [x] README rewritten with accurate commands (2025-11-21, bf22318e)
- [x] MIT license added (2025-11-21, bf22318e)
- [x] Multi-user architecture designed (2025-11-21, session 20251121-065812)
- [x] Recipe model for adapter sharing (2025-11-21, session 20251121-065812)

---

## Architecture Summary

**Key Insight:** Git commits and session files ARE the event sources. Scrape materializes them into a unified event log.

**Pipeline:**
```
Event Sources (git-synced)     →  scrape  →  Unified DB    →  oxidize  →  Vectors
.git/ (commits)                              patina.db                    *.usearch
layer/sessions/*.md                          ├── eventlog (source of truth)
src/**/* (AST)                               └── materialized views
```

**Database Structure (following LiveStore pattern):**
```
patina.db
├── eventlog                    ← Source of truth: ALL events unified
│   ├── git.commit events
│   ├── session.* events
│   └── code.* events
│
└── Materialized Views          ← Derived from eventlog
    ├── commits, co_changes     (from git events)
    ├── sessions, observations  (from session events)
    └── functions, call_graph   (from code events)
```

**What's Shared (git-tracked):**
- `.git/` - commit history = temporal events
- `layer/sessions/*.md` - session events (decisions, observations)
- `.patina/oxidize.yaml` - recipe for building adapters

**What's Local (rebuilt via scrape):**
- `.patina/data/patina.db` - unified eventlog + materialized views
- `.patina/data/embeddings/` - vectors built from recipe

**What's Personal:**
- `~/.patina/persona/` - cross-project beliefs
- `~/.patina/projects.registry` - registered projects

**Adapter Structure:**
```
src/adapters/
├── foundational/       ← LLM chat (Claude, Gemini)
├── embeddings/         ← frozen models (E5, BGE)
└── projections/        ← learned layers
    ├── dimensions/     ← semantic, temporal, etc.
    └── world-model/    ← state-encoder, etc.
```

---

## Notes

- Transport: REST + optional WebSocket (not gRPC), port 50051
- Registry format: YAML in `projects.registry`
- Personas: personal beliefs, never shared via git
- Adapters: ~4MB each, share recipes not weights
- North star: CRDT persona network (far future)
- Design docs: `layer/surface/patina-embedding-architecture.md`
