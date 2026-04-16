---
type: belief
id: partial-completion-is-not-done
persona: architect
facets: [process, completion, acceptance-criteria]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-04-10
revised: 2026-04-10
---

# partial-completion-is-not-done

Acceptance criteria define done — 8 of 11 is not done, it is a different state; either finish the work or revise the criteria, but do not declare partial completion as complete.

## Statement

Acceptance criteria define done — 8 of 11 is not done, it is a different state; either finish the work or revise the criteria, but do not declare partial completion as complete.

## Evidence

- [[session-20260409-143847-707078000]] - A spec was reported as effectively-done with 8 of 11 acceptance criteria met; the user explicitly rejected this framing and required completing all 11 before archive, which surfaced real gaps in test coverage and metrics that would have rotted into technical debt (weight: 0.95)
- [[commit-fb570cab]] - The first of three follow-up commits required to close the partial-completion gap; added missing test coverage that would not have existed if the work had been declared done at 8 of 11
- [[commit-1cb90e39]] - Closed an explicit assertion gap in an end-to-end test; the work was reported as covered before this fix, but the assertion was implicit
- [[commit-87258f33]] - Added a metrics baseline that was a stated criterion but had no implementation; partial completion would have left this as silent debt

## Supports

- [[spec-driven-design]] — specs authorize action by listing exit criteria; declaring incomplete work as complete undermines the contract that specs hold
- [[specs-are-actionable-beliefs]] — if a spec is an actionable belief, partial completion is a partial belief, not the same belief
- [[ground-every-assertion]] — criteria are the assertions; meeting them is the grounding

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- A spec criterion that turns out to be wrong or unmeetable. The right response is to revise the spec, not to declare the spec done with the criterion unmet. Attack defeated by the rule "either finish or revise."

## Applied-In

- Three follow-up commits were required to bring a spec from 8 of 11 acceptance criteria to 11 of 11 before it could be marked complete and archived. The gaps were real and would have become silent technical debt if accepted as done.

## Revision Log

- 2026-04-10: Created — metrics computed by `patina scrape`
