---
type: belief
id: commit-early-commit-often
persona: architect
facets: [git, workflow, development-process]
confidence:
  score: 0.90
  signals:
    evidence: 0.95
    source_reliability: 0.90
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: high
status: active
extracted: 2026-01-17
revised: 2026-01-17
---

# commit-early-commit-often

Make small, focused commits frequently rather than batching changes into large commits.

## Statement

Make small, focused commits frequently rather than batching changes into large commits. Each commit should represent one logical change with a single purpose. Use `git add -p` for surgical staging when files contain multiple changes.

## Evidence

- [[CLAUDE.md]] Git Discipline section: "Commit often, use scalpel not shotgun. One commit = one purpose" (weight: 0.95)
- [[session-20260116-221800]] Session ended with 8 commits, each focused on specific fix (weight: 0.85)
- [[session-20260117-072948]]: Git history shows ~150+ commits in 2 weeks with clear single-purpose messages (weight: 0.90)

## Supports

- [[eventlog-is-truth]] - Granular commits create better audit trail
- [[measure-first]] - Small commits make it easier to measure impact of changes

## Attacks

- [[batch-commits]] (status: defeated, reason: "makes bisecting failures harder, obscures reasoning")
- [[wip-commits]] (status: scoped, reason: "acceptable on feature branches, not main")

## Attacked-By

- [[context-switching-cost]] (status: active, confidence: 0.5, scope: "mitigated by session tracking - commits mark progress checkpoints")
- [[commit-noise]] (status: defeated, confidence: 0.3, scope: "detailed history valuable for learning, not noise")

## Applied-In

- Session workflow: `/session-update` monitors uncommitted changes, warns if old
- `.claude/bin/session-start.sh` tags commits at session boundaries
- Pre-push hooks enforce clean commits before CI

## Revision Log

- 2026-01-17: Created (confidence: 0.90)
