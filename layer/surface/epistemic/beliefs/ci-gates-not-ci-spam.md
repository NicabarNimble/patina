---
type: belief
id: ci-gates-not-ci-spam
persona: architect
facets: [ci, process, github-actions]
confidence:
  score: 0.90
  signals:
    evidence: 0.95
    source_reliability: 0.90
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: medium
status: active
extracted: 2026-01-27
revised: 2026-01-27
---

# ci-gates-not-ci-spam

CI workflows should either do meaningful work or not trigger at all. Use step-level conditionals instead of job-level if-skips, which cause 'no jobs run' failure emails. A silently succeeding job is better than a skipped job that spams the maintainer.

## Statement

CI workflows should either do meaningful work or not trigger at all. Use step-level conditionals instead of job-level if-skips, which cause 'no jobs run' failure emails. A silently succeeding job is better than a skipped job that spams the maintainer.

## Evidence

- session-20260126-211444: pr-gate.yml used job-level if to skip maintainer, causing GitHub to email 'Run failed: no jobs were run' on every PR. Fixed by moving check to step-level if - job always runs, step skips silently. (weight: 0.95)

## Supports

<!-- Add beliefs this supports -->

## Attacks

<!-- Add beliefs this defeats -->

## Attacked-By

<!-- Add beliefs that challenge this -->

## Applied-In

<!-- Add concrete applications -->

## Revision Log

- 2026-01-27: Created (confidence: 0.90)
