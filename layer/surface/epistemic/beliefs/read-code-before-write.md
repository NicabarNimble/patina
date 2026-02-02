---
type: belief
id: read-code-before-write
persona: architect
facets: [development-process, design, code-quality]
confidence:
  score: 0.88
entrenchment: medium
status: active
extracted: 2026-01-22
revised: 2026-01-22
---

# read-code-before-write

Always read existing code before writing new code. Understanding existing patterns prevents architectural mistakes and reduces cognitive load.

## Statement

Always read existing code before writing new code. Understanding existing patterns prevents architectural mistakes and reduces cognitive load.

## Evidence

- [[session-20260122-220957]] - E3 implementation followed ID offset pattern after reading oxidize/enrichment code (weight: 0.95)
- [[session-20260122-154954]] - Mother-naming refactor read 50+ files before surgical edits (weight: 0.90)
- [[session-20260122-220957]]: [[CLAUDE.md]] instructs: "NEVER propose changes to code you haven't read" (weight: 0.85)

## Supports

- [[dependable-rust]] - Understanding existing interfaces before extending
- [[spec-first]] - Research before implementation
- [[measure-first]] - Understand baseline before changing

## Attacks

- [[move-fast-break-things]] (status: defeated, reason: speed without understanding causes rework)
- [[just-ask-llm]] (status: scoped, reason: LLM suggestions need validation against existing patterns)

## Attacked-By

- [[time-pressure]] (status: active, confidence: 0.3, scope: "urgent hotfixes may skip deep reading")

## Applied-In

- E3 belief indexing: Read scrape/layer, oxidize, scry/enrichment before adding beliefs
- Mother-naming refactor: Read all mothership references before renaming
- Pattern: `git grep` + read files before proposing changes

## Revision Log

- 2026-01-22: Created from E3 implementation session (confidence: 0.88)
