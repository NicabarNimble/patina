---
type: belief
id: test-gates-against-canon
status: active
confidence: high
entrenchment: medium
facets: [process, architecture, spec-driven-design]
session_origin: session-20260331-224232-852361000
created: 2026-04-01
---
# Test Gates Against Canon

Before committing to a fix direction for any spec gate, test it against the architectural canon. Code that looks wrong in isolation may be correct when viewed from the system's intended direction.

## Evidence

In [[session-20260331-224232-852361000]], reading [[child-construction-canon]] before editing the audit remediation specs changed the disposition of 4 out of 30 gates:
- A25 (spec lib inversion): would have "fixed" an intentional daemon-first dispatch pattern. Dropped.
- A6 (session frontmatter): would have forced `String` where `Option<String>` is correct for 538 permanent pre-UID session artifacts. Reversed.
- A22 (blanket dead_code): would have deleted toy host functions that are alive in the toybox but unused by current children. Reframed to three-category audit.
- A3 (capability divergence): would have deleted as simple dedup. Elevated to security-critical per [[children-have-agency-toys-are-capabilities]] — the capability check is the gate for the entire child/toy system.

Without the canon read, 4 gates would have been executed in wrong directions. The cost of reading was 10 minutes. The cost of wrong execution would have been broken daemon dispatch, broken old session parsing, deleted future toy surface, and weakened security boundary.

## Test

Before executing a fix gate: can you state which architectural canon (spec, belief, or core value) this gate serves? If not, read the canon first. If the gate contradicts the canon, the gate is wrong, not the code.

## Connects

- [[child-construction-canon]] — the primary architectural canon for Mother/child/toy system
- [[audit-before-refactor]] — read code before changing it; this belief extends that to read architecture before changing code
- [[spec-driven-design]] — specs decide, code executes; the canon is the spec that decides direction
- [[five-boundaries-no-overlap]] — the five roles define which canon applies to which code
