---
type: explore
id: three-layers
status: design
created: 2026-01-13
updated: 2026-02-06
tags: [architecture, authority, convergence]
related:
  - layer/surface/build/feat/mother/SPEC.md
  - layer/surface/build/feat/doctor-dev/SPEC.md
---

# Explore: Three-Layer Authority Model

> Authority to declare completion must live outside the model.

## Responsibility Map

| Layer | Need | Focus | Location |
|-------|------|-------|----------|
| **mother** | Infrastructure | Identity, secrets, coordination, daemon | `~/.patina/` |
| **patina** | Product | Knowledge extraction and retrieval (RAG) | `.patina/` |
| **awaken** | Shipping | Build, deploy, make it run | containers/prod |

This separation already exists in the code. The open question is whether it formalizes into separate binaries or stays as internal boundaries.

## The Authority Insight

Models operating inside patina can claim "done" — but model self-report is unreliable. Local plausibility masquerades as task completion.

**Principle:** Models propose, systems verify. "Done" requires mechanical validation, not assertion.

| Layer | Authority Role |
|-------|----------------|
| **mother** | Cross-project verification, CI gates, quality thresholds |
| **patina** | Provides facts for verification, NOT completion authority |
| **awaken** | Shipping gates — nothing deploys without passing invariants |

**The diagnostic question:** "If an agent claims this task is done, what mechanically happens next?"

- Human review → pre-convergence design
- System verification → convergence-aware design
- Forced continuation until invariants pass → convergence-first design

Patina should be convergence-first where possible, convergence-aware everywhere else.

## Implications

1. **Specs need machine-checkable exit criteria** — not just prose
2. **Session-end could verify invariants** — forced convergence before archive
3. **Failures become first-class data** — iteration count, failure modes, convergence signals

## Connection to doctor-dev

The "deacon patrol" pattern at session boundaries is the first convergence-aware mechanism. doctor-dev catches spec drift and status contradictions — mechanical verification, not assertion.

## Open Questions

- One binary or three? (Responsibility separation matters more than binary separation)
- Does awaken need to exist at all, or is "shipping" just CI/CD config?
- How do machine-checkable exit criteria interact with `patina spec status complete`?

## References

- feat/doctor-dev — first convergence mechanism (session boundary checks)
- feat/mother — infrastructure layer spec
- layer/core/unix-philosophy.md — single responsibility per layer
