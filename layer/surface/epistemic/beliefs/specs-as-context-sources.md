---
type: belief
id: specs-as-context-sources
persona: architect
facets: [architecture, spec-system, context-pipeline]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-09
revised: 2026-02-09
---

# specs-as-context-sources

Specs should be first-class queryable context sources that participate in a structured context pipeline (spec → extractable state → template variables → agent prompt), not passive documents that LLMs must discover and read manually.

## Statement

Specs should be first-class queryable context sources that participate in a structured context pipeline (spec → extractable state → template variables → agent prompt), not passive documents that LLMs must discover and read manually.

## Evidence

- [[session-20260209-075426]]: [[session-20260209-061005]] - Cross-project analysis of marcus/sidecar revealed its task→context→prompt pipeline (getTaskContext → ExpandPromptTemplate → buildAgentCommand) where every agent session starts with exactly the right context. Patina specs currently lack this structured extraction — they have progress checkboxes but no machine-readable state that feeds into context injection. (weight: 0.9)

## Supports

- [[spec-carries-progress]]: If specs carry progress via checkboxes, the next step is making that progress machine-extractable for context injection
- [[spec-drives-tooling]]: Queryable specs are the generalization — not just version tooling reads from specs, but the entire context pipeline does
- [[progressive-disclosure]]: Structured spec state enables progressive disclosure — metadata (phase, status) always available, details on-demand

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Complexity risk: Adding structured extraction to specs could over-engineer what works as simple markdown. Counter: sidecar's ExpandPromptTemplate is just regex — the extraction can be equally simple.

## Applied-In

- **Reference implementation**: marcus/sidecar's `getTaskContext` → `ExpandPromptTemplate` → `buildAgentCommand` pipeline demonstrates this pattern with 6 agent adapters
- **Gap identified**: `patina context --topic X` returns semantically related content but cannot extract spec phase/exit-criteria/blockers as structured variables

## Revision Log

- 2026-02-09: Created — metrics computed by `patina scrape`
