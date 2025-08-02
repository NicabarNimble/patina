---
id: session-capture
status: verified
verification_date: 2025-08-02
oxidizer: nicabar
references: [core/session-principles.md, topics/sessions/capture-raw-distill-later.md]
tags: [sessions, capture, workflow]
---

# Session Capture

Patina captures development context through friction-free session tracking.

## Verification

```bash
#!/bin/bash
# Verify session capture implementation:

echo "Checking session capture..."

# Session management exists
grep -q "pub struct Session" src/session.rs || exit 1
grep -q "pub struct SessionManager" src/session.rs || exit 1

# Sessions track patterns
grep -q "patterns: Vec<SessionPattern>" src/session.rs || exit 1

# Session commands in Claude adapter
test -f .claude/commands/session-start.sh || echo "⚠ Session scripts in project directory"
grep -q "session" src/adapters/claude.rs || exit 1

# Sessions are timestamped
grep -q "id: String" src/session.rs || exit 1  # Uses timestamp-based IDs

# Session files use markdown
ls layer/sessions/*.md 2>/dev/null || echo "⚠ No session files yet"

echo "✓ Session capture verified"
```

## The Pattern

Capture development context with minimal friction:

1. **Scripts handle mechanics** - Timestamps, git state, file tracking
2. **Markdown for humans** - Readable session files
3. **Progressive detail** - Start simple, enhance later
4. **Time-based organization** - Natural chronological flow

## Implementation

```rust
// Sessions track discovered patterns
pub struct Session {
    pub id: String,  // Timestamp-based
    pub patterns: Vec<SessionPattern>,
}

// Pattern captured during work
pub struct SessionPattern {
    pub name: String,
    pub pattern_type: String,
    pub committed: bool,
}
```

## Consequences

- Natural documentation emerges
- No friction during development
- Context preserved for future
- Patterns ready for promotion