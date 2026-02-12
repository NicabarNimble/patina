---
id: session-capture
status: verified
verification_date: 2026-02-12
oxidizer: nicabar
references: [core/session-principles.md, topics/sessions/capture-raw-distill-later.md]
tags: [sessions, capture, workflow]
---

# Session Capture

Patina captures development context through friction-free session tracking.


## The Pattern

Capture development context with minimal friction:

1. **Scripts handle mechanics** - Timestamps, git state, file tracking
2. **Markdown for humans** - Readable session files
3. **Progressive detail** - Start simple, enhance later
4. **Time-based organization** - Natural chronological flow

## Implementation

Sessions are YAML-frontmatter markdown files in `layer/sessions/`, managed by `patina session` (CLI) or `/session-*` (adapter slash commands):

```yaml
---
type: session
id: 20260212-161126              # Timestamp-based ID
title: "feature name"
status: active                   # active → archived
llm: claude                      # Which adapter
created: 2026-02-12T21:11:26Z
git:
  branch: patina
  starting_commit: e1a1736c...
  start_tag: session-20260212-161126-claude-start
---

## Previous Session Context
## Goals
## Activity Log
## Beliefs Captured
## Session Classification
```

Git tags bracket each session (`session-{id}-start` / `session-{id}-end`). On end, sessions are classified by work type (feature-work, pattern-work, fix-work, etc.) based on git metrics (files changed, commits, patterns modified).

## Consequences

- Natural documentation emerges
- No friction during development
- Context preserved for future
- Patterns ready for promotion