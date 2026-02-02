---
type: belief
id: practical-memory-over-epistemic-formalism
persona: architect
facets: [epistemic, architecture, design-philosophy]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-02
revised: 2026-02-02
---

# practical-memory-over-epistemic-formalism

The belief system is a decision memory with regression tests, not an epistemic knowledge graph — value comes from mechanical verification against the codebase, not from graph traversal or belief revision algorithms.

## Statement

The belief system is a decision memory with regression tests, not an epistemic knowledge graph — value comes from mechanical verification against the codebase, not from graph traversal or belief revision algorithms.

## Evidence

- [[session-20260202-063713]]: Andrew Ng review showed 47 verification queries catching real drift (archive-completed-work found 4 stale specs), while supports/attacks/entrenchment fields are metadata for human reading, not computation. No code path changes behavior based on graph relationships. (weight: 0.9)
- [[session-20260202-063713]]: Helland inside/outside boundary is the core architectural insight — beliefs are outside data (committed positions), verification results are inside data (derived, recomputable). This is practical data architecture, not epistemic theory. (weight: 0.8)
- [[session-20260202-063713]]: System closest analogy is living ADRs (architecture decision records) with automated regression tests, not a knowledge graph or Bayesian network. (weight: 0.7)

## Supports

- [[measure-first]] — verification queries are measurement, not formalism
- [[ground-before-reasoning]] — mechanical checks ground reasoning in code reality

## Attacks

## Attacked-By

## Applied-In

- Verification system design: chose SQL/assay/temporal queries over graph traversal algorithms
- Belief audit display: shows verification pass/fail (practical) alongside entrenchment (decorative)
- The supports/attacks sections in this very file — metadata for humans, not traversed by code

## Revision Log

- 2026-02-02: Created — metrics computed by `patina scrape`
