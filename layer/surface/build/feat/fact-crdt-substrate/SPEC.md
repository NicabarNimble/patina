---
type: feat
id: fact-crdt-substrate
status: draft
created: 2026-02-17
sessions:
  origin: 20260217-115200
related:
  - src/eventlog.rs
  - src/commands/scrape/forge/mod.rs
beliefs:
  - beliefs-are-the-product
  - work-triages-specs
  - git-is-the-knowledge-substrate
  - mother-is-the-daemon
---

# feat: Fact CRDT Substrate — Local-First Facts that Sync via Mother

> Replace ad hoc SQLite inserts with a schema-aware CRDT store (Jazz-style) so
> facts are append-only, offline-friendly, and replicated between projects and
> Mother. SQLite remains as the materialized view powering assay/scry/beliefs.

## Motivation

- Projects increasingly need to ingest non-code data (Workspace, research). Facts
  should sync across devices/projects without fragile export/import flows.
- Beliefs rely on facts as evidence; if facts only live in a single `.patina/db`,
  collaboration suffers.
- A CRDT layer lets us keep facts local-first (fast, offline) while syncing to
  Mother or other peers when available — no central point of failure.

## Architecture

```
connectors/plugins ──> CRDT fact store (per schema) ──> materializer ──> SQLite
                                                       │
                                                       └─> Mother replica
```

### Phase A — Prototype (Single Fact Type)
1. Embed a Jazz-like CRDT library (Rust) and wrap it with a `FactStore` trait.
2. Choose one domain (forge issues) as pilot. Ingestion writes to CRDT instead of
   SQLite; a background materializer mirrors CRDT entries into the existing
   tables (eventlog, forge_issues, FTS).
3. Provide tooling to inspect CRDT state (`patina fact log forge.issue`).
4. Validate performance + data parity vs. legacy path.

### Phase B — Schema-Aware CRDTs
1. Generate CRDT document types from schemas (see [[fact-schema-registry]]):
   - Grow-only set for append-only facts (issues, emails)
   - Map CRDT for updatable facts (calendar events) keyed by source ID
2. Each schema declares its CRDT type via metadata (`crdt = "gset"` etc.).
3. `patina scrape` writes to CRDTs using schema metadata; materializer updates
   SQLite via generated code.

### Phase C — Replication & Sync
1. Store CRDT replicas under `.patina/local/data/facts/<schema>.jazz`.
2. Mother runs as another replica in `~/.patina/mother/facts/`.
3. Commands:
   - `patina fact sync` merges local facts with Mother (UDS/TCP)
   - `patina mother fact sync --repo foo` pulls facts into Mother
4. Conflict resolution is handled by CRDT semantics; connectors record source
   IDs to avoid duplicates.

### Phase D — Tooling & Belief Integration
1. Materializer updates belief-grounding tables in near-real-time (watch CRDT
   change feed instead of waiting for `scrape`).
2. Add cues when CRDT drift is detected (e.g., local facts ahead of Mother).
3. Provide API for specs/agents: `patina fact export` to share facts, `patina
   fact tail` to stream new facts for observability.

## Rollout Plan
- 0. Pilot forge issues quietly; run CRDT + SQLite in parallel and compare checksums.
- 1. Gate CRDT per schema via config (`fact_crdt = true`).
- 2. Once stable, disable direct SQLite inserts and rely on materializer.
- 3. Document migration path; keep ability to compact CRDTs back into SQLite if
     needed (export/import utility).

## Rollback
- Feature flag per schema to fall back to direct SQLite writes.
- CRDT files are append-only; deleting them reverts to legacy behavior (after a
  full `patina scrape`).
- Sync commands detect version mismatches and refuse to merge until upgraded.

## Exit Criteria
1. Forge issues ingested via CRDT show identical query results in assay/scry and
   belief grounding counts as legacy path.
2. Local-first workflow demo: capture facts offline, later `patina fact sync`
   merges them into Mother without conflicts.
3. Schema metadata declares CRDT type; code generation produces the correct
   Rust + SQLite materializer artifacts.
4. Belief audit uses CRDT change feed to update grounding metrics within one
   minute of fact ingestion (no manual scrape).
5. Docs and tooling describe backup/compact flows for CRDT stores.

## Alignment Audit (2026-02-23, session 20260223-132543)

**Disposition: DEFER**

Reviewed against spec-workflow-rigor architectural decisions. No conflicts — this
spec is orthogonal to the spec/session lifecycle work.

**Dead references removed:**
- `fact-schema-registry/SPEC.md` — spec does not exist in tree (abandoned/never created)
- `patina-polymorphic-extraction/SPEC.md` — spec does not exist in tree (abandoned/never created)
- Replaced with actual code paths: `src/eventlog.rs`, `src/commands/scrape/forge/mod.rs`

**Architecture gap:** When implemented, should follow the three-layer capability
pattern ([[plugins-are-three-prong-bundles]]): `patina fact` CLI + MCP tools +
adapter skill. No CRDT code or dependencies exist yet — entirely aspirational.

**No urgency.** Depends on schema registry work and Mother daemon maturation.
Existing eventlog + materialized view pattern (forge_issues, beliefs table)
is working. CRDT layer is a future optimization for sync/replication.
