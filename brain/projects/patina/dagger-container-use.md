# Dagger Container-Use Pattern

## Overview
Integration of Dagger's container-use pattern for AI agent workflows in Patina, enabling isolated development environments with git branch isolation and session tracking.

## Implementation Details

### Template Selection
The init command now detects agent workflow requirements:
```rust
let use_agent_template = design.get("project")
    .and_then(|p| p.get("features"))
    .and_then(|f| f.as_array())
    .map(|features| features.iter().any(|f| f.as_str() == Some("agent-workflows")))
    .unwrap_or(false);
```

### Agent Command Structure
New `patina agent` command with subcommands:
- `workspace`: Creates isolated development container
- `test`: Runs tests in separate environment
- `shell`: (Future) Interactive container access

### Session Integration
```rust
// Agent command automatically uses Patina session ID
let session_id = env::var("PATINA_SESSION_ID")
    .unwrap_or_else(|_| {
        if let Ok(session) = patina::session::Session::load(Path::new(".")) {
            session.id.to_string()
        } else {
            format!("agent-{}", chrono::Utc::now().timestamp())
        }
    });
```

### Dagger Pipeline Features

#### Workspace Isolation
- Separate container per agent task
- Git branch per session (`agent/{session-id}`)
- Isolated caches to prevent conflicts
- Full development environment

#### Enhanced Build Pipeline
- Linting with clippy
- Test isolation
- Graceful fallbacks
- Clear progress reporting

## Key Design Decisions

### 1. Template-Based Approach
- Agent features opt-in via PROJECT_DESIGN.toml
- Keeps simple projects simple
- Progressive enhancement

### 2. Session-First Design
- Agent work tied to Patina sessions
- Natural integration with knowledge capture
- Traceable agent activities

### 3. Git Branch Strategy
- Automatic branch creation
- Clear naming convention
- Easy review workflow

### 4. Cache Isolation
- Per-session cache volumes
- Prevents build conflicts
- Maintains reproducibility

## Usage Patterns

### Basic Agent Workflow
```bash
# Start session
/session-start "implement authentication"

# Create agent workspace
patina agent workspace

# Agent works in container...
# Human captures insights
/session-update

# Review changes
git diff agent/$(patina session-id)

# End session
/session-end
```

### Parallel Agent Development
```bash
# Multiple agents on different features
PATINA_SESSION_ID=auth patina agent workspace
PATINA_SESSION_ID=api patina agent workspace

# Each gets own branch and container
```

## Benefits Realized

1. **Safety**: Agents can't break main development
2. **Visibility**: All agent actions reviewable
3. **Integration**: Natural fit with Patina's workflow
4. **Flexibility**: Works with any AI agent

## Future Enhancements

1. **Container Persistence**: Keep containers running for reuse
2. **Agent Communication**: Shared volumes for collaboration
3. **Metrics Collection**: Track agent performance
4. **Auto-merge**: Confidence-based automatic integration

This implementation brings Dagger's container-use vision to life within Patina's context management framework.