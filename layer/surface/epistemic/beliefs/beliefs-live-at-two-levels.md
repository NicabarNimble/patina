---
type: belief
id: beliefs-live-at-two-levels
persona: architect
facets: [architecture, beliefs, persona, knowledge, storage]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-21
revised: 2026-04-09
---

# beliefs-live-at-two-levels

Beliefs live at two levels: project-level beliefs live in the project's git repo (layer/surface/epistemic/beliefs/) and travel with the code — they're facts about THIS codebase. Voice-level beliefs live in Mother's state, crypto-scoped by voice keypair — they're knowledge about how a person or team works, spanning projects. Both are signed by the voice keypair. Project beliefs die with the project. Voice beliefs persist across projects and sync across Mothers via P2P.

## Statement

Beliefs live at two levels: project-level beliefs live in the project's git repo (layer/surface/epistemic/beliefs/) and travel with the code — they're facts about THIS codebase. Voice-level beliefs live in Mother's state, crypto-scoped by voice keypair — they're knowledge about how a person or team works, spanning projects. Both are signed by the voice keypair. Project beliefs die with the project. Voice beliefs persist across projects and sync across Mothers via P2P.

## Evidence

- [[session-20260320-212325-011658000]]: Converged through walkthrough of public repo, private repo, and multi-machine identity scenario. Project beliefs are code-specific (pagination-is-cursor-based). Voice beliefs are context-specific (prefer-sync-first-rust, deploy-windows-tuesday-thursday). Different lifetimes, different storage, same signing key. (weight: 0.95)

## Supports

- [[oxidized-knowledge]] — project knowledge is git-tracked and shared, voice knowledge is scoped differently. This belief refines the split with crypto-scoped voice storage.
- [[agents-are-guests-mother-is-infrastructure]] — Mother stores voice beliefs because Mother is the infrastructure that persists across agents and projects.
- [[four-layer-architecture]] — beliefs at the center. This belief clarifies WHERE those beliefs physically live at two levels.

## Attacks

- [[persona-is-a-patina-instance]] (scoped) — the old model implied persona beliefs live in a separate Patina instance. Now they live in Mother's state, crypto-scoped as voice identity.

## Attacked-By

- "Splitting beliefs across two locations creates sync complexity" — project beliefs are in git, voice beliefs are in Mother. A belief that starts as project-specific might become voice-level. Migration path needed.
- "Voice beliefs in Mother state aren't git-tracked" — valid concern. May need a voice-level git repo or export mechanism for auditability.

## Applied-In

- Current: all beliefs live in project `layer/` — this works for single-project, single-voice. The two-level split activates when voices and multi-project work arrive.

## Revision Log

- 2026-03-21: Created — metrics computed by `patina scrape`
- 2026-04-09: Revised — updated Era 3 terminology from persona-level to voice-level belief storage.
