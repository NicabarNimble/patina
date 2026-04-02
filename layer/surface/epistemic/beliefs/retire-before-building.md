---
type: belief
id: retire-before-building
persona: architect
facets: [architecture, refactor, naming, process]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-26
revised: 2026-03-26
---

# retire-before-building

Retire dead abstractions before building new ones in the same namespace. When dead code occupies a concept — a name, directory, interface, or protocol slot — new work that needs that concept must fight the ghost or build around it. The cost of removing dead code is always lower than the cost of building on top of it.

## Statement

Retire dead abstractions before building new ones in the same namespace. When dead code occupies a concept — a name, directory, interface, or protocol slot — new work that needs that concept must fight the ghost or build around it. The cost of removing dead code is always lower than the cost of building on top of it.

## Evidence

- [[session-20260326-165149-931909000]]: The `command` child kind (dead one-shot CLI plugin) occupied the `wit/command/` namespace and the word "command" in the codebase. The [[child-command-surface]] spec needed "command" to mean knowledge-children owning CLI command surfaces. Build agents and humans confused the two. [[scaffold-world-retirement]] had to land first to clear the namespace. (weight: 0.95)
- [[session-20260326-165149-931909000]]: Build agent audit found [[child-command-surface]] SPEC.md pointing at `wit/command/` (the dead world) for its new WIT contract. Cross-spec drift was invisible until explicitly checked. (weight: 0.90)

## Supports

- [[investigate-before-delete]] — investigate first, then delete confidently; this belief adds: delete *before* building, not alongside or after
- [[stale-context-is-hostile-context]] — dead abstractions are a form of stale context that actively misdirects

## Attacks

<!-- none yet -->

## Attacked-By

- Pragmatic concern: "what if removing it breaks something we didn't know about?" Mitigated by: investigate-before-delete + verification gates. If it's truly dead (zero callers), removal risk is near zero.

## Applied-In

- [[scaffold-world-retirement]] — retired `command` and `task` child kinds before [[child-command-surface]] could proceed
- [[child-command-surface]] SPEC.md — `blocked_by: scaffold-world-retirement` encodes this sequencing explicitly

## Revision Log

- 2026-03-26: Created in [[session-20260326-165149-931909000]] — discovered through naming collision between dead `command` kind and new command-surface spec.
