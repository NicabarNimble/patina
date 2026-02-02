---
type: belief
id: session-git-integration
persona: architect
facets: [git, session-tracking, workflow, architecture]
confidence:
  score: 0.87
  signals:
    evidence: 0.92
    source_reliability: 0.87
    recency: 0.80
    survival: 0.50
    user_endorsement: 0.50
entrenchment: high
status: active
extracted: 2026-01-17
revised: 2026-01-17
---

# session-git-integration

Session tracking should use git tags and commits as first-class events, not separate from version control.

## Statement

Session tracking should use git tags and commits as first-class events, not separate from version control. Git history becomes the source of truth for session timelines, work classification, and session boundaries.

## Evidence

- [[session-20260117-072948]]: Git history shows 10 session tags in 2 days: `session-TIMESTAMP-claude-start` and `session-TIMESTAMP-claude-end` (weight: 0.90)
- [[session-20260117-072948]]: [[CLAUDE.md]] Session-Git Commands section: "Integrated Git workflow into session tracking" (weight: 0.88)
- [[session-20260117-072948]]: [[.claude/bin/session-start.sh]] automatically creates git tags at session boundaries (weight: 0.92)
- [[session-20260117-072948]]: Session files include git metrics: commits, files changed, session tags (weight: 0.85)

## Supports

- [[eventlog-is-truth]] - Git commits are the append-only event log for sessions
- [[commit-early-commit-often]] - Sessions encourage commits as progress checkpoints
- [[measure-first]] - Git metrics provide measurable session classification

## Attacks

- [[separate-session-db]] (status: defeated, reason: "duplication, drift from actual work")
- [[session-metadata-only]] (status: defeated, reason: "loses connection to actual code changes")

## Attacked-By

- [[non-git-projects]] (status: active, confidence: 0.4, scope: "Patina requires git, acceptable tradeoff for git-aware features")
- [[rebase-breaks-tags]] (status: scoped, confidence: 0.5, scope: "work branches only, tags preserved on main")

## Applied-In

- `.claude/bin/session-start.sh` - Creates `session-TIMESTAMP-claude-start` tag
- `.claude/bin/session-end.sh` - Creates `session-TIMESTAMP-claude-end` tag
- Session files reference git ranges: `session-START..session-END`
- `/session-update` uses git diff to detect uncommitted changes
- Session classification uses git metrics: commits, files changed, patterns modified

## Verification

```verify type="sql" label="Session start tags exist" expect=">= 100"
SELECT COUNT(*) FROM git_tags WHERE tag_name LIKE 'session-%-start'
```

```verify type="sql" label="Session end tags exist" expect=">= 100"
SELECT COUNT(*) FROM git_tags WHERE tag_name LIKE 'session-%-end'
```

```verify type="sql" label="Commits linked to sessions" expect=">= 10"
SELECT COUNT(*) FROM commits WHERE sha IN (SELECT DISTINCT source_id FROM eventlog WHERE event_type = 'git.commit' AND data LIKE '%session_id%')
```

```verify type="sql" label="Grounding reaches session scripts" expect=">= 1"
SELECT COUNT(*) FROM belief_code_reach WHERE belief_id = 'session-git-integration' AND file_path LIKE '%session%'
```

## Revision Log

- 2026-01-17: Created (confidence: 0.87)
