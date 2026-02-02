---
type: belief
id: stale-context-is-hostile-context
persona: architect
facets: [epistemic, specs, context-management, patina-core]
entrenchment: high
status: active
endorsed: true
extracted: 2026-02-02
revised: 2026-02-02
---

# stale-context-is-hostile-context

Stale specs and outdated documentation are worse than no documentation for LLM collaboration — an LLM trusts what it reads, so obsolete context actively misdirects rather than merely failing to inform.

## Statement

Patina exists at the boundary between human non-linear thinking and LLM zero-persistence memory. The human builds and learns across many parallel timelines — direction changes organically as understanding deepens. The LLM reboots every session with no memory, reading whatever context exists to reconstruct understanding. This asymmetry means stale context is not merely unhelpful — it is hostile. A spec that says "E4.5 (exploring)" when E4.5 is complete causes the next LLM to plan work that's already done, miss work that emerged during building, and misunderstand the project's actual state.

The failure mode is not "docs are out of date" — it's that **the symbiotic relationship between human and LLM breaks down when the context layer lies**. Specs become frozen snapshots of past intent while sessions capture what actually happened. The LLM reads the spec, trusts it, and reasons from false premises. The human can't maintain all specs linearly because they think non-linearly across many timelines. No tool currently detects or surfaces this decay.

This is Patina's core challenge: being the accurate context reboot layer. Every stale spec, every outdated statistics section, every unchecked exit criterion that's actually met — these are bugs in Patina's primary mission. Verification queries detect belief drift against code. We need an equivalent mechanism for spec drift against reality.

## Evidence

- [[session-20260202-093028]]: Epistemic-layer SPEC had E4.5 marked as "exploring" when it was actually complete with its own 28-step spec, 47 verification queries, and archived via git tag. Build.md showed 35 beliefs when 46 exist. Patch milestones were 2 versions behind. All discovered during E4.5 close-out review. (weight: 0.9)
- [[session-20260202-093028]]: User articulated the core insight — "you can hold many timelines and I cannot, but you need the organization to resurface those timelines as your memory completely goes away. Most of what Patina is to become is this symbiotic relationship — the layer of understanding guided by a user for LLMs to use as a reboot of context." (weight: 1.0)
- [[session-20260202-063713]]: Previous session updated 47 verification queries but did not update the parent epistemic-layer SPEC's E4.5 section — the child spec diverged from the parent, and the parent became a lie. (weight: 0.8)

## Supports

- [[spec-carries-progress]] — specs must track progress, but this belief goes further: tracking isn't enough if no one detects when tracking stops
- [[ground-before-reasoning]] — LLMs must read reality before reasoning, but stale specs present false reality
- [[practical-memory-over-epistemic-formalism]] — the system is decision memory; stale memory is corrupted memory

## Attacks

- [[specs-source-of-truth]] (status: scoped, reason: "specs are source of truth for *intent*, but they decay as source of truth for *state* when building-and-learning changes direction. The source of truth for state is git + sessions + DB, not the spec.")

## Attacked-By

## Applied-In

- Epistemic-layer SPEC sync (2026-02-02): compressed E4.5 from 150 lines of stale design context to 20-line outcome summary pointing to archived spec
- Build.md refresh (2026-02-02): updated milestones, belief counts, phase status from stale Jan-29 snapshot to current Feb-02 reality
- Belief verification close-out: found and fixed 5+ stale references across build.md and epistemic-layer SPEC

## Revision Log

- 2026-02-02: Created — the spec drift discovered during E4.5 close-out revealed Patina's core context-reboot challenge
