---
type: belief
id: project-config-in-git
persona: architect
facets: [devops, configuration, ci]
confidence:
  score: 0.85
  signals:
    evidence: 0.90
    source_reliability: 0.85
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: high
status: active
extracted: 2026-01-17
revised: 2026-01-17
---

# project-config-in-git

Project configuration should be tracked in git, only machine-specific settings belong in gitignore.

## Statement

Project configuration should be tracked in git, only machine-specific settings belong in gitignore. Split config into project-level decisions (model selection, recipe settings) and machine-specific settings (OS, architecture, detected tools).

## Evidence

- [[session-20260116-221800]] CI failed because `.patina/config.toml` was gitignored → CI created default config with wrong embedding model (`all-minilm-l6-v2` instead of project's `e5-base-v2`) (weight: 0.92)
- [[commit-b5d318e7]] Fixed by re-tracking config.toml with note: "only `[environment]` section is machine-specific" (weight: 0.90)
- [[session-20260116-154950]] Same issue with `oxidize.yaml` being gitignored (weight: 0.85)

## Supports

- [[spec-first]] - Configuration choices are design decisions that belong in version control
- [[eventlog-is-truth]] - Git history shows why config choices were made

## Attacks

- [[gitignore-all-config]] (status: defeated, reason: "leads to CI/local divergence, hard-to-debug failures")
- [[env-files-pattern]] (status: scoped, reason: "only applies to secrets and credentials, not project config")

## Attacked-By

- [[secret-leakage-risk]] (status: active, confidence: 0.6, scope: "mitigated by splitting project config from secrets - only track non-sensitive settings")
- [[machine-specific-drift]] (status: defeated, confidence: 0.3, scope: "solved by config sections - `[environment]` is machine-specific, rest is project-level")

## Applied-In

- `.patina/config.toml` now tracked in git (project settings), only temp files gitignored
- `.patina/oxidize.yaml` tracked as project recipe, not machine-specific
- `.gitignore` updated with clear comments explaining what's machine-specific vs project-level

## Revision Log

- 2026-01-17: Created (confidence: 0.85)
