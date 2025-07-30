---
id: hook-session-architecture
version: 1
created_date: 2025-07-29
confidence: medium
oxidizer: nicabar
tags: []
promoted_from: projects/patina
---

# Hook-Based Session Architecture Decision

**⚠️ DEPRECATED**: This complex hook-based approach was explored but not implemented. The current session system uses simple slash commands (/session-start, /session-update, /session-end) without hooks or automation. See `layer/topics/development/session-implementation.md` for the current implementation.

## Context
Traditional session capture requires manual intervention and can miss important context. The goal is to create an automated system that captures everything while remaining lightweight during AI interactions.

## Decision
Implement a hook-based session capture system that:
1. Uses Claude Code's native hook system for automatic event capture
2. Maintains lightweight logs during sessions
3. Processes and enriches logs asynchronously
4. Integrates with existing Patina layer system

## Architecture Components

### 1. Hook Layer
- **Purpose**: Automatic event capture
- **Implementation**: Bash scripts triggered by Claude Code hooks
- **Data Format**: Simple pipe-delimited logs
- **Performance**: <50ms per hook execution

### 2. Processing Layer
- **Purpose**: Merge hooks + JSONL + enrichments
- **Implementation**: Rust module with feature flag
- **Timing**: Post-session or on-demand
- **Output**: Unified markdown sessions

### 3. Enrichment Layer
- **Purpose**: Add context and decisions
- **Implementation**: Sub-agent system
- **Trigger**: Periodic or manual
- **Integration**: Separate conversation context

### 4. Storage Layer
- **Purpose**: Persistent knowledge storage
- **Structure**: Hierarchical (core/topics/projects)
- **Format**: Markdown with frontmatter
- **Evolution**: Patterns migrate up hierarchy

## Technical Decisions

### Feature Flag Approach
```toml
[features]
default = []
hooks = []  # Experimental hook system
```

Rationale: Allows testing without affecting stable code

### Separate Configuration
```
.claude/settings.json       # Production
.claude/settings.hooks.json # Experimental
```

Rationale: Easy switching between modes

### Timestamp Ownership
- Scripts generate timestamps
- AI provides context
- No AI-generated timestamps

Rationale: Consistency and reliability

### Processing Pipeline
1. Raw capture (hooks)
2. Structured data (JSONL)
3. Enrichment (sub-agents)
4. Unification (Rust processor)
5. Storage (layer system)

## Benefits Realized

1. **Zero Manual Overhead**: Automatic capture
2. **Complete Coverage**: Nothing missed
3. **Low Token Usage**: Enrichment is async
4. **Backward Compatible**: Existing system intact
5. **Testable**: Each component isolated

## Challenges Addressed

1. **Timestamp Chaos**: Scripts own timestamps
2. **Token Efficiency**: Lightweight capture, rich processing
3. **Integration Complexity**: Feature flags isolate changes
4. **Data Correlation**: Unified timeline from multiple sources

## Migration Strategy

1. **Phase 1**: Implement hooks (current)
2. **Phase 2**: Parallel operation
3. **Phase 3**: Comparison and validation
4. **Phase 4**: Gradual transition
5. **Phase 5**: Deprecate old system

## Validation Approach

Using Dagger containers to:
- Test hook execution
- Verify data capture
- Validate processing
- Ensure no regression

## Future Enhancements

1. **Real-time Processing**: Stream processing of events
2. **Pattern Detection**: Automatic pattern extraction
3. **Cross-Session Analysis**: Identify trends
4. **Integration Points**: Git, CI/CD, monitoring

## Related Decisions
- Sub-Agent Architecture
- Container-First Development
- Session Persistence Design