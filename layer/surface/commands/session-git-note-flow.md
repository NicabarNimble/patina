---
id: session-git-note-flow
status: active
created: 2025-08-17
tags: [commands, git-integration, session-management, llm-workflow, testing]
references: [session-git-start-flow.md, session-git-update-flow.md, session-git-end-flow.md]
---

# /session-git-note Command Flow (Testing Version)

## Purpose
Testing version of deeply integrated session+git note command. Once proven, will replace standard `/session-note`.

## Important: Testing Strategy
- This is `/session-git-note` for testing in fresh Claude instance
- Preserves all original session functionality
- Adds mandatory git context to every note
- Once validated, will become the new `/session-note`

## Simple Flow (Session + Git Memory)

### 1. Single Command Execution
```bash
/session-git-note "JWT is better than sessions for this architecture"
```

Automatically enriches with git context:
```bash
# Detect what files are currently being worked on
CURRENT_FILES=$(git diff --name-only HEAD)
RECENT_COMMITS=$(git log --oneline -3)
```

### 2. Smart Note Enhancement

The note gets augmented with context:

```markdown
## Insight: JWT is better than sessions for this architecture

**Context**: Working on auth/token.rs, auth/middleware.rs
**Recent commits**: 
  - abc123 feat: add JWT token generation
  - def456 refactor: remove session-based auth
  - ghi789 test: add token validation tests

**Pattern detected**: authentication-strategy
**Confidence boost**: +15% (insight validates approach)
```

### 3. Simple Note with Context

```bash
# Add note with git context
echo "## Note: $NOTE_TEXT" >> active-session.md
echo "Git SHA: $(git rev-parse HEAD)" >> active-session.md
echo "Branch: $(git branch --show-current)" >> active-session.md
echo "" >> active-session.md

# Suggest commit if important
echo "Consider committing with: git commit -am 'note: $NOTE_TEXT'"
```

### 4. Note Types (Simple)

```bash
# All notes are just text with git context
# No automatic pattern creation
# No complex triggers

/session-git-note "Discovered JWT works well"
# -> Adds to session file with git SHA

/session-git-note "Failed approach: websockets too complex"
# -> Adds to session file with git SHA

/session-git-note "Learning: Arc<Mutex<>> for sharing"
# -> Adds to session file with git SHA
```

### 5. Note Examples

```bash
# Simple note capture - no categories needed
/session-git-note "Database indexes reduced query time 10x"
/session-git-note "Found race condition in concurrent updates"
/session-git-note "async/await not needed for file IO"
/session-git-note "Scaling limit: 1000 concurrent users"
/session-git-note "Idea: Try Redis for session cache"

# Each note just gets added with git context
# Future enhancement: parse and categorize notes
```

## Implementation Notes

### Core Functionality
- Capture note text to session file
- Add git SHA for context
- Suggest commit if appropriate
- Keep it simple

### Future Enhancements
- Note categorization and analysis
- Pattern extraction from notes
- Linking notes to specific code locations

## Configuration Options

```toml
# .patina/config.toml
[session.note]
auto_enrich = true                    # Add git context to notes
trigger_commits = true                # Suggest commits for significant notes
create_patterns = true                # Auto-create patterns from insights
link_to_code = true                  # Link notes to file:line
categorize = true                    # Require note type
significance_threshold = "medium"     # When to trigger git actions
```

## Memory Preservation

### Where Notes Live

1. **During Session**: In active-session.md
2. **After Session**: Archived in layer/sessions/[id].md
3. **In Git**: Commit messages preserve key insights

### Searching Notes

```bash
# Notes are searchable through git
git log --grep="note:" --oneline
# Or through archived sessions
grep -r "JWT" layer/sessions/
```

## Example Flows

### Insight Note
```bash
$ /session-note insight "Middleware pattern perfect for auth"

💡 Insight captured!

Context added:
- Working on: auth/middleware.rs
- After commit: "refactor: extract auth to middleware"
- Pattern: middleware-pattern (confidence: +20%)

Suggestion: This validates your approach. Consider committing:
  git commit -am "validate: middleware pattern for auth (see note)"

Note saved to session and linked to current git context.
```

### Bug Note
```bash
$ /session-note bug "User can bypass auth by direct API call"

🐛 Critical bug noted!

Immediate actions:
1. ✅ Auto-committed current state for safety
2. ✅ Created branch: fix/auth-bypass-vulnerability
3. ✅ Added to layer/critical/security-issues.md

Please fix immediately or document workaround.
```

### Learning Note
```bash
$ /session-note learning "Rust lifetimes finally clicked - use references when possible"

📚 Learning captured!

This insight:
- Added to: layer/surface/learnings/rust-lifetimes.md
- Linked to: your current implementation in src/core/processor.rs
- Searchable by: future you or team members

Your growth is now part of Patina's memory!
```

## Success Metrics

- Notes include git context automatically
- Critical notes trigger immediate commits
- Insights evolve into patterns
- Failed approaches become anti-patterns
- All notes searchable across sessions

## The Philosophy

Notes are not just comments - they're **active memory creation**. Each note:
- Captures context (what code, what commit, what worked)
- Triggers actions (commits, patterns, warnings)
- Builds knowledge (searchable, reusable, evolving)
- Prevents repeated mistakes (anti-patterns, failed approaches)

The git integration makes notes **living documentation** tied to actual code evolution.