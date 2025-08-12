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