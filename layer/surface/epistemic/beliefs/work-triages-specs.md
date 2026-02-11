---
type: belief
id: work-triages-specs
persona: architect
facets: [governance, specs, process, pragmatism]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-11
revised: 2026-02-11
---

# work-triages-specs

Instead of spending sessions categorizing and prioritizing specs, let the actual build determine what matters — frozen specs are a reference library that the build consumes or archives.

## Statement

Instead of spending sessions categorizing and prioritizing specs, let the actual build determine what matters — frozen specs are a reference library that the build consumes or archives.

## Evidence

- [[session-20260211-121154]]: 32 specs accumulated across months of work. Attempting to triage them (categorize, prioritize, decide keep/archive) was itself a spec-shaped problem with no clear exit criteria. Freezing all specs and letting the plugin system build selectively consume them is more honest — the work determines what's valuable, not a planning session. (weight: 0.9)

## Reasoning

Spec triage is itself an unbounded task — "decide the priority of 30 things" has no clear exit criteria and produces decisions that go stale immediately. The alternative: freeze everything, start building the next concrete thing (plugin system), and let the build pull from frozen specs as needed. Specs that get consumed were valuable. Specs that remain untouched after the build are noise — archive them. The work is the triage.

This is the [[unix-philosophy]] applied to governance: don't build a system to manage specs, let the work flow determine what matters.

## Supports

- [[spec-driven-design]] — specs authorize action, but accumulated specs without action are governance debt, not governance
- [[transparent-complexity]] — 32 specs is invisible complexity in the governance layer; freezing makes the problem visible

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

- Planning has value: some upfront triage prevents building the wrong thing. Freezing everything risks losing context that only exists in human memory right now.
  - Status: acknowledged — mitigated by keeping specs readable (not archived behind git tags) so the build can reference them. The freeze is "pause," not "delete."

## Applied-In

- [[session-20260211-121154]]: Hard freeze of all 29 specs to design status ([[commit-6113de2e]]). Plugin system build will selectively consume from frozen specs. "explore" type to be removed after plugin system ships.

## Revision Log

- 2026-02-11: Created — metrics computed by `patina scrape`
