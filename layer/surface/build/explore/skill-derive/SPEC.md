---
type: explore
id: skill-derive
status: abandoned
created: 2026-01-20
updated: 2026-02-07
blocked_by:
- skills-focused-adapter
related:
- layer/surface/build/feat/skills-focused-adapter/SPEC.md
- layer/surface/build/refactor/skill-enforcement/SPEC.md
---

# Explore: Belief-Driven Skill Generation

> A skill is a belief made executable. Can we auto-generate skills from high-confidence beliefs?

## Core Insight

| Belief Property | Skill Property |
|-----------------|----------------|
| Statement | Instruction content |
| Evidence | Reference docs (provenance) |
| Confidence | Whether to generate at all |
| Facets | When to activate (routing) |
| Supports/Attacks | Dependencies / conflicts |

The value lives in Patina (beliefs, patterns, knowledge). Skills are delivery mechanisms that inject that value into LLM adapters.

## The Question

`patina skill derive` would:
1. Filter beliefs by confidence threshold (>= 0.80)
2. Select template based on belief type (cli-wrapper, workflow, guard)
3. Render adapter-specific skill output
4. Every generated skill traces back to its source belief(s)

## Open Questions

- What's the minimum confidence for a belief to earn a skill?
- Which beliefs are actually skill-derivable? (Not all knowledge is actionable)
- Should skill generation be automatic (on belief change) or manual (`patina skill derive`)?
- How does skill success/failure feed back into belief confidence?
- Does this need skills-focused-adapter to ship first, or can it work with current adapter structure?

## Candidate Beliefs

| Belief | CLI Mapping | Skill Purpose |
|--------|-------------|---------------|
| `session-git-integration` | `patina session *` | Session management |
| `progressive-disclosure` | `patina context *` | Context loading |
| `spec-first` | `patina spec *` | Spec management |
| `eventlog-is-truth` | `patina scry *` | Knowledge retrieval |

## Future Extensions

- **Skill composition**: beliefs with `supports` → compound skills
- **Conditional skills**: beliefs with `attacked-by` → context-aware warnings
- **Cross-project skills**: mother beliefs → skills that work everywhere
- **Skill evolution**: belief revision (AGM) → auto-regenerate affected skills

## References

- layer/surface/epistemic/_index.md — 68 beliefs, potential skill sources
- feat/skills-focused-adapter — skills infrastructure (build this first)
- Session 20260120-165543 — origin of belief→skill insight
