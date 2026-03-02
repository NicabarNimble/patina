---
type: refactor
id: knowledge-system-architecture
status: draft
created: 2026-03-02
blocked_by:
- data-architecture-v2
sessions:
  origin: 20260302-072907
related:
- data-architecture-v2
- spec-plugin-extraction
- fact-crdt-substrate
- data-measure-surface
- data-fast-incremental
beliefs:
- patina-is-domain-agnostic-knowledge-system
- persona-is-a-patina-instance
- patina-is-knowledge-protocol
- beliefs-are-the-product
exit_criteria:
- id: scrape-is-plugin-dispatched
  text: '`patina scrape` dispatches to plugins by source kind — code, forge, and at least one new source use the same dispatch interface'
  checked: false
- id: schemas-live-with-plugins
  text: fact schemas (WIT-defined types, table definitions) ship with the plugin, not in Patina core
  checked: false
- id: plugins-can-emit-facts
  text: SDK exposes fact emission — plugins can write staging files or emit events through the host boundary
  checked: false
- id: forge-extracted-to-plugin
  text: forge connector (GitHub) runs as a plugin, not built into `src/forge/`
  checked: false
- id: spec-extracted-to-plugin
  text: spec subsystem runs as a plugin, not built into `src/spec/`
  checked: false
- id: sessions-extracted-to-plugin
  text: session subsystem runs as a plugin, not built into `src/session/`
  checked: false
- id: persona-uid-in-mother
  text: Mother manages a persona registry with UIDs — `patina init` can select or create a persona
  checked: false
- id: persona-owns-beliefs
  text: beliefs carry persona provenance — the `persona` field maps to a Mother-registered UID, not a hardcoded string
  checked: false
- id: persona-linking
  text: personas can be linked through Mother with directional, scoped knowledge streams
  checked: false
- id: public-private-personas
  text: personas have visibility levels — private (invitation only), public (discoverable), shared (org-scoped)
  checked: false
- id: lake-registry-in-mother
  text: Mother manages a data lake registry — name, kind, location, credentials — extending the existing ref repo pattern
  checked: false
- id: core-is-domain-agnostic
  text: Patina core has no domain-specific code — no Rust syntax knowledge, no GitHub API knowledge, no email parsing. All domain logic lives in plugins.
  checked: false
---
# refactor: Patina as Domain-Agnostic Knowledge System with Persona Federation

> Restructure Patina from a development tool into a domain-agnostic
> knowledge system. Plugin system receives all domain-specific logic
> (scrape dispatch, schemas, grammars). Personas are full Patina instances
> with own beliefs/plugins/projects, federated through Mother.

## Problem

Patina is currently a development tool with domain-specific code baked into
core: forge (GitHub API), code grammars (Rust, Python), spec management,
session tracking. This limits Patina to software development projects.

But the core engine — event sourcing, beliefs, search, embeddings, Mother
federation — is domain-agnostic. A production Patina instance monitoring a
Google Workspace data lake and building business beliefs should use the same
engine as a development instance tracking code architecture. The domain
specifics should be plugins, not core.

Additionally, personas are currently a string field (`persona: architect`)
on beliefs with no system support. They should be full Patina instances —
sovereign knowledge systems with their own beliefs, plugins, and projects,
connected through Mother's federation.

## Current State

**Core is tangled with domain logic:**
- `src/forge/` — GitHub-specific API code, `gh` CLI integration
- `src/spec/` — spec lifecycle management (should be a plugin)
- `src/session/` — session tracking (should be a plugin)
- Code grammars — pipeline plugins exist but dispatch is partially built-in
- Forge schema (`forge.wit`) — lives in `.patina/schemas/`, not with plugin

**Plugin system gaps:**
- No fact emission API — plugins can read (scry/assay/context) but not write
- No scrape plugin dispatch — `patina scrape` has hardcoded code/forge paths
- Schemas are managed centrally, not shipped with plugins
- SDK (`patina-sdk`) supports 4 worlds but no data ingestion pattern

**Persona is a dead field:**
- All 179 beliefs use `persona: architect` — never varied
- No persona registry, no UIDs, no federation across personas
- No concept of public/private personas or persona linking

**Mother manages ref repos but not data lakes:**
- Ref repo registry exists and works
- No generalized data lake registry
- No credential management for external data sources beyond forge

## Target State

### Patina Core (domain-agnostic)

The core provides:
- **Event sourcing** — append-only eventlog, projections, materialization
- **Belief system** — evidence chains, supports/attacks, grounding, evolution
- **Search** — FTS5 (assay), semantic vectors (scry), progressive disclosure (context)
- **Embeddings** — ONNX Runtime, model management
- **Plugin dispatch** — load WASM plugins, route scrape/pipeline/command/task
- **Mother** — federation, persona registry, lake registry, credential management

The core does NOT know about: Rust syntax, GitHub APIs, email headers,
calendar formats, Obsidian wiki-links, or any domain-specific data shape.

### Plugin System (domain knowledge)

Plugins bring domain expertise through three roles:

| Role | What it does | Plugin type |
|------|-------------|-------------|
| **Fetch** | Get data from source into a known local shape | Task or mother-child |
| **Scrape** | Read local data, produce normalized facts | Pipeline (grammar pattern) |
| **Schedule** | Manage sync lifecycle, monitor freshness | Mother-child |

Each plugin ships its own schema (WIT-defined types), its own table
definitions, and its own FTS/embedding configuration. `patina scrape`
dispatches to the right plugin based on source kind.

Examples:
- `code` — grammar plugins for Rust, Python, etc. (partially exists)
- `forge` — GitHub/Gitea issues and PRs (extract from core)
- `google-workspace` — email, calendar, drive (new)
- `obsidian` — markdown notes with wiki-links (new)
- `slack`, `zoom`, `office-365` — future connectors

### Personas (sovereign instances)

A persona is a full Patina instance:
- Own beliefs, own plugins, own projects, own knowledge
- Mother-assigned UID — provenance marker on every belief
- Visibility: private, public, or shared (org-scoped)
- Linked through Mother with directional, scoped streams

```
MOTHER (federation)
  │
  ├── Persona: "Developer-Nick" (private)
  │   ├── plugins: code grammars, forge
  │   ├── projects: patina, client-tools
  │   └── beliefs: architecture decisions
  │
  ├── Persona: "ABC-Production" (private, linked to Developer-Nick)
  │   ├── plugins: google-workspace, monitoring
  │   ├── projects: email-automation
  │   └── beliefs: operational patterns (streams to/from dev)
  │
  └── Persona: "Open-Source-Patina" (public)
      ├── plugins: code grammars, forge
      └── beliefs: discoverable by anyone
```

### Data Lakes (Mother-managed external data)

Mother's ref repo registry generalizes into a data lake registry:
- Name, kind, location, credentials, sync mode
- Projects reference lakes, don't own them
- Catalog (metadata index) is cheap — run on everything
- Content pull is project-scoped via projection config

## Phases

### Phase 1: Plugin System Completion
**Goal:** Plugin system can receive extracted subsystems.

- Scrape plugin dispatch — `patina scrape` routes by source kind
- Fact emission from plugins — SDK `host_emit` or staging file convention
- Schema-with-plugin — each plugin ships its own WIT schema + table defs
- Prerequisite: [[data-measure-surface]] (completes [[data-architecture-v2]])

### Phase 2: Core Extraction
**Goal:** All domain-specific code moves to plugins.

- Forge → plugin (first extraction, proves pattern end-to-end)
- Spec → plugin (complex extraction, involves CLI surface)
- Sessions → plugin (simpler, fewer dependencies)
- Code grammars dispatch cleanup (partially done via pipeline plugins)
- [[spec-plugin-extraction]] covers spec extraction in detail

### Phase 3: Mother and Personas
**Goal:** Persona federation is operational.

- Persona registry with UIDs in Mother
- `patina init` persona selection/creation
- Belief provenance — `persona` field maps to Mother UID
- Persona linking with directional, scoped streams
- Public/private/shared visibility
- Lake registry extending ref repo pattern
- [[fact-crdt-substrate]] provides sync mechanism

## Non-Goals

- **Mac app for Mother** — desirable but separate concern. This spec
  covers the backend architecture, not the UI surface.
- **Production hardening** — scaling to thousands of users, multi-tenant
  security, SLA guarantees. Those are future specs built on this foundation.
- **Specific connector implementations** — Google Workspace, Obsidian,
  Slack plugins are separate specs. This spec provides the infrastructure
  they plug into.

## Design Decisions

### Why extract before federation?
You can't federate a tangled system. If forge is baked into core, a
non-development persona still ships with GitHub code it doesn't need.
Extraction makes Patina lean — each persona installs only what it needs.

### Why personas as instances, not filters?
The original design had personas as belief filters within one instance.
This was fragile — same evidence, different weights? Separate instances
are cleaner. Each builds knowledge from its own experience. Mother
provides cross-persona discovery without polluting belief sovereignty.
See [[persona-is-a-patina-instance]].

### Why lake registry in Mother?
Ref repos already work this way — Mother-managed, shared across projects.
Data lakes are the same pattern with different storage backends. One
registry, one credential store, one sync lifecycle. Projects declare
what they need, Mother provides the pipe.

### Why scrape dispatch over separate commands?
`patina scrape google-workspace` vs `patina google-workspace scrape`.
Single entry point (`scrape`) with plugin dispatch is simpler —
consistent interface, one command to learn, plugins extend it.
Matches the grammar pattern: `patina scrape` already routes to
grammar-rust vs grammar-python by file extension.

## References

- [[patina-is-domain-agnostic-knowledge-system]] — foundational belief
- [[persona-is-a-patina-instance]] — persona architecture belief
- [[patina-is-knowledge-protocol]] — protocol framing (evolved: drop "development")
- [[beliefs-are-the-product]] — why this matters
- [[data-architecture-v2]] — internal data stack (prerequisite)
- [[session-20260302-072907]] — design session where this emerged
- [[session-20260302-061023]] — QMD analysis that primed the reframe
