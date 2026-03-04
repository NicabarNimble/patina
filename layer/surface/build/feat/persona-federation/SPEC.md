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
- knowledge-system-architecture
- forge-plugin-extraction
- core-plugin-extraction
beliefs:
- persona-is-a-patina-instance
- beliefs-are-the-product
- patina-is-domain-agnostic-knowledge-system
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
- id: lake-registry
  text: Mother manages a data lake registry — name, kind, location, credentials — extending the existing ref repo pattern
  checked: false
---
# feat: Persona registry, belief provenance, and Mother-federated linking

> Personas become full Patina instances with Mother-assigned UIDs,
> belief provenance, directional linking, and visibility levels.
> Each persona has its own beliefs, plugins, and projects.

## Problem

Personas are currently a dead string field — all 191 beliefs use
`persona: architect`, never varied. There's no registry, no UIDs,
no federation across personas. A production Patina monitoring Google
Workspace should be a separate persona from a dev instance tracking
code architecture, connected through Mother but sovereign in their
knowledge.

## Solution

Three mechanisms: identity, provenance, federation.

**Identity:** Mother persona registry with UIDs. `patina init` selects
or creates a persona. Each persona is a full Patina instance — own
beliefs, own plugins, own projects.

**Provenance:** Every belief carries its persona UID. When beliefs
flow between personas via Mother, provenance tracks origin. No
anonymous beliefs.

**Federation:** Personas link through Mother with directional, scoped
streams. Developer-Nick's architecture decisions can flow to
ABC-Production's operational beliefs, but not the reverse unless
explicitly configured. Visibility levels (private/public/shared)
control discoverability.

## Exit Criteria

See frontmatter.

## Non-Goals

- **Mac app UI for Mother.** This is backend architecture. UI is a
  separate concern.
- **Multi-tenant security / SLA guarantees.** Production hardening
  is a future spec built on this foundation.
- **Specific connector implementations.** Google Workspace, Obsidian,
  Slack plugins are separate specs that use the persona infrastructure.
