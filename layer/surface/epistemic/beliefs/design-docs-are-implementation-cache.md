---
type: belief
id: design-docs-are-implementation-cache
persona: architect
facets: [workflow, specs, productivity]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-23
revised: 2026-02-23
---

# design-docs-are-implementation-cache

Implementation design docs that cache function signatures, side effects, and step order eliminate re-analysis time — invest in them for specs requiring more than one session of work.

## Statement

Implementation design docs that cache function signatures, side effects, and step order eliminate re-analysis time — invest in them for specs requiring more than one session of work.

## Evidence

- [[session-20260223-162443]]: Phase 1 of [[spec-workflow-rigor]] implemented all 14 steps from `design.md` in ~20 minutes with zero design time. Every function signature, git tag convention, and side effect was pre-decided in the audit session ([[session-20260223-152707]]). (weight: 0.95)
- [[session-20260223-152707]]: Produced `design.md` by reading all 7 source files — gap analysis, 14-step implementation order, state transition matrix, refactoring plan. Investment: ~45 minutes of reading and writing. Payoff: entire implementation session had no design decisions to make. (weight: 0.9)
- "Evaluating AGENTS.md" (Gloaguen et al., ETH Zurich, Feb 2026): Context files work when they contain rules the LLM can't discover by reading code. Design docs are exactly this — they're decisions, not descriptions. (weight: 0.6)

## Supports

- [[spec-is-a-directory]]: Design docs are one of the three files in the spec directory (SPEC.md = why this shape, design.md = how to build it, walkthroughs.md = UX contract). Each loaded by different consumers at different times.
- [[spec-first]]: Design before implement. The design doc is the concrete manifestation of this principle for multi-session specs.

## Attacks

<!-- None identified -->

## Attacked-By

- **Overhead for small specs**: A design doc is waste if the spec fits in a single session and the LLM can discover the implementation path by reading code. The threshold is "more than one session" — below that, the design doc costs more than it saves.
- [[context-files-are-rules-not-docs]]: If the design doc becomes stale or describes things the LLM can discover, it becomes the kind of documentation the ETH paper warns against. Design docs must be decisions, not descriptions.

## Applied-In

- `layer/surface/build/feat/spec-workflow-rigor/design.md` — First application. Produced by audit session, consumed by implementation session. 14 steps executed in order, zero re-analysis.

## Revision Log

- 2026-02-23: Created — metrics computed by `patina scrape`
