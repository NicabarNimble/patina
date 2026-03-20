---
type: belief
id: session-system-needs-multi-interface-redesign
persona: architect
facets: [architecture, sessions]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-03-18
revised: 2026-03-18
---

# session-system-needs-multi-interface-redesign

The Patina session system was designed around Claude Code's logging gap and does not generalize to multi-interface workflows — patching it is wrong, a redesign is needed

## Statement

The Patina session system was designed around Claude Code's logging gap and does not generalize to multi-interface workflows — patching it is wrong, a redesign is needed

## Evidence

- [[session-20260318-221008-061837000]] - original session system built to fill Claude Code's missing LLM reply logs; OpenCode and Gemini handle context differently; [[spec-session-handoff-enrichment]] is a patch on a broken foundation, not the right path (weight: 0.9)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-03-18: Created — metrics computed by `patina scrape`
