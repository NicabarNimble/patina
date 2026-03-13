---
type: explore
id: interface-skill-architecture
status: draft
created: 2026-03-12
sessions:
  origin: 20260312-001728
related:
- src/adapters/templates.rs
- resources/opencode
- resources/claude
- resources/gemini
- layer/surface/build/refactor/cli-first-spec-workflow/SPEC.md
exit_criteria: []
---
# explore: Interface Skill Architecture

> Explore how skills should be designed per interface so the mandatory thin surface, runtime-specific behavior, and personality live in interface-owned skills while deterministic project truth stays in Patina.

## Question

What should belong inside interface-owned skills, and what should remain
inside Patina's deterministic CLI/backend contract, now that Claude,
OpenCode, Gemini, and future interfaces are clearly diverging in UI
behavior?

Sub-questions:

- Which skills are truly interface-specific versus just thin wrappers
  over Patina commands?
- How should personality, runtime quirks, and command packaging live in
  skills without letting skills become the source of project truth?
- How should Patina expose deterministic workflows so every interface can
  consume them differently without redefining them?
- How do beliefs and sessions fit into this model, given that they are
  the most obviously interface-sensitive skill surfaces today?

## Findings

This exploration should examine at least these current signals:

- `mother-owned-interface-bundles` and related interface-bundle work
- `patina-ai-interface-layer`
- `session-narrative-system`
- `opencode-session-spec-capabilities`
- the recent decision to treat interfaces as distinct UI/runtime
  surfaces rather than forcing one shared authored skill layer
- the newer CLI-first spec direction

Expected areas to map:

- **Patina-owned deterministic core**
  - CLI commands
  - session lifecycle backends
  - belief creation backends
  - spec workflow
  - Mother/child/toy backend contracts

- **Interface-owned skill layer**
  - personality/tone
  - slash-command packaging
  - runtime-specific context gathering
  - compensations for host limitations
  - thin orchestration around Patina commands

- **Likely special cases**
  - sessions
  - beliefs
  - maybe selected review/setup flows where runtime behavior materially
    changes what can be captured or shown

## Conclusions

This exploration should conclude with:

- a clear boundary between Patina-owned deterministic workflows and
  interface-owned skill behavior
- a statement of which skills should remain interface-specific
- a statement of which areas should become CLI-first and shared
- guidance for how future interface bundles should evolve without
  recreating divergent backend logic
- follow-up recommendations, likely feeding:
  - `cli-first-spec-workflow`
  - future belief/session skill cleanup
  - interface bundle separation work
