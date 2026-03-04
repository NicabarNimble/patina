---
type: feat
id: persona-federation
status: draft
created: 2026-03-04
blocked_by:
- mother-maturation
sessions:
  origin: 20260303-184231
related:
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
  text: personas can be linked through Mother with directional, scoped knowledge streams
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
3. Edge apps need to identify themselves to Mother
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

1. Add `persona_registry` table to `mother.db`
2. Add `persona_links` table to `mother.db`
3. Modify `patina init` to select/create persona
4. Add `persona_uid` to project config (links project to persona)
5. Update belief creation to use persona UID from project config
6. Implement belief stream routing in Mother (push/pull per link config)
7. Add `patina persona list/create/link` commands
8. Add visibility filtering to `patina mother` search

## Exploration Needed

- **Migration path for existing beliefs.** 191+ beliefs with
  `persona: architect`. Create a default persona UID and migrate?
  Or leave existing beliefs as-is and only apply to new ones?

- **Lake registry relationship.** [[data-architecture-v3]] adds lake
  registry to Mother. Should lake access be persona-scoped? (Persona A
  can access lake X but not lake Y.) **Lean toward: yes, lakes are
  accessed through personas. A persona's project config declares which
  lakes it consumes.**

- **Edge app personas.** A Cloudflare chat agent is a persona. It
  generates events that flow back to local Patina. How does it
  authenticate with Mother? API key? JWT? This is the edge interface
  design — not specced yet. See [[local-first-edge-deployable]].

- **E2EE on belief streams.** Belief streams between personas could
  be encrypted end-to-end (Signal protocol). Mother routes but can't
  read. User's blockchain/crypto background makes this a priority
  direction. See [[content-addressed-references]]. **Not in scope
  for this spec but the linking architecture should not preclude it.**

- **Persona as org unit.** "Shared" visibility implies org-scoped
  personas. What is an "org" in Mother? A group of linked personas?
  A separate registry? **Needs design.**

## Non-Goals

- **Mac app UI for Mother.** Backend architecture only.
- **Multi-tenant security / SLA guarantees.** Future hardening spec.
- **Specific connector implementations.** Connectors use personas but
  are separate specs.
- **E2EE implementation.** Architecture should allow it, but not build it.
