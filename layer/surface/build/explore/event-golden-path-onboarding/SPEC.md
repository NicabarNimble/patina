---
type: explore
id: event-golden-path-onboarding
status: draft
created: 2026-04-14
sessions:
  origin: 20260413-075041-892082000
exit_criteria: []
---
# explore: Explore: Event-in -> Typed-action -> Event-out onboarding path

> Find the smallest teachable path that makes Patina understandable in 5 minutes for new contributors.

## Question

How do we package Patina’s existing typed/runtime power into a 5-minute onboarding path that new builders can actually complete?

Sub-questions:
- Can we teach the system using only three concepts: **event in → typed action → event out**?
- What is the minimum runnable artifact that proves this loop end-to-end?
- Which parts of current Patina surface are essential vs. overwhelming for first contact?

## Findings

Initial observations (from current architecture + recent slices):
- Patina already has the hard primitives: typed child calls, policy enforcement, observability history, and reusable children.
- New users struggle less with capability and more with **surface complexity** (many commands, many folders, many concepts at once).
- The workshop-style framing (append/stream/reduce) is conceptually sticky and can be mapped directly onto Mother typed-call flows.
- Cross-project spec routing improves operator ergonomics for multi-repo work, but onboarding still needs a single “golden path” runnable.

Planned experiment slices:
1. **Golden-path example** in `rivet-deno-lab` with one request event, one Mother typed call, one response event.
2. **Operator quickstart** with ≤5 commands and one expected success payload.
3. **Inspector proof** that shows correlation in Mother typed-call history so users see internal state, not just output.

## Conclusions

Working conclusion:
- Patina does not need new core primitives for onboarding; it needs a tighter first-run narrative and artifact.

Proposed output of this explore:
- A “golden path kit” that includes:
  - one minimal event schema,
  - one typed call bridge implementation,
  - one copy-paste quickstart,
  - one troubleshooting block keyed to observable Mother data.

Success signal:
- A new contributor can run the flow and explain “what happened” in under 5 minutes without reading architecture docs first.
