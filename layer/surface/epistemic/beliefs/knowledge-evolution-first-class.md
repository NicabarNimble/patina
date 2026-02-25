---
type: belief
id: knowledge-evolution-first-class
persona: architect
facets: [knowledge-management, epistemic, architecture]
entrenchment: very-high
status: active
endorsed: true
extracted: 2026-02-22
revised: 2026-02-22
---

# knowledge-evolution-first-class

Knowledge has lifecycle states - evolution through hypothesis → validation → refutation is normal, not exceptional - distinguish theory from proven fact to prevent knowledge graph pollution

## Statement

Knowledge has lifecycle states - evolution through hypothesis → validation → refutation is normal, not exceptional - distinguish theory from proven fact to prevent knowledge graph pollution

## Evidence

- [[session-20260222-054702]]: [[session-20260222-054702]] - Discovered false beliefs in system (keychain-ssh claims complete but never worked). Motivated spec-knowledge-evolution to add lifecycle states to all beliefs/specs (weight: 0.9)

## Supports

- [[evidence-driven-validation]]: Lifecycle states enforce evidence requirements (can't transition to validated without proof)
- [[refutation-is-discovery]]: Refuted status preserves learning from failed hypotheses
- [[build-correct-not-temporary]]: Building lifecycle system correctly from start, not bolting on later

## Attacks

- Current belief/spec system: No lifecycle states, everything treated as validated truth
- "Short-term fix" mentality: "We'll add lifecycle later" → never happens

## Attacked-By

- Complexity objection: "Too much overhead to track lifecycle states"
  - **Defeated**: Pollution from false beliefs is worse than discipline of lifecycle tracking
- Friction concern: "Slows down belief capture"
  - **Defeated**: Capturing false beliefs is worse than slower, accurate capture

## Applied-In

- [[spec-knowledge-evolution]]: Complete redesign of belief/spec system with lifecycle built-in
- Future belief schema v2: Required `status:` and `confidence:` fields
- Future `patina belief hypothesis` command: Create hypotheses, test, then promote or refute

## Revision Log

- 2026-02-22: Created — metrics computed by `patina scrape`
