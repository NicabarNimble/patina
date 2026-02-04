---
type: belief
id: ci-gates-not-ci-spam
persona: architect
facets: [ci, process, github-actions]
confidence:
  score: 0.90
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

## Verification

```verify type="sql" label="Grounding reaches CI scripts" expect=">= 1"
SELECT COUNT(*) FROM belief_code_reach WHERE belief_id = 'ci-gates-not-ci-spam' AND (file_path LIKE '%pre-push%' OR file_path LIKE '%ci%' OR file_path LIKE '%test.yml%')
```

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
