# Design: monty-patina sandbox alignment

## Intent

Translate Monty learnings into Patina-native architecture decisions without
breaking Mother authority, child portability, or toy-grant governance.

## Design constraints

1. Preserve canon boundaries: Mother authorizes, children compute, toys mediate.
2. Do not introduce hidden capability channels outside `[needs].toys`.
3. Keep WASI/component-model portability as a first-class constraint.
4. Avoid scope creep while active spec work is ongoing.

## Comparison model

Use a 5-axis decision frame for each potential Monty-inspired adoption:

1. Security boundary clarity
2. Capability explicitness/auditability
3. Runtime latency impact
4. Portability impact
5. Operational complexity for users/developers

Any adoption that improves latency but weakens explicit grants or authority
boundaries is rejected.

## Candidate adoption tracks

### Track A: execution checkpoints

Goal: add explicit suspend/resume semantics for long-latency child actions.

- Shape: Mother-managed checkpoint API at a child-action boundary.
- Benefit: better reliability for long-running tool/IO operations.
- Risk: state model complexity and replay/idempotency pitfalls.

### Track B: contract feedback loop

Goal: tighten child execution feedback (type/shape/contract errors) so agents can
correct quickly, similar to Monty's iteration strengths.

- Shape: structured error envelopes for toy and child action failures.
- Benefit: fewer blind retries, better agent correction loops.
- Risk: overfitting to one agent style if envelope design is too narrow.

### Track C: positioning + threat model clarity

Goal: explicitly document how Patina differs from "typical sandbox" products and
where it is stronger/weaker than interpreter-first systems.

- Shape: architecture note + threat-model matrix for docs/spec references.
- Benefit: sharper product messaging and better design tradeoff consistency.
- Risk: none significant; mainly doc quality risk.

## Rejected directions (for this phase)

- Embedding a Python interpreter as Patina's primary child runtime.
- Introducing ad hoc host callbacks that bypass toy manifests.
- Expanding scope into implementation before open spec commitments are done.

## Deliverables for exploration completion

1. Primitive mapping table (Monty -> Patina + non-equivalences).
2. Decision memo: adopt/adapt/defer for each candidate track.
3. Sequenced follow-on feat spec proposals with dependency gates.

## Promotion criteria

Promote this explore to feat only when:

- Active open-spec priorities are explicitly settled in spec workflow.
- At least one Track (A/B/C) has a narrow first slice with measurable proof.
- Security and authority invariants are written as non-negotiable acceptance
  gates in the promoted feat.

## Verification

```bash
patina spec check monty-patina-sandbox-alignment --json
patina spec next
```
