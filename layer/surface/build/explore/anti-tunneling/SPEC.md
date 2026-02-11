---
type: explore
id: anti-tunneling
status: abandoned
created: 2026-02-09
updated: 2026-02-09
sessions:
  origin: 20260209-061005
related:
- layer/surface/build/explore/belief-validation-system/SPEC.md
- layer/surface/build/explore/anti-slop/SPEC.md
beliefs:
- anti-tunneling-as-belief-challenge
- measure-the-measurement
- practical-memory-over-epistemic-formalism
---

# explore: Anti-Tunneling as Belief Challenge Lens

> Beliefs should bubble up with minimal resistance. The system proves and challenges them through their existence. Tunneling risk is a computed dimension, not a creation gate.

## Problem

Patina's belief system has challenge mechanisms for **truth** (verification queries) and **connectedness** (grounding scores), but nothing that challenges whether a belief is **solving the wrong problem**. A belief can be true, well-grounded, and still be tunneling — building infrastructure for a mountain that doesn't need to exist.

Current challenge dimensions:

| Dimension | What it catches | Gap |
|-----------|----------------|-----|
| Verification queries | Structural claims contradicted by code | Only structural beliefs |
| Grounding score | Beliefs disconnected from codebase | Doesn't question necessity |
| Audit warnings | No evidence, unused, floating | Flags absence, not wrongness |
| Attacked-By | Counter-arguments | Manual, rarely populated |
| **Tunneling risk** | **Belief solving wrong problem** | **Missing — this spec** |

## Origin

Cross-project analysis of [marcus/sidecar](https://github.com/marcus/sidecar.git) via `patina repo add` revealed its task-context-prompt pipeline. Discussion of what patina could learn led to the Anti-Tunneling Playbook — a set of techniques for preventing LLMs from solving the wrong problem. The key insight: these techniques shouldn't gate belief creation, but should be a **diagnostic lens** the system applies to beliefs as they mature.

## The Anti-Tunneling Playbook (Source Material)

Five techniques that detect when you're "tunneling through a mountain that doesn't need to exist":

1. **Reframe Gate** — Force restatement of real goal, split must-have vs assumed constraints, propose alternatives
2. **"3 Reasons This Might Be Wrong"** — Argue against the framing before helping
3. **Assumption Ledger** — Turn hidden "truths" into testable statements
4. **Premortem + Escape Hatch** — Name the swap boundary or you're building a mountain
5. **Two-pass Red Team / Build** — Separate critique from construction

### The Smell Test

Quick signals that tunneling is happening:
- **Idle work**: Does the system do work when nothing changes?
- **Artificial constraints**: Why do we need this constraint?
- **Mountain language**: "optimize", "cache", "memoize", "reconcile", "ensure", "always"
- **Comfort tools**: Chose X because we know X, not because the problem fits

## Proposed: Tunneling Risk as Audit Dimension

### Computable Signals

| Signal | Detection Method | Feasibility |
|--------|-----------------|-------------|
| **Mountain language** | Regex/keyword scan of belief statement | Easy — word list match |
| **No defeated attacks** | Check Attacked-By section for defeated entries | Easy — already parsed |
| **Complexity growth** | Applied-In entries growing in word count over time | Medium — needs temporal tracking |
| **Assumed constraints unidentified** | No constraint decomposition in evidence | Medium — heuristic on evidence structure |
| **No alternatives considered** | No Attacks section (never challenged) | Easy — section emptiness check |
| **Comfort tool pattern** | Belief recommends specific tool without problem-shape justification | Hard — semantic analysis |

### Proposed Audit Output

```
patina belief audit --tunneling

BELIEF                    TUNNEL-RISK   SIGNALS
sync-first                low           2 defeated attacks, removes complexity
always-validate-input     medium        mountain language ("always"), 0 attacks
cache-embeddings          high          3 applied-in growing complex, comfort-tool, 0 attacks
ensure-type-safety        medium        mountain language ("ensure"), but 3 defeated attacks offset
```

### Risk Levels

| Level | Criteria | Remediation |
|-------|----------|-------------|
| **low** | Has defeated attacks AND no mountain language AND alternatives considered | None needed |
| **medium** | 1-2 signals present | Suggest: "Consider 3 reasons this might be wrong" |
| **high** | 3+ signals, no defeated attacks, mountain language | Suggest: Reframe Gate — split must-have vs assumed constraints |

### Remediation is Suggestion, Not Enforcement

When tunnel-risk is high, the system suggests playbook techniques — it doesn't block or downgrade the belief. The human decides whether the belief is genuinely tunneling or just hasn't been challenged yet.

Suggested remediation prompts (surfaced in audit):
- "This belief has high tunnel risk. Consider: what 3 reasons might it be the wrong problem?"
- "No alternatives have been considered. What would you lose if you removed this belief?"
- "The Attacked-By section is empty. What's the strongest argument against this?"

## What We Already Have

| Infrastructure | How It Helps |
|----------------|-------------|
| `patina belief audit` | Already surfaces warnings — tunneling risk is another column |
| Belief parsing in scraper | Already extracts sections (Evidence, Attacked-By, Applied-In) |
| Mountain language detection | Trivial regex on statement text |
| Attacked-By parsing | Already exists — just need to check for "defeated" status |
| `belief_fts` table | Could search for comfort-tool patterns |

## What Needs to Be Built

| Component | Effort | Description |
|-----------|--------|-------------|
| Tunneling signal detection | ~100 lines | Keyword scan + section emptiness checks |
| Risk level computation | ~50 lines | Aggregate signals into low/medium/high |
| Audit integration | ~50 lines | Add `--tunneling` flag or integrate into default audit |
| Remediation suggestions | ~30 lines | Map risk level to playbook technique |
| **Total** | **~230 lines** | |

## Open Questions

1. Should tunneling risk be stored in the `beliefs` table or computed on-the-fly during audit?
2. How do we distinguish "unchallenged" (no attacks yet) from "unchallengeable" (so obviously true no one argues)?
3. Should mountain language detection be configurable (project-specific word lists)?
4. Could tunneling analysis apply to **specs** too, not just beliefs? (Specs that keep accumulating phases might be tunneling.)
5. How does this relate to the belief-validation-system explore spec? Is it a sub-feature or parallel?

## Relationship to Other Specs

- **belief-validation-system**: Focuses on computed confidence from verifiable data. Anti-tunneling is orthogonal — a belief can be high-confidence AND tunneling.
- **anti-slop**: Signal-over-noise for contributions. Anti-tunneling is signal-over-noise for beliefs themselves.
- **specs-as-context-sources**: Tunneling analysis could surface in the spec context pipeline as a warning when starting work on a spec grounded in high-tunnel-risk beliefs.

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-09 | design | Initial exploration from sidecar cross-project analysis and anti-tunneling playbook discussion |
