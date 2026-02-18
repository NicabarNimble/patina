---
type: feat
id: fact-schema-registry
status: draft
created: 2026-02-17
sessions:
  origin: 20260217-114500
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
> validation, and embedding metadata. This makes new domains (Workspace, book
> projects) first-class without core code edits.

## Problem

Polymorphic extraction unblocks multiple payload kinds, but each new fact type
still demands hand-written Rust structs, SQLite tables, embedding offsets, and
assay plumbing. That makes onboarding non-code domains slow and error-prone.
We need a declarative way to describe facts so the runtime can:

- Validate payloads emitted by plugins/connectors
- Create/upgrade storage schemas (eventlog views, tables, FTS indexes)
- Tell oxidize/scry how to embed/enrich new domains
- Share schemas across projects (Mother) without rebuilds

## Proposal

### Phase A — WIT Schema Packages
1. Define `package patina:schema/<name>@<version>` WIT files describing fact
   structs, index hints, and embedding metadata.
2. Update plugin manifests to reference schema packages (e.g.,
   `[schemas.workspace.email] package = "patina:schema/email@1.0.0"`).
3. Extend `patina-host/host.wit` with a `schema` interface so connectors can
   query metadata (field types, required keys, offsets).

### Phase B — Registry & Tooling
1. Mother hosts a registry of approved schema packages (stored in
   `~/.patina/mother/schemas`). Projects sync via `patina mother schema sync`.
2. CLI `patina schema new` scaffolds a schema WIT package + metadata (owner,
   base type, version). Includes linting for collisions and naming rules.
3. `patina schema install <package>` writes schema files under
   `.patina/schemas/` for local projects.

### Phase C — Generated Storage + Code
1. Build tooling (build.rs or CLI) that reads installed schemas and generates:
   - Rust structs/enums for facts (with serde derives)
   - SQLite migrations for tables/indexes/FTS views
   - Embedding offset assignments & oxidize configs
2. Scrape runtime loads schema metadata at startup; when a plugin emits a fact,
   the host validates it against the schema and routes it to the generated
   storage layer. No more hand-written insert functions per domain.

### Phase D — Agent & Spec Integration
1. Specs reference schema IDs in frontmatter (e.g., `schemas: [workspace.email]`)
   so governance knows which fact types a spec introduces.
2. MCP exposes `schemas.list`/`schemas.show` so agents know what facts they can
   emit/query.
3. Documentation updates: `layer/core/spec-driven-design.md` gains guidance on
   introducing schemas + required beliefs/specs.

## Rollback / Safety
- Registry opt-in: projects can set `schema_registry = false` to keep manual
  table definitions.
- Generated migrations live under `.patina/migrations/schemas/`; deleting them
  reverts to manual storage. A checksum guards against drift.
- Schema versions are semantic; downgrades prompt explicit confirmation.

## Exit Criteria
1. Workspace/email schema defined once and used by both connector + core without
   manual Rust/SQL additions.
2. `patina schema new` scaffolds a schema, validates it, and installs it locally.
3. `patina scrape` rejects malformed facts before they hit the DB, citing the
   schema violation.
4. Oxidize automatically embeds at least one non-code schema (workspace email)
   using generated config.
5. Documentation + MCP tools surface available schemas to humans/agents.
