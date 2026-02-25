---
type: belief
id: specs-orthogonal-to-sessions
persona: architect
facets: [architecture, spec-system, session-system, workflow]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-02-23
revised: 2026-02-23
---

# specs-orthogonal-to-sessions

Specs and sessions are orthogonal dimensions — specs are the unit of work, sessions are the unit of time. A session might touch 3 specs. A spec might take 5 sessions. They live independently but must respect each other's boundaries.

## Statement

Specs and sessions are orthogonal dimensions — specs are the unit of work, sessions are the unit of time. A session might touch 3 specs. A spec might take 5 sessions. Specs live outside sessions but must respect session boundaries (commit before end).

## Evidence

- [[session-20260223-120524]]: During spec-workflow-rigor analysis, discovered that the natural workflow is: working on spec A → discover bug → pause spec A → create fix spec B → either fix B now or defer → resume A. This flow crosses session boundaries — the fix spec might be completed in a different session than it was created. Sessions are temporal containers; specs are work containers. Conflating them loses the ability to track work across time. (weight: 0.85)

## Supports

- [[active-is-a-black-hole]] — orthogonality means specs need their own lifecycle independent of sessions
- [[spec-first]] — specs as the primary unit of work, not sessions

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

- [[spec-workflow-rigor]] — pause/resume/block/split operate on specs independent of session state. A spec can be paused in one session and resumed in another.
- Session system — sessions track time, git metrics, and LLM conversation. They don't own spec lifecycle.

## Revision Log

- 2026-02-23: Created — emerged from code analysis session discovering natural pause→create→fix→resume workflow
