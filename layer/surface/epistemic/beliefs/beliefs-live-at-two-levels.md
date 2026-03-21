---
type: belief
id: beliefs-live-at-two-levels
persona: architect
facets: [architecture, beliefs, persona, knowledge, storage]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-21
revised: 2026-03-21
---

# beliefs-live-at-two-levels

Beliefs live at two levels: project-level beliefs live in the project's git repo (layer/surface/epistemic/beliefs/) and travel with the code — they're facts about THIS codebase. Persona-level beliefs live in Mother's state, crypto-scoped by persona keypair — they're knowledge about how a person or team works, spanning projects. Both are signed by the persona keypair. Project beliefs die with the project. Persona beliefs persist across projects and sync across Mothers via P2P.

## Statement

Beliefs live at two levels: project-level beliefs live in the project's git repo (layer/surface/epistemic/beliefs/) and travel with the code — they're facts about THIS codebase. Persona-level beliefs live in Mother's state, crypto-scoped by persona keypair — they're knowledge about how a person or team works, spanning projects. Both are signed by the persona keypair. Project beliefs die with the project. Persona beliefs persist across projects and sync across Mothers via P2P.

## Evidence

- [[session-20260320-212325-011658000]]: Converged through walkthrough of public repo, private repo, and multi-machine persona scenario. Project beliefs are code-specific (pagination-is-cursor-based). Persona beliefs are context-specific (prefer-sync-first-rust, deploy-windows-tuesday-thursday). Different lifetimes, different storage, same signing key. (weight: 0.95)

## Supports

- [[oxidized-knowledge]] — project knowledge is git-tracked and shared, persona knowledge is scoped differently. This belief refines the split with crypto-scoped persona storage.
- [[agents-are-guests-mother-is-infrastructure]] — Mother stores persona beliefs because Mother is the infrastructure that persists across agents and projects.
- [[four-layer-architecture]] — beliefs at the center. This belief clarifies WHERE those beliefs physically live at two levels.

## Attacks

- [[persona-is-a-patina-instance]] (scoped) — the old model implied persona beliefs live in a separate Patina instance. Now they live in Mother's state, crypto-scoped.

## Attacked-By

- "Splitting beliefs across two locations creates sync complexity" — project beliefs are in git, persona beliefs are in Mother. A belief that starts as project-specific might become persona-level. Migration path needed.
- "Persona beliefs in Mother state aren't git-tracked" — valid concern. May need a persona-level git repo or export mechanism for auditability.

## Applied-In

- Current: all beliefs live in project `layer/` — this works for single-project, single-persona. The two-level split activates when personas and multi-project work arrive.

## Revision Log

- 2026-03-21: Created — metrics computed by `patina scrape`
