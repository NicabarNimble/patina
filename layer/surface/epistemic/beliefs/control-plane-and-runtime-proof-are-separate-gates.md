---
type: belief
id: control-plane-and-runtime-proof-are-separate-gates
persona: architect
facets: [control-plane, runtime, verification, fail-closed]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-29
revised: 2026-04-29
---

# control-plane-and-runtime-proof-are-separate-gates

Treat control-plane success and runtime load success as separate gates; fail closed until both are proven.

## Statement

Treat control-plane success and runtime load success as separate gates; fail closed until both are proven.

## Evidence

- [[session-20260428-230202-450986000]]: External Slate proof for [[child-registry-control-plane-remaining]] validated source sync/approve/install/assign via Mother control-plane while routed execute failed closed on runtime load ("failed to parse WebAssembly module"), demonstrating the two-gate requirement. Related commits: [[commit-fda637be]], [[commit-a8c2f090]], [[commit-59582e47]].

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-04-29: Created — metrics computed by `patina scrape`
