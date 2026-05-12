---
type: belief
id: mother-brokers-child-skills
persona: patina
facets: [mother, skills, children, agent-instructions]
entrenchment: medium
status: active
endorsed: true
extracted: 2026-05-12
revised: 2026-05-12
---

# mother-brokers-child-skills

Mother should broker child skills: AGENTS.md should route only to the overarching Patina/Mother skill system, while child-specific workflows live in Mother-discoverable child skill packages exposed by active children.

## Statement

Mother should broker child skills: AGENTS.md should route only to the overarching Patina/Mother skill system, while child-specific workflows live in Mother-discoverable child skill packages exposed by active children.

## Evidence

- [[session-20260508-144836-859149000]]: User approved this as belief-worthy after rejecting child-specific policy in `AGENTS.md` and asking for Mother to answer which active child skills are available.
- `AGENTS.md`: now points only to the overarching `.pi/skills/patina-mother-system/SKILL.md` skill-system entrypoint instead of embedding Slate-specific workflow policy.
- `.pi/skills/patina-mother-system/SKILL.md`: documents the intended Mother-owned child skill broker and future `patina mother skills ...` help/discovery shape.
- `layer/slate/work/patina-mother-skill-routing/work.toml`: completed Slate work capturing this routing correction and proof trail.

## Supports

- [[control-plane-authority-distributed-execution]]

## Attacks

- Embedding per-child operational policy directly in `AGENTS.md` as durable project instruction.

## Applied-In

- `AGENTS.md`
- `.pi/skills/patina-mother-system/SKILL.md`
- `.pi/skills/patina-slate-code/SKILL.md`

## Attacked-By

<!-- Add beliefs that challenge this -->

## Revision Log

- 2026-05-12: Created — metrics computed by `patina scrape`
