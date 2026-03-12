---
type: feat
id: persona-federation
status: draft
created: 2026-03-04
blocked_by: []
sessions:
  origin: 20260303-184231
related:
- agentic-surface-architecture
- session-narrative-system
- forge-plugin-extraction
- core-plugin-extraction
- data-architecture-v3
beliefs:
- persona-is-a-patina-instance
- beliefs-are-the-product
- patina-is-domain-agnostic-knowledge-system
- patina-is-beliefs-plus-action
- mother-is-connection-and-continuity
exit_criteria:
- id: persona-registry
  text: Mother manages a persona registry with UIDs — `patina init` can select or create a persona
  checked: false
- id: belief-provenance
  text: beliefs carry persona provenance — the `persona` field maps to a Mother-registered UID, not a hardcoded string
  checked: false
- id: persona-linking
  text: personas can be linked through Mother with directional, scoped configuration (push/pull/bidirectional, facet filtering)
  checked: false
- id: persona-visibility
  text: 'personas have visibility levels: private (invitation only), public (discoverable), shared (org-scoped)'
  checked: false
---
# feat: Persona registry, belief provenance, and Mother-federated linking

> Personas become full Patina instances with Mother-assigned UIDs,
> belief provenance, directional linking, and visibility levels.

## Context

**Architecture context:**
- [[session-20260303-190855]] — "Mother has personas which divide
  belief networks. These are connected nodes in the network. A persona
  is a Patina instance." Established that projects = action layer +
  belief layer, personas separate them, Mother federates them.
- [[session-20260304-120702]] — refined: multi-user is federation,
  not shared state. Edge apps are child nodes in Mother's network.
  A CRM on Cloudflare and a dev tool on localhost are two personas
  under the same Mother.
- [[persona-is-a-patina-instance]] — personas are sovereign knowledge
  systems, not string labels
- [[mother-is-connection-and-continuity]] — Mother manages persona
  registry and routes belief streams between linked personas

## Problem

Personas are currently a dead string field — all 191+ beliefs use
`persona: architect`, never varied. There's no registry, no UIDs,
no federation across personas.

This matters because:
1. A production Patina (monitoring email, building business beliefs)
   should be a separate persona from a dev Patina (tracking code)
2. Cross-project learning ("evolve" verb) requires persona identity
   to track where knowledge came from
3. Interfaces and edge apps need to identify themselves to Mother
4. Multi-user scenarios need per-user personas, not shared state

## Solution

Three mechanisms: identity, provenance, federation.

**Identity:** Mother persona registry with UIDs.

```sql
CREATE TABLE persona_registry (
    uid         TEXT PRIMARY KEY,   -- UUID
    name        TEXT NOT NULL,      -- human name
    visibility  TEXT NOT NULL,      -- "private", "public", "shared"
    created     TEXT NOT NULL,      -- ISO 8601
    metadata    TEXT                -- JSON: description, tags, etc.
);
```

`patina init` selects or creates a persona. Each persona is a full
Patina instance — own beliefs, own plugins, own projects.

Interactive interfaces and edge apps check in to Mother under a persona
context. They do not bypass persona boundaries and reach children or
belief streams directly.

**Provenance:** Every belief carries its persona UID. The `persona`
field in belief frontmatter maps to a Mother-registered UID. When
beliefs flow between personas, provenance tracks origin.

**Federation:** Personas link through Mother with directional, scoped
streams.

```sql
CREATE TABLE persona_links (
    from_uid    TEXT NOT NULL,      -- source persona
    to_uid      TEXT NOT NULL,      -- target persona
    direction   TEXT NOT NULL,      -- "push", "pull", "bidirectional"
    scope       TEXT,               -- filter: facets, belief IDs, etc.
    created     TEXT NOT NULL,
    UNIQUE(from_uid, to_uid)
);
```

Developer-Nick's architecture decisions can flow to ABC-Production's
operational beliefs, but not the reverse unless explicitly configured.

## Steps

1. Remove pre-pivot `patina persona` command (`src/commands/persona/`),
   its path helpers (`src/paths.rs` persona module), and
   `~/.patina/personas/default/` directory structure — this is legacy
   code from before the architectural pivot, not the foundation
2. Add `persona_registry` table to `mother.db`
3. Add `persona_links` table to `mother.db`
4. Modify `patina init` to select/create persona
5. Add `persona_uid` to project config (links project to persona)
6. Update belief creation to use persona UID from project config
7. Implement belief stream routing in Mother (push/pull per link config)
8. Add `patina persona list/create/link` commands (new, Mother-level)
9. Add visibility filtering to `patina mother` search

## Design Decisions (resolved in DESIGN.md)

- **Migration: create default persona, retroactive UID.** Register
  "architect" persona in Mother → write UID to project config → scrape
  updates belief files. One-time migration is cleaner than permanent
  dual-resolution. `patina scrape` already reads and rewrites belief
  frontmatter — add UID during a scrape cycle.

- **Persona registry in graph.db.** Same reasoning as lake registry —
  extend existing Mother database. graph.db already holds federated
  belief data; persona registry is metadata FOR beliefs. SQL is the
  right tool (UIDs, links, queries), not YAML.

- **1:N persona→project.** A persona owns multiple projects. A project
  belongs to exactly one persona. If two personas both need the same
  data, they federate through Mother — they don't share a project
  directory. Project config gains a `persona_uid` field.

- **Persona discovery: local-only first.** Same machine, same Mother
  instance. Network-wide discovery is a fundamentally different
  mechanism — defer until there's a real use case.

- **Org is deferred.** A label on persona_links, not a first-class
  entity. Build linking first, add org semantics when a real use case
  emerges.

## Exploration Needed (genuinely open)

- **Lake access control.** Should lake access be persona-scoped? Lean
  toward yes — a persona's project config declares which lakes it
  consumes. But lakes don't exist yet ([[spec-data-architecture-v3]]
  builds them). Design this when both specs are in flight.

- **Edge app authentication.** How does a Cloudflare Worker persona
  authenticate with Mother? Deferred to edge-interface design per
  [[local-first-edge-deployable]]. The UID is opaque — it doesn't
  encode location. Remote personas are just personas Mother knows about
  but can't directly reach without a transport layer.

- **E2EE on belief streams.** Not in scope but the linking architecture
  must not preclude it. Mother routes payloads she can't read.
  [[content-addressed-references]] keeps the path open.

## Non-Goals

- **Mac app UI for Mother.** Backend architecture only.
- **Multi-tenant security / SLA guarantees.** Future hardening spec.
- **Specific connector implementations.** Connectors use personas but
  are separate specs.
- **E2EE implementation.** Architecture should allow it, but not build it.
