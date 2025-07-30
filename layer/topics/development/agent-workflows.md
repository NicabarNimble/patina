---
id: agent-workflows
version: 1
created_date: 2025-07-23
confidence: medium
oxidizer: nicabar
tags: []
---

# Agent Workflows with Dagger

## Overview
AI agents need isolated, reproducible environments to work safely and effectively. Patina integrates Dagger's container-use pattern to provide structured workspaces for agent development.

## Core Principles

### 1. Isolation First
Each agent task runs in its own:
- Container environment
- Git branch (`agent/{session-id}`)
- Cache namespace
- Working directory

### 2. Session Integration
Agent workflows are tied to Patina sessions:
- Session ID tracks agent work
- Context flows between human and agent
- Changes are reviewable before merging

### 3. Reproducible Environments
Every agent workspace is:
- Built from the same base image
- Includes all necessary tools
- Cached for performance
- Fully deterministic

## Implementation Pattern

### Project Setup
```toml
# PROJECT_DESIGN.toml
[project]
features = ["agent-workflows"]
```

### Pipeline Structure
```go
// Agent workspace creation
func runAgentWorkspace(ctx context.Context, client *dagger.Client) {
    container := client.Container().
        From("rust:1.75").
        WithDirectory("/workspace", hostDir).
        WithEnvVariable("PATINA_SESSION_ID", sessionID).
        WithExec([]string{"git", "checkout", "-b", "agent/" + sessionID})
}
```

### Usage Pattern
```bash
# Start agent workspace
patina agent workspace

# Run isolated tests
patina agent test

# With explicit session
PATINA_SESSION_ID=fix-bug-123 patina agent workspace
```

## Benefits

### For Development
1. **Safe Experimentation**: Agents can't affect main branch
2. **Parallel Work**: Multiple agents on different tasks
3. **Clear Review**: All changes on dedicated branches
4. **Session Context**: Links to Patina's knowledge system

### For AI Agents
1. **Clean Environment**: No interference from other work
2. **Full Tooling**: Everything needed pre-installed
3. **Git Integration**: Natural version control
4. **Deterministic**: Same result every time

## Integration with Patina Sessions

### Workflow
1. Start Patina session: `/session-start "implement feature X"`
2. Create agent workspace: `patina agent workspace`
3. Agent works in isolated container
4. Capture insights: `/session-update`
5. Review changes: `git diff agent/{session-id}`
6. End session with learnings: `/session-end`

### Knowledge Flow
```
Human Intent (session start)
    ↓
Agent Work (isolated container)
    ↓
Git Branch (reviewable changes)
    ↓
Session Notes (captured insights)
    ↓
Brain Patterns (distilled knowledge)
```

## Best Practices

### 1. One Task Per Agent
- Clear boundaries
- Easy review
- Focused context

### 2. Session-Driven Development
- Always start with session
- Capture insights during work
- End with distillation

### 3. Branch Hygiene
- Review before merging
- Clean up old branches
- Document decisions

### 4. Cache Management
- Session-specific caches
- Periodic cleanup
- Share when appropriate

## Future Evolution

### Planned Enhancements
1. **Interactive Shells**: Direct container access
2. **Agent Collaboration**: Multiple agents on same session
3. **MCP Integration**: Direct tool access in containers
4. **Automated Review**: AI-assisted change review

### Architecture Extensions
1. **Plugin System**: Custom agent environments
2. **Template Library**: Pre-configured workspaces
3. **Metrics Collection**: Agent performance tracking
4. **Knowledge Mining**: Automatic pattern extraction

This pattern enables safe, structured AI agent development while maintaining the human-in-the-loop principles core to Patina's philosophy.