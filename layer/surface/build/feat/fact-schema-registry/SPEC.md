---
type: feat
id: fact-schema-registry
status: active
created: 2026-02-17
updated: 2026-02-18
sessions:
  origin: 20260217-114500
  completed:
  - 20260217-224547  # Prerequisite: offset consolidation, forge in oxidize, spec revision
  - 20260218-065114  # Session 1: Phase A + partial B
  - 20260218-104008  # Session 2: Phase B completion + C1-C3
related:
- layer/surface/build/feat/patina-polymorphic-extraction/SPEC.md
beliefs:
- beliefs-are-the-product
- patina-identity
- unix-philosophy
- dependable-rust
---

# feat: Fact Schema Registry — Declarative Data Contracts for Patina

> Extend polymorphic extraction with a registry of fact schemas expressed in WIT.
> Connectors/plugins declare schemas once, and the host auto-generates storage,
> validation, and embedding metadata. This makes new domains first-class without
> core code edits.

## Problem

Polymorphic extraction unblocks multiple payload kinds, but each new fact type
still demands hand-written Rust structs, SQLite tables, embedding offsets, and
assay plumbing. That makes onboarding non-code domains slow and error-prone.
We need a declarative way to describe facts so the runtime can:

- Validate payloads emitted by plugins/connectors
- Create/upgrade storage schemas (eventlog views, tables, FTS indexes)
- Tell oxidize/scry how to embed/enrich new domains
- Share schemas across projects (Mother) without rebuilds

## Proof Domain: Forge

Forge (GitHub issues + PRs) is the proof domain for this spec. Forge already has:
- Connectors (`scrape forge`, `forge sync`) writing staging files
- A WASM pipeline plugin (`grammar-forge`) producing typed facts
- Storage in eventlog with `forge.issue`/`forge.pr` event types
- Embedding support in oxidize (`FORGE_ID_OFFSET`) and enrichment
- FTS5 indexing via `code_fts` and assay search

The exit criteria validate that schema-generated artifacts match what we
hand-wrote for forge. Future domains (workspace, book projects) follow the
same pattern without core code edits.

## Proposal

### Phase A — WIT Schema Packages ✓ (Session 1)
1. ✓ Define `package patina:schema/<name>@<version>` WIT files describing fact
   structs, index hints, and embedding metadata. → `wit/schema/forge/forge.wit`
2. ✓ Update plugin manifests to reference schema packages (e.g.,
   `[schemas.forge] package = "patina:schema/forge@1.0.0"`).
3. ✓ Extend `patina-host/host.wit` with a `schema` interface so connectors can
   query metadata (field types, required keys, offsets). → 9 host.wit copies.

### Phase B — Registry & Tooling ✓ (Sessions 1-2)
1. **Deferred:** Mother registry (`~/.patina/mother/schemas`, `patina mother
   schema sync`) deferred until Mother daemon is active. Not blocking.
2. ✓ CLI `patina schema new` scaffolds a schema WIT package + metadata (owner,
   base type, version). Includes linting for collisions and naming rules.
3. ✓ `patina schema install <package>` writes schema files under
   `.patina/schemas/` for local projects.

### Phase C — Code Generation via CLI ✓ (Session 2)

Generator approach: `patina schema generate` (explicit CLI command, not
build.rs). Matches project philosophy: explicit build steps, rebuildable
artifacts, no surprise rebuild churn. Generated files are checked in and
diffable.

#### C1 — Generate Rust Types + Validation ✓
1. ✓ `patina schema generate --types` reads installed schemas and generates
   Rust structs/enums with serde derives. Includes minimal WIT parser for
   records/enums/types and kebab→snake/pascal name converters.
2. Scrape runtime loads schema metadata at startup; when a plugin emits a
   fact, the host validates it against the schema and routes it to the
   generated storage layer. → **Validation hook: Session 3.**
3. ✓ Exit checkpoint: generated forge types match hand-written ones
   (IssueState, PrState, Comment, Issue, PullRequest — all fields match).

#### C2 — Generate SQLite Migrations ✓
1. ✓ `patina schema generate --migrations` emits SQLite migrations for
   tables, indexes, and FTS views per schema. WIT→SQLite type mapping,
   auto-indexes on state and temporal columns.
2. Migration runner applies/upgrades schemas on `patina scrape`.
   → **Runner integration: Session 3.**
3. ✓ Exit checkpoint: generated forge DDL matches create_materialized_views()
   structure (forge_issues, forge_prs tables, indexes, FK to eventlog).

#### C3 — Generate Embedding Config ✓
1. ✓ `patina schema generate --embeddings` emits offset assignments and
   oxidize corpus queries per schema.
2. ✓ Offsets drawn from `embeddings::offsets` registry (single source of truth).
   Generated FORGE_ID_OFFSET = 5_000_000_000 matches hand-written.
3. Exit checkpoint: oxidize indexes forge using generated config, scry
   returns forge results. → **Wiring: Session 3.**

### Phase D — Agent & Spec Integration (Session 3)
1. Specs reference schema IDs in frontmatter (e.g., `schemas: [forge]`)
   so governance knows which fact types a spec introduces.
2. MCP exposes `schemas.list`/`schemas.show` so agents know what facts they
   can emit/query.
3. Documentation updates: `layer/core/spec-driven-design.md` gains guidance
   on introducing schemas + required beliefs/specs.

## Rollback / Safety
- Registry opt-in: projects can set `schema_registry = false` to keep manual
  table definitions.
- Generated code lives under `src/generated/schemas/`; deleting it reverts
  to hand-written storage. A checksum guards against drift.
- Schema versions are semantic; downgrades prompt explicit confirmation.

## Exit Criteria
1. ✓ Forge schema (`patina:schema/forge@1.0.0`) defined in WIT, installed
   locally, and used by `grammar-forge` plugin + host without manual
   Rust/SQL additions beyond what the generator emits.
2. ✓ `patina schema new` scaffolds a schema, validates it, and installs it.
3. `patina scrape` rejects malformed forge facts before they hit the DB,
   citing the schema violation. → **Session 3**
4. ✓ Generated Rust types for forge match the hand-written `forge.issue` /
   `forge.pr` structures (diff is empty or cosmetic-only).
5. ✓ Generated SQLite migrations produce tables identical to current hand-written
   forge tables.
6. Oxidize embeds forge facts using generated embedding config (offset from
   `embeddings::offsets`, corpus query from schema metadata). → **Session 3**
7. `patina scry` returns forge results using generated enrichment metadata.
   → **Session 3**
8. MCP tools `schemas.list` / `schemas.show` expose forge schema to agents.
   → **Session 3**
